"""Router Documents — import de dossiers, chunking et recherche."""
from __future__ import annotations

import logging
import mimetypes
from pathlib import Path
from typing import Annotated

import aiofiles
from fastapi import APIRouter, Depends, File, HTTPException, UploadFile
from pydantic import BaseModel
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.config import settings
from app.database import Document, DocumentChunk, get_db
from app.services.settings import get_setting_value
from app.services.document_processor import process_file
from app.services.debug import debug_buffer

logger = logging.getLogger(__name__)
router = APIRouter()

# Extensions texte supportées
TEXT_EXTENSIONS = {
    ".txt", ".md", ".rst", ".py", ".rs", ".go", ".js", ".ts", ".tsx", ".jsx",
    ".java", ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".sh", ".bash",
    ".yaml", ".yml", ".toml", ".json", ".xml", ".html", ".css", ".scss",
    ".sql", ".dockerfile", ".env", ".conf", ".cfg", ".ini",
}


@router.get("/")
async def list_documents(
    db: Annotated[AsyncSession, Depends(get_db)],
) -> list[dict]:
    result = await db.execute(select(Document).order_by(Document.imported_at.desc()))
    docs = result.scalars().all()
    return [
        {
            "id": d.id,
            "filename": d.filename,
            "relative_path": d.relative_path,
            "mime_type": d.mime_type,
            "size_bytes": d.size_bytes,
            "chunk_count": d.chunk_count,
            "imported_at": d.imported_at.isoformat(),
        }
        for d in docs
    ]


@router.post("/upload", status_code=201)
async def upload_documents(
    db: Annotated[AsyncSession, Depends(get_db)],
    files: list[UploadFile] = File(...),
) -> dict:
    """Upload et indexe un ou plusieurs fichiers."""
    debug_buffer.log("INFO", "import", f"Upload de {len(files)} fichier(s)")
    imported = []
    errors = []

    for file in files:
        if not file.filename:
            continue
        ext = Path(file.filename).suffix.lower()
        if ext not in TEXT_EXTENSIONS:
            msg = f"Extension non supportée: {ext}"
            debug_buffer.log("WARN", "import", f"Fichier {file.filename} rejeté: {msg}")
            errors.append({"file": file.filename, "error": msg})
            continue

        content_bytes = await file.read()
        max_file_size = await get_setting_value("max_file_size", 10 * 1024 * 1024)
        if len(content_bytes) > max_file_size:
            msg = f"Fichier trop volumineux ({len(content_bytes)} > {max_file_size})"
            debug_buffer.log("WARN", "import", f"Fichier {file.filename}: {msg}")
            errors.append({"file": file.filename, "error": msg})
            continue

        try:
            text = content_bytes.decode("utf-8", errors="replace")
        except Exception as e:
            debug_buffer.log("ERROR", "import", f"Décodage UTF-8 échoué pour {file.filename}: {e}")
            errors.append({"file": file.filename, "error": str(e)})
            continue

        # Sauvegarde physique
        dest = settings.documents_dir / file.filename
        dest.parent.mkdir(parents=True, exist_ok=True)
        debug_buffer.log("DEBUG", "import", f"Sauvegarde physique: {dest}")
        async with aiofiles.open(dest, "wb") as f:
            await f.write(content_bytes)

        # Chunking et persistance
        try:
            result = await _index_text(db, file.filename, file.filename, text, len(content_bytes))
            debug_buffer.log("INFO", "import", f"Indexé: {file.filename} → {result['chunks']} chunks")
            imported.append(result)
        except Exception as e:
            debug_buffer.log("ERROR", "import", f"Indexation échouée pour {file.filename}: {e}", error=str(e))
            errors.append({"file": file.filename, "error": f"Indexation: {e}"})

    debug_buffer.log("INFO", "import", f"Upload terminé: {len(imported)} importés, {len(errors)} erreurs")
    return {"imported": imported, "errors": errors}


class FolderImportRequest(BaseModel):
    folder_path: str    # Chemin absolu ou relatif sur le serveur


@router.post("/import-folder", status_code=201)
async def import_folder(
    body: FolderImportRequest,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    """Importe récursivement un dossier entier depuis le système de fichiers du serveur."""
    folder = Path(body.folder_path).resolve()
    debug_buffer.log("INFO", "import", f"Import dossier demandé: {body.folder_path} → résolu: {folder}")

    # Sécurité : le dossier doit être dans documents_dir, workspace_root ou data_dir
    allowed_roots = [
        settings.documents_dir.resolve(),
        settings.workspace_root.resolve(),
        settings.data_dir.resolve(),
    ]
    debug_buffer.log("DEBUG", "import", f"Racines autorisées: {[str(r) for r in allowed_roots]}")

    if not any(str(folder).startswith(str(root)) for root in allowed_roots):
        msg = f"Dossier {folder} hors des répertoires autorisés"
        debug_buffer.log("ERROR", "import", msg, allowed_roots=[str(r) for r in allowed_roots])
        raise HTTPException(
            status_code=403,
            detail=f"Dossier hors des répertoires autorisés. Chemins Docker valides : {', '.join(str(r) for r in allowed_roots)}"
        )

    if not folder.exists():
        debug_buffer.log("ERROR", "import", f"Dossier inexistant: {folder}")
        raise HTTPException(status_code=404, detail=f"Dossier inexistant: {folder}")
    if not folder.is_dir():
        debug_buffer.log("ERROR", "import", f"Pas un dossier: {folder}")
        raise HTTPException(status_code=404, detail=f"Ce n'est pas un dossier: {folder}")

    imported = []
    errors = []
    skipped = 0

    all_files = sorted(folder.rglob("*"))
    debug_buffer.log("DEBUG", "import", f"Trouvé {len(all_files)} entrées dans {folder}")

    for path in all_files:
        if not path.is_file():
            continue
        if path.suffix.lower() not in TEXT_EXTENSIONS:
            skipped += 1
            debug_buffer.log("DEBUG", "import", f"Ignoré (extension): {path.name} ({path.suffix})")
            continue
        max_file_size = await get_setting_value("max_file_size", 10 * 1024 * 1024)
        file_size = path.stat().st_size
        if file_size > max_file_size:
            msg = f"Trop volumineux ({file_size} > {max_file_size})"
            debug_buffer.log("WARN", "import", f"{path.name}: {msg}")
            errors.append({"file": str(path), "error": msg})
            continue

        try:
            async with aiofiles.open(path, encoding="utf-8", errors="replace") as f:
                text = await f.read()

            rel_path = str(path.relative_to(folder))
            result = await _index_text(db, path.name, rel_path, text, file_size)
            debug_buffer.log("DEBUG", "import", f"Indexé: {rel_path} → {result['chunks']} chunks")
            imported.append(result)

        except Exception as e:
            debug_buffer.log("ERROR", "import", f"Erreur fichier {path}: {e}", error=str(e), error_type=type(e).__name__)
            errors.append({"file": str(path), "error": str(e)})

    debug_buffer.log("INFO", "import", f"Import dossier terminé: {len(imported)} importés, {skipped} ignorés, {len(errors)} erreurs")
    return {
        "folder": str(folder),
        "imported": len(imported),
        "skipped": skipped,
        "errors": errors,
        "files": imported,
    }


@router.get("/search")
async def search_documents(
    q: str,
    max_results: int = 10,
    db: Annotated[AsyncSession, Depends(get_db)] = None,
) -> dict:
    """Recherche full-text simple dans les chunks."""
    from sqlalchemy import or_

    terms = [t.strip() for t in q.lower().split() if len(t.strip()) > 2]
    if not terms:
        return {"results": [], "query": q}

    conditions = [DocumentChunk.content.ilike(f"%{term}%") for term in terms[:5]]
    stmt = (
        select(DocumentChunk, Document)
        .join(Document, DocumentChunk.document_id == Document.id)
        .where(or_(*conditions))
        .limit(max_results)
    )
    result = await db.execute(stmt)
    rows = result.all()

    return {
        "query": q,
        "results": [
            {
                "document": row.Document.filename,
                "path": row.Document.relative_path,
                "chunk_index": row.DocumentChunk.chunk_index,
                "excerpt": row.DocumentChunk.content[:400],
            }
            for row in rows
        ],
        "count": len(rows),
    }


@router.delete("/{document_id}", status_code=204)
async def delete_document(
    document_id: int,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> None:
    await db.execute(delete(DocumentChunk).where(DocumentChunk.document_id == document_id))
    await db.execute(delete(Document).where(Document.id == document_id))
    await db.commit()


# ------------------------------------------------------------------
# Helper d'indexation
# ------------------------------------------------------------------

async def _index_text(
    db: AsyncSession,
    filename: str,
    relative_path: str,
    text: str,
    size_bytes: int,
) -> dict:
    """Découpe le texte en chunks et les persiste en base."""
    from app.services.document_processor import chunk_text

    mime = mimetypes.guess_type(filename)[0] or "text/plain"
    chunk_size = await get_setting_value("chunk_size", 1500)
    chunk_overlap = await get_setting_value("chunk_overlap", 150)
    chunks = chunk_text(text, chunk_size, chunk_overlap)

    doc = Document(
        filename=filename,
        relative_path=relative_path,
        mime_type=mime,
        size_bytes=size_bytes,
        chunk_count=len(chunks),
    )
    db.add(doc)
    await db.flush()  # Obtient doc.id

    for i, (chunk_text_content, token_count) in enumerate(chunks):
        db.add(DocumentChunk(
            document_id=doc.id,
            chunk_index=i,
            content=chunk_text_content,
            tokens=token_count,
        ))

    await db.commit()
    return {"id": doc.id, "filename": filename, "chunks": len(chunks)}
