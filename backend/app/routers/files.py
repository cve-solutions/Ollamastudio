"""Router fichiers — navigation workspace et opérations CRUD sécurisées."""
from __future__ import annotations

import mimetypes
from pathlib import Path
from typing import Annotated

import aiofiles
from fastapi import APIRouter, HTTPException, UploadFile
from fastapi.responses import FileResponse
from pydantic import BaseModel

from app.config import settings

router = APIRouter()


def _safe_path(relative: str) -> Path:
    """Empêche la traversée de répertoire (T1083 mitigation)."""
    root = settings.workspace_root.resolve()
    target = (root / relative).resolve()
    if not str(target).startswith(str(root)):
        raise HTTPException(status_code=403, detail="Accès refusé hors workspace")
    return target


def _entry_dict(path: Path) -> dict:
    stat = path.stat()
    return {
        "name": path.name,
        "path": str(path.relative_to(settings.workspace_root)),
        "type": "directory" if path.is_dir() else "file",
        "size": stat.st_size if path.is_file() else None,
        "modified": stat.st_mtime,
        "mime": mimetypes.guess_type(path.name)[0] if path.is_file() else None,
    }


@router.get("/tree")
async def get_tree(path: str = ".") -> dict:
    """Retourne l'arborescence d'un répertoire (un niveau)."""
    target = _safe_path(path)
    if not target.exists():
        raise HTTPException(status_code=404, detail="Chemin inexistant")
    if not target.is_dir():
        raise HTTPException(status_code=400, detail="Le chemin n'est pas un répertoire")

    entries = []
    for item in sorted(target.iterdir(), key=lambda p: (p.is_file(), p.name.lower())):
        entry = _entry_dict(item)
        if item.is_dir():
            # Pour les dossiers, indique s'ils ont des enfants
            entry["has_children"] = any(True for _ in item.iterdir())
        entries.append(entry)

    return {
        "path": path,
        "entries": entries,
        "workspace_root": str(settings.workspace_root),
    }


@router.get("/read")
async def read_file(path: str) -> dict:
    """Lit le contenu d'un fichier texte."""
    target = _safe_path(path)
    if not target.exists():
        raise HTTPException(status_code=404, detail="Fichier inexistant")
    if target.is_dir():
        raise HTTPException(status_code=400, detail="C'est un répertoire")
    if target.stat().st_size > settings.max_file_size:
        raise HTTPException(status_code=413, detail="Fichier trop volumineux")

    async with aiofiles.open(target, encoding="utf-8", errors="replace") as f:
        content = await f.read()

    return {
        "path": path,
        "content": content,
        "mime": mimetypes.guess_type(target.name)[0],
        "size": target.stat().st_size,
    }


class WriteRequest(BaseModel):
    path: str
    content: str


@router.post("/write", status_code=201)
async def write_file(body: WriteRequest) -> dict:
    """Écrit (ou crée) un fichier texte dans le workspace."""
    target = _safe_path(body.path)
    target.parent.mkdir(parents=True, exist_ok=True)
    async with aiofiles.open(target, "w", encoding="utf-8") as f:
        await f.write(body.content)
    return {"path": body.path, "size": len(body.content.encode())}


class DeleteRequest(BaseModel):
    path: str


@router.delete("/delete")
async def delete_path(body: DeleteRequest) -> dict:
    """Supprime un fichier ou répertoire vide."""
    target = _safe_path(body.path)
    if not target.exists():
        raise HTTPException(status_code=404, detail="Chemin inexistant")
    if target.is_dir():
        try:
            target.rmdir()
        except OSError:
            raise HTTPException(status_code=409, detail="Répertoire non vide")
    else:
        target.unlink()
    return {"deleted": body.path}


class RenameRequest(BaseModel):
    old_path: str
    new_path: str


@router.post("/rename")
async def rename_path(body: RenameRequest) -> dict:
    """Renomme ou déplace un fichier dans le workspace."""
    src = _safe_path(body.old_path)
    dst = _safe_path(body.new_path)
    if not src.exists():
        raise HTTPException(status_code=404, detail="Source inexistante")
    dst.parent.mkdir(parents=True, exist_ok=True)
    src.rename(dst)
    return {"old_path": body.old_path, "new_path": body.new_path}


@router.post("/mkdir", status_code=201)
async def mkdir(path: str) -> dict:
    """Crée un répertoire (et ses parents)."""
    target = _safe_path(path)
    target.mkdir(parents=True, exist_ok=True)
    return {"created": path}


@router.post("/upload", status_code=201)
async def upload_file(path: str, file: UploadFile) -> dict:
    """Upload un fichier dans le workspace."""
    target = _safe_path(f"{path}/{file.filename}" if path != "." else file.filename or "upload")
    target.parent.mkdir(parents=True, exist_ok=True)
    content = await file.read()
    if len(content) > settings.max_file_size:
        raise HTTPException(status_code=413, detail="Fichier trop volumineux")
    async with aiofiles.open(target, "wb") as f:
        await f.write(content)
    return {"path": str(target.relative_to(settings.workspace_root)), "size": len(content)}
