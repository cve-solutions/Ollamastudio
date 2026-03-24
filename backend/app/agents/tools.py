"""Définition et exécution des outils disponibles pour l'agent OllamaStudio."""
from __future__ import annotations

import asyncio
import logging
import os
import subprocess
from pathlib import Path
from typing import Any

import aiofiles

from app.config import settings

logger = logging.getLogger(__name__)

# ------------------------------------------------------------------
# Schémas des outils (format OpenAI function calling)
# ------------------------------------------------------------------

TOOL_SCHEMAS: list[dict] = [
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Lit le contenu d'un fichier dans le workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Chemin relatif au workspace"},
                },
                "required": ["path"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write_file",
            "description": "Écrit (ou crée) un fichier dans le workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Chemin relatif au workspace"},
                    "content": {"type": "string", "description": "Contenu à écrire"},
                },
                "required": ["path", "content"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "list_files",
            "description": "Liste les fichiers et répertoires d'un chemin du workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Chemin relatif, '.' pour la racine", "default": "."},
                    "recursive": {"type": "boolean", "default": False},
                },
                "required": [],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "run_command",
            "description": "Exécute une commande shell dans le workspace. Timeout configurable.",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Commande shell à exécuter"},
                    "timeout": {"type": "integer", "default": 30, "description": "Timeout en secondes"},
                },
                "required": ["command"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "grep_files",
            "description": "Recherche un pattern regex dans les fichiers du workspace.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Pattern regex"},
                    "path": {"type": "string", "default": ".", "description": "Répertoire de recherche"},
                    "file_pattern": {"type": "string", "default": "*", "description": "Filtre glob (ex: *.py)"},
                    "max_results": {"type": "integer", "default": 50},
                },
                "required": ["pattern"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "git_status",
            "description": "Retourne le statut git du workspace.",
            "parameters": {"type": "object", "properties": {}, "required": []},
        },
    },
    {
        "type": "function",
        "function": {
            "name": "search_documents",
            "description": "Recherche dans les documents importés par mots-clés.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Termes de recherche"},
                    "max_results": {"type": "integer", "default": 5},
                },
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "patch_file",
            "description": "Applique un patch unified diff à un fichier existant.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Chemin relatif du fichier"},
                    "old_content": {"type": "string", "description": "Contenu exact à remplacer"},
                    "new_content": {"type": "string", "description": "Nouveau contenu"},
                },
                "required": ["path", "old_content", "new_content"],
            },
        },
    },
]


# ------------------------------------------------------------------
# Sécurité — validation des chemins
# ------------------------------------------------------------------

def _safe_path(relative: str) -> Path:
    """Résout un chemin relatif et vérifie qu'il reste dans le workspace.

    Raises:
        PermissionError: Si le chemin tente de sortir du workspace (T1083 mitigation).
    """
    root = settings.workspace_root.resolve()
    target = (root / relative).resolve()
    if not str(target).startswith(str(root)):
        raise PermissionError(f"Accès refusé hors workspace: {relative}")
    return target


# ------------------------------------------------------------------
# Implémentation des outils
# ------------------------------------------------------------------

async def tool_read_file(path: str) -> dict:
    target = _safe_path(path)
    if not target.exists():
        return {"error": f"Fichier inexistant: {path}"}
    if target.stat().st_size > settings.max_file_size:
        return {"error": f"Fichier trop volumineux (max {settings.max_file_size // 1024} Ko)"}
    try:
        async with aiofiles.open(target, encoding="utf-8", errors="replace") as f:
            content = await f.read()
        return {"path": path, "content": content, "lines": content.count("\n") + 1}
    except OSError as e:
        return {"error": str(e)}


async def tool_write_file(path: str, content: str) -> dict:
    target = _safe_path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    try:
        async with aiofiles.open(target, "w", encoding="utf-8") as f:
            await f.write(content)
        return {"path": path, "written_bytes": len(content.encode())}
    except OSError as e:
        return {"error": str(e)}


async def tool_patch_file(path: str, old_content: str, new_content: str) -> dict:
    result = await tool_read_file(path)
    if "error" in result:
        return result
    current = result["content"]
    if old_content not in current:
        return {"error": "Contenu 'old_content' introuvable dans le fichier — patch annulé."}
    patched = current.replace(old_content, new_content, 1)
    return await tool_write_file(path, patched)


async def tool_list_files(path: str = ".", recursive: bool = False) -> dict:
    target = _safe_path(path)
    if not target.exists():
        return {"error": f"Chemin inexistant: {path}"}
    entries = []
    try:
        if recursive:
            for item in sorted(target.rglob("*")):
                rel = item.relative_to(settings.workspace_root)
                entries.append({
                    "path": str(rel),
                    "type": "directory" if item.is_dir() else "file",
                    "size": item.stat().st_size if item.is_file() else None,
                })
        else:
            for item in sorted(target.iterdir()):
                rel = item.relative_to(settings.workspace_root)
                entries.append({
                    "path": str(rel),
                    "name": item.name,
                    "type": "directory" if item.is_dir() else "file",
                    "size": item.stat().st_size if item.is_file() else None,
                })
    except OSError as e:
        return {"error": str(e)}
    return {"path": path, "entries": entries, "count": len(entries)}


async def tool_run_command(command: str, timeout: int = 30) -> dict:
    """Exécute une commande dans le workspace — limité au répertoire workspace.

    Sécurité T1059 : timeout strict, pas de shell d'accès root.
    """
    timeout = min(timeout, settings.shell_timeout)
    try:
        proc = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=str(settings.workspace_root),
            env={**os.environ, "HOME": str(settings.workspace_root)},
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
        except asyncio.TimeoutError:
            proc.kill()
            return {"error": f"Timeout ({timeout}s) dépassé", "command": command}

        out = stdout.decode("utf-8", errors="replace")[: settings.shell_max_output]
        err = stderr.decode("utf-8", errors="replace")[: settings.shell_max_output]
        return {
            "command": command,
            "stdout": out,
            "stderr": err,
            "returncode": proc.returncode,
        }
    except Exception as e:
        return {"error": str(e), "command": command}


async def tool_grep_files(
    pattern: str,
    path: str = ".",
    file_pattern: str = "*",
    max_results: int = 50,
) -> dict:
    target = _safe_path(path)
    try:
        result = subprocess.run(
            ["grep", "-rn", "--include", file_pattern, "-E", pattern, str(target)],
            capture_output=True,
            text=True,
            timeout=15,
            cwd=str(settings.workspace_root),
        )
        lines = result.stdout.splitlines()[:max_results]
        # Rend les chemins relatifs
        root_str = str(settings.workspace_root) + "/"
        lines = [l.replace(root_str, "") for l in lines]
        return {"pattern": pattern, "matches": lines, "count": len(lines)}
    except subprocess.TimeoutExpired:
        return {"error": "Timeout grep"}
    except Exception as e:
        return {"error": str(e)}


async def tool_git_status() -> dict:
    try:
        result = subprocess.run(
            ["git", "status", "--short", "--branch"],
            capture_output=True,
            text=True,
            timeout=10,
            cwd=str(settings.workspace_root),
        )
        if result.returncode != 0:
            return {"error": "Pas un dépôt git ou git non disponible", "stderr": result.stderr}
        return {"output": result.stdout}
    except Exception as e:
        return {"error": str(e)}


async def tool_search_documents(query: str, max_results: int = 5) -> dict:
    """Recherche simple par mots-clés dans les chunks de documents importés."""
    from sqlalchemy import or_, select, text

    from app.database import AsyncSessionFactory, DocumentChunk

    terms = query.lower().split()
    if not terms:
        return {"results": []}

    async with AsyncSessionFactory() as db:
        # Recherche full-text basique SQLite
        conditions = [
            DocumentChunk.content.ilike(f"%{term}%") for term in terms[:5]
        ]
        stmt = (
            select(DocumentChunk)
            .where(or_(*conditions))
            .limit(max_results)
        )
        result = await db.execute(stmt)
        chunks = result.scalars().all()

    return {
        "query": query,
        "results": [
            {"chunk_id": c.id, "document_id": c.document_id, "excerpt": c.content[:300]}
            for c in chunks
        ],
        "count": len(chunks),
    }


# ------------------------------------------------------------------
# Dispatcher centralisé
# ------------------------------------------------------------------

TOOL_EXECUTORS: dict[str, Any] = {
    "read_file": tool_read_file,
    "write_file": tool_write_file,
    "patch_file": tool_patch_file,
    "list_files": tool_list_files,
    "run_command": tool_run_command,
    "grep_files": tool_grep_files,
    "git_status": tool_git_status,
    "search_documents": tool_search_documents,
}


async def execute_tool(name: str, arguments: dict) -> dict:
    """Exécute un outil par son nom. Retourne toujours un dict."""
    executor = TOOL_EXECUTORS.get(name)
    if executor is None:
        return {"error": f"Outil inconnu: {name}"}
    try:
        result = await executor(**arguments)
        return result if isinstance(result, dict) else {"result": result}
    except PermissionError as e:
        return {"error": f"Permission refusée: {e}"}
    except Exception as e:
        logger.exception("Erreur outil %s: %s", name, e)
        return {"error": f"Erreur interne: {e}"}
