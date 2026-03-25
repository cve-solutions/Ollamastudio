"""Router shell — terminal WebSocket pour xterm.js avec PTY."""
from __future__ import annotations

import asyncio
import logging
import os
import pty
import select
import signal
import struct
import termios
import tty
from typing import Annotated

from fastapi import APIRouter, WebSocket, WebSocketDisconnect
from pydantic import BaseModel

from app.config import settings
from app.services.debug import debug_buffer

logger = logging.getLogger(__name__)
router = APIRouter()

# Sécurité : commandes interdites (T1059 mitigation partielle)
BLOCKED_COMMANDS = frozenset(["su", "sudo", "passwd", "adduser", "useradd", "chsh"])


class ShellSession:
    """Gère un pseudo-terminal (PTY) associé à une connexion WebSocket."""

    def __init__(self, ws: WebSocket, cwd: str):
        self.ws = ws
        self.cwd = cwd
        self.pid: int | None = None
        self.master_fd: int | None = None

    async def start(self) -> None:
        """Lance le processus shell avec PTY."""
        debug_buffer.log("INFO", "ws", f"Démarrage session shell PTY dans {self.cwd}")
        env = {
            **os.environ,
            "TERM": "xterm-256color",
            "COLORTERM": "truecolor",
            "HOME": str(settings.workspace_root),
            "PS1": r"\[\033[01;32m\]\u@ollamastudio\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ ",
        }
        self.pid, self.master_fd = pty.fork()

        if self.pid == 0:
            # Processus enfant — exécute bash dans le workspace
            os.chdir(self.cwd)
            os.execvpe(
                "/bin/bash",
                ["/bin/bash", "--login"],
                env,
            )
        else:
            # Processus parent — lit/écrit sur le master fd
            await self._io_loop()

    async def _io_loop(self) -> None:
        """Boucle I/O entre PTY et WebSocket."""
        loop = asyncio.get_event_loop()
        fd = self.master_fd

        async def read_from_pty():
            """Lit depuis le PTY et envoie au WebSocket."""
            while True:
                await loop.run_in_executor(None, self._wait_readable, fd)
                try:
                    data = os.read(fd, 4096)
                except OSError:
                    break
                await self.ws.send_bytes(data)

        async def read_from_ws():
            """Lit depuis le WebSocket et écrit dans le PTY."""
            while True:
                try:
                    msg = await self.ws.receive()
                except WebSocketDisconnect:
                    logger.debug("[Shell WS] Client déconnecté")
                    break

                msg_type = msg.get("type", "unknown")
                logger.debug("[Shell WS] Message reçu: type=%s keys=%s", msg_type, list(msg.keys()))

                if msg_type == "websocket.disconnect":
                    logger.debug("[Shell WS] Disconnect frame reçu")
                    break

                if "bytes" in msg:
                    data = msg["bytes"]
                    logger.debug("[Shell WS] Bytes reçus: %d octets", len(data))
                elif "text" in msg:
                    data = msg["text"].encode()
                    logger.debug("[Shell WS] Texte reçu: %r", msg["text"][:100])
                else:
                    logger.debug("[Shell WS] Message ignoré (pas de bytes/text): %s", msg_type)
                    continue

                # Commandes de contrôle JSON (resize, signal)
                if data.startswith(b"{"):
                    await self._handle_control(data)
                    continue

                logger.debug("[Shell WS] Écriture PTY: %d octets", len(data))
                os.write(fd, data)

        await asyncio.gather(read_from_pty(), read_from_ws())

    def _wait_readable(self, fd: int) -> None:
        select.select([fd], [], [], 0.1)

    async def _handle_control(self, raw: bytes) -> None:
        """Gère les messages de contrôle JSON (resize terminal, signal)."""
        import json
        try:
            ctrl = json.loads(raw)
        except (json.JSONDecodeError, ValueError):
            return

        cmd = ctrl.get("type")
        if cmd == "resize" and self.master_fd is not None:
            rows = ctrl.get("rows", 24)
            cols = ctrl.get("cols", 80)
            size = struct.pack("HHHH", rows, cols, 0, 0)
            try:
                import fcntl
                fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, size)
                debug_buffer.log("DEBUG", "ws", f"Terminal resize {rows}x{cols}")
            except Exception as e:
                debug_buffer.log("WARN", "ws", f"Échec resize terminal: {e}", error=str(e))
        elif cmd == "signal" and self.pid is not None:
            sig_name = ctrl.get("signal", "SIGINT")
            sig = getattr(signal, sig_name, signal.SIGINT)
            try:
                os.kill(self.pid, sig)
                debug_buffer.log("DEBUG", "ws", f"Signal {sig_name} envoyé au PID {self.pid}")
            except ProcessLookupError:
                debug_buffer.log("WARN", "ws", f"Processus {self.pid} introuvable pour signal {sig_name}")

    def cleanup(self) -> None:
        """Tue le processus et ferme le fd."""
        debug_buffer.log("DEBUG", "ws", f"Nettoyage session shell PID={self.pid} FD={self.master_fd}")
        if self.pid:
            try:
                os.kill(self.pid, signal.SIGTERM)
                os.waitpid(self.pid, os.WNOHANG)
            except (ProcessLookupError, ChildProcessError) as e:
                debug_buffer.log("WARN", "ws", f"Cleanup PID {self.pid}: {e}")
        if self.master_fd:
            try:
                os.close(self.master_fd)
            except OSError as e:
                debug_buffer.log("WARN", "ws", f"Cleanup FD {self.master_fd}: {e}")


@router.websocket("/ws")
async def shell_websocket(ws: WebSocket) -> None:
    """WebSocket terminal — compatible xterm.js."""
    client = ws.client
    debug_buffer.log("INFO", "ws", f"Nouvelle connexion WebSocket shell depuis {client}")
    await ws.accept()
    cwd = str(settings.workspace_root)
    session = ShellSession(ws, cwd)
    try:
        await session.start()
    except WebSocketDisconnect:
        logger.info("Client shell déconnecté")
        debug_buffer.log("INFO", "ws", "Client shell déconnecté proprement")
    except Exception as e:
        logger.exception("Erreur session shell: %s", e)
        debug_buffer.log("ERROR", "ws", f"Erreur session shell: {e}", error=str(e), error_type=type(e).__name__)
        try:
            await ws.send_text(f"\r\n\033[31m[Erreur shell: {e}]\033[0m\r\n")
        except Exception as send_err:
            debug_buffer.log("WARN", "ws", f"Impossible d'envoyer l'erreur au client: {send_err}")
    finally:
        session.cleanup()
        try:
            await ws.close()
        except Exception as close_err:
            debug_buffer.log("WARN", "ws", f"Erreur fermeture WebSocket: {close_err}")
