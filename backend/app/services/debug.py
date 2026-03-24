"""Service de debug — ring buffer de logs accessibles via API."""
from __future__ import annotations

import logging
import time
from collections import deque
from dataclasses import dataclass, field, asdict
from typing import Any

logger = logging.getLogger(__name__)

MAX_ENTRIES = 500


@dataclass
class DebugEntry:
    timestamp: float
    level: str  # INFO, WARN, ERROR, DEBUG
    category: str  # http, ws, import, skill, ollama, db, error
    message: str
    details: dict[str, Any] = field(default_factory=dict)


class DebugBuffer:
    """Ring buffer thread-safe de logs debug."""

    def __init__(self, maxlen: int = MAX_ENTRIES):
        self._buffer: deque[DebugEntry] = deque(maxlen=maxlen)
        self._enabled = False

    @property
    def enabled(self) -> bool:
        return self._enabled

    @enabled.setter
    def enabled(self, value: bool) -> None:
        self._enabled = value
        if value:
            self.log("INFO", "debug", "Mode debug activé")
        else:
            self.log("INFO", "debug", "Mode debug désactivé")

    def log(self, level: str, category: str, message: str, **details: Any) -> None:
        """Ajoute une entrée au buffer (toujours, même si disabled, pour les erreurs)."""
        if not self._enabled and level not in ("ERROR", "WARN"):
            return
        entry = DebugEntry(
            timestamp=time.time(),
            level=level,
            category=category,
            message=message,
            details=details,
        )
        self._buffer.append(entry)
        # Log aussi via le logger standard
        log_msg = f"[DEBUG:{category}] {message}"
        if details:
            log_msg += f" | {details}"
        if level == "ERROR":
            logger.error(log_msg)
        elif level == "WARN":
            logger.warning(log_msg)
        else:
            logger.debug(log_msg)

    def get_entries(
        self,
        limit: int = 100,
        category: str | None = None,
        level: str | None = None,
        since: float | None = None,
    ) -> list[dict]:
        """Retourne les entrées filtrées."""
        entries = list(self._buffer)
        if since:
            entries = [e for e in entries if e.timestamp >= since]
        if category:
            entries = [e for e in entries if e.category == category]
        if level:
            entries = [e for e in entries if e.level == level]
        return [asdict(e) for e in entries[-limit:]]

    def clear(self) -> None:
        self._buffer.clear()


# Singleton global
debug_buffer = DebugBuffer()
