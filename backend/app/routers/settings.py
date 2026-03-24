"""Router paramètres — CRUD des paramètres applicatifs persistés en BDD."""
from __future__ import annotations

from typing import Annotated, Any

import httpx
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from sqlalchemy.ext.asyncio import AsyncSession

from app.database import get_db
from app.services.settings import (
    bulk_update_settings,
    get_all_settings,
    get_setting,
    get_settings_by_category,
    update_setting,
)

router = APIRouter()


class SettingUpdate(BaseModel):
    value: Any


class BulkSettingUpdate(BaseModel):
    settings: dict[str, Any]


class ConnectionTestRequest(BaseModel):
    base_url: str


# ── Liste complète et mise à jour en masse ────────────────────────────────────
# (déclarées AVANT les routes avec {key} pour éviter les conflits de routage)

@router.get("/")
async def list_settings(db: Annotated[AsyncSession, Depends(get_db)]) -> list[dict]:
    """Retourne tous les paramètres groupés par catégorie."""
    return await get_all_settings(db)


@router.put("/")
async def bulk_update(
    body: BulkSettingUpdate,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> list[dict]:
    """Met à jour plusieurs paramètres en une fois."""
    return await bulk_update_settings(db, body.settings)


# ── Test de connexion Ollama ─────────────────────────────────────────────────

@router.post("/ollama/test-connection")
async def test_ollama_connection(body: ConnectionTestRequest) -> dict:
    """Teste la connexion à un serveur Ollama et retourne les modèles disponibles."""
    base_url = body.base_url.rstrip("/")
    try:
        async with httpx.AsyncClient(timeout=10) as client:
            resp = await client.get(f"{base_url}/api/tags")
            resp.raise_for_status()
            data = resp.json()
            models = data.get("models", [])
            return {
                "connected": True,
                "model_count": len(models),
                "models": [
                    {
                        "name": m.get("name", ""),
                        "size": m.get("size"),
                        "modified_at": m.get("modified_at"),
                        "details": m.get("details"),
                    }
                    for m in models
                ],
            }
    except httpx.ConnectError:
        raise HTTPException(
            status_code=503,
            detail=f"Impossible de se connecter à {base_url} — serveur inaccessible",
        )
    except httpx.HTTPStatusError as e:
        raise HTTPException(
            status_code=503,
            detail=f"Serveur Ollama a répondu avec une erreur: {e.response.status_code}",
        )
    except Exception as e:
        raise HTTPException(
            status_code=503,
            detail=f"Erreur de connexion: {e}",
        )


# ── Par catégorie ─────────────────────────────────────────────────────────────

@router.get("/category/{category}")
async def list_settings_by_category(
    category: str,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> list[dict]:
    """Retourne les paramètres d'une catégorie donnée."""
    return await get_settings_by_category(db, category)


# ── Lecture / écriture unitaire ───────────────────────────────────────────────

@router.get("/{key}")
async def read_setting(
    key: str,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    """Retourne un paramètre par sa clé."""
    val = await get_setting(db, key)
    if val is None:
        raise HTTPException(status_code=404, detail=f"Paramètre '{key}' introuvable")
    return {"key": key, "value": val}


@router.put("/{key}")
async def write_setting(
    key: str,
    body: SettingUpdate,
    db: Annotated[AsyncSession, Depends(get_db)],
) -> dict:
    """Met à jour un paramètre existant."""
    result = await update_setting(db, key, body.value)
    if result is None:
        raise HTTPException(status_code=404, detail=f"Paramètre '{key}' introuvable")
    return result
