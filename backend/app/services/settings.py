"""Service de gestion des paramètres applicatifs en base de données."""
from __future__ import annotations

import json
import logging
from typing import Any

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.database import AppSetting, AsyncSessionFactory

logger = logging.getLogger(__name__)

# Valeurs par défaut — seedées au premier lancement
DEFAULT_SETTINGS: list[dict[str, str]] = [
    # ── Ollama ────────────────────────────────────────────────────────
    {
        "key": "ollama_base_url",
        "value": "http://localhost:11434",
        "category": "ollama",
        "label": "URL du serveur Ollama",
        "description": "Adresse complète du serveur Ollama (ex: http://192.168.1.100:11434)",
        "value_type": "string",
    },
    {
        "key": "ollama_api_mode",
        "value": "openai",
        "category": "ollama",
        "label": "Mode API",
        "description": "openai (/v1/chat/completions) ou anthropic (/v1/messages)",
        "value_type": "string",
    },
    {
        "key": "ollama_default_model",
        "value": "qwen3-coder",
        "category": "ollama",
        "label": "Modèle par défaut",
        "description": "Modèle Ollama utilisé par défaut pour les nouvelles sessions",
        "value_type": "string",
    },
    {
        "key": "ollama_timeout",
        "value": "300",
        "category": "ollama",
        "label": "Timeout Ollama (secondes)",
        "description": "Temps maximum d'attente pour les requêtes Ollama",
        "value_type": "int",
    },
    # ── Sécurité ──────────────────────────────────────────────────────
    {
        "key": "cors_origins",
        "value": '["http://localhost:5173","http://localhost:3000"]',
        "category": "security",
        "label": "Origines CORS autorisées",
        "description": "Liste JSON des origines autorisées pour les requêtes cross-origin",
        "value_type": "json",
    },
    {
        "key": "shell_timeout",
        "value": "30",
        "category": "security",
        "label": "Timeout shell (secondes)",
        "description": "Durée maximale d'exécution d'une commande shell",
        "value_type": "int",
    },
    {
        "key": "shell_max_output",
        "value": "65536",
        "category": "security",
        "label": "Taille max sortie shell (octets)",
        "description": "Taille maximale de la sortie d'une commande shell (64 Ko par défaut)",
        "value_type": "int",
    },
    {
        "key": "max_file_size",
        "value": "10485760",
        "category": "security",
        "label": "Taille max fichier (octets)",
        "description": "Taille maximale d'un fichier lu ou écrit (10 Mo par défaut)",
        "value_type": "int",
    },
    # ── Documents / RAG ───────────────────────────────────────────────
    {
        "key": "chunk_size",
        "value": "1500",
        "category": "documents",
        "label": "Taille des chunks (tokens)",
        "description": "Taille approximative en tokens de chaque chunk de document",
        "value_type": "int",
    },
    {
        "key": "chunk_overlap",
        "value": "150",
        "category": "documents",
        "label": "Chevauchement des chunks",
        "description": "Nombre de tokens de chevauchement entre les chunks",
        "value_type": "int",
    },
    {
        "key": "max_chunks_context",
        "value": "8",
        "category": "documents",
        "label": "Chunks max en contexte",
        "description": "Nombre maximum de chunks injectés dans le contexte de chat",
        "value_type": "int",
    },
    # ── Sessions ──────────────────────────────────────────────────────
    {
        "key": "max_sessions",
        "value": "100",
        "category": "sessions",
        "label": "Nombre max de sessions",
        "description": "Nombre maximum de sessions de chat conservées",
        "value_type": "int",
    },
    {
        "key": "max_messages_per_session",
        "value": "500",
        "category": "sessions",
        "label": "Messages max par session",
        "description": "Nombre maximum de messages conservés par session",
        "value_type": "int",
    },
    # ── MCP ───────────────────────────────────────────────────────────
    {
        "key": "mcp_timeout",
        "value": "30",
        "category": "mcp",
        "label": "Timeout MCP (secondes)",
        "description": "Temps maximum d'attente pour les appels MCP",
        "value_type": "int",
    },
    # ── Debug ──────────────────────────────────────────────────────────
    {
        "key": "debug_mode",
        "value": "false",
        "category": "debug",
        "label": "Mode debug",
        "description": "Active le traçage détaillé de toutes les actions (requêtes, WebSocket, imports, erreurs)",
        "value_type": "bool",
    },
]


def _cast_value(raw: str, value_type: str) -> Any:
    """Convertit une valeur string en son type réel."""
    if value_type == "int":
        return int(raw)
    if value_type == "bool":
        return raw.lower() in ("true", "1", "yes")
    if value_type == "json":
        return json.loads(raw)
    return raw


def _serialize_value(value: Any, value_type: str) -> str:
    """Sérialise une valeur en string pour stockage."""
    if value_type == "json":
        return json.dumps(value, ensure_ascii=False)
    if value_type == "bool":
        return "true" if value else "false"
    return str(value)


async def seed_default_settings() -> None:
    """Insère les paramètres par défaut s'ils n'existent pas encore."""
    async with AsyncSessionFactory() as db:
        for dflt in DEFAULT_SETTINGS:
            existing = await db.get(AppSetting, dflt["key"])
            if existing is None:
                db.add(AppSetting(**dflt))
        await db.commit()
    logger.info("Paramètres par défaut vérifiés/seedés.")


async def get_all_settings(db: AsyncSession) -> list[dict[str, Any]]:
    """Retourne tous les paramètres groupés."""
    result = await db.execute(select(AppSetting).order_by(AppSetting.category, AppSetting.key))
    rows = result.scalars().all()
    return [
        {
            "key": r.key,
            "value": _cast_value(r.value, r.value_type),
            "category": r.category,
            "label": r.label,
            "description": r.description,
            "value_type": r.value_type,
        }
        for r in rows
    ]


async def get_settings_by_category(db: AsyncSession, category: str) -> list[dict[str, Any]]:
    """Retourne les paramètres d'une catégorie."""
    result = await db.execute(
        select(AppSetting).where(AppSetting.category == category).order_by(AppSetting.key)
    )
    rows = result.scalars().all()
    return [
        {
            "key": r.key,
            "value": _cast_value(r.value, r.value_type),
            "category": r.category,
            "label": r.label,
            "description": r.description,
            "value_type": r.value_type,
        }
        for r in rows
    ]


async def get_setting(db: AsyncSession, key: str) -> Any:
    """Retourne la valeur typée d'un paramètre, ou None."""
    row = await db.get(AppSetting, key)
    if row is None:
        return None
    return _cast_value(row.value, row.value_type)


async def update_setting(db: AsyncSession, key: str, value: Any) -> dict[str, Any] | None:
    """Met à jour un paramètre existant. Retourne le paramètre mis à jour."""
    row = await db.get(AppSetting, key)
    if row is None:
        return None
    row.value = _serialize_value(value, row.value_type)
    await db.flush()
    return {
        "key": row.key,
        "value": _cast_value(row.value, row.value_type),
        "category": row.category,
        "label": row.label,
        "description": row.description,
        "value_type": row.value_type,
    }


async def bulk_update_settings(db: AsyncSession, updates: dict[str, Any]) -> list[dict[str, Any]]:
    """Met à jour plusieurs paramètres en une seule transaction."""
    updated = []
    for key, value in updates.items():
        row = await db.get(AppSetting, key)
        if row is not None:
            row.value = _serialize_value(value, row.value_type)
            await db.flush()
            updated.append({
                "key": row.key,
                "value": _cast_value(row.value, row.value_type),
                "category": row.category,
                "label": row.label,
                "description": row.description,
                "value_type": row.value_type,
            })
    return updated


async def get_setting_value(key: str, default: Any = None) -> Any:
    """Helper autonome — ouvre sa propre session DB pour lire un paramètre."""
    async with AsyncSessionFactory() as db:
        row = await db.get(AppSetting, key)
        if row is None:
            return default
        return _cast_value(row.value, row.value_type)
