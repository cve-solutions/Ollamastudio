"""Router modèles — liste les modèles disponibles depuis Ollama."""
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.config import settings
from app.services.ollama import OllamaClient

router = APIRouter()


class ModelInfo(BaseModel):
    name: str
    size: int | None = None
    modified_at: str | None = None
    digest: str | None = None
    details: dict | None = None


@router.get("/", response_model=list[ModelInfo])
async def list_models(
    base_url: str | None = None,
) -> list[ModelInfo]:
    """Retourne la liste des modèles disponibles sur le serveur Ollama."""
    client = OllamaClient(base_url=base_url)
    try:
        models = await client.list_models()
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"Impossible de joindre Ollama: {e}")

    return [
        ModelInfo(
            name=m.get("name", ""),
            size=m.get("size"),
            modified_at=m.get("modified_at"),
            digest=m.get("digest"),
            details=m.get("details"),
        )
        for m in models
    ]


@router.get("/config")
async def get_ollama_config() -> dict:
    """Retourne la configuration Ollama active."""
    return {
        "base_url": settings.ollama_base_url,
        "api_mode": settings.ollama_api_mode,
        "default_model": settings.ollama_default_model,
        "timeout": settings.ollama_timeout,
    }
