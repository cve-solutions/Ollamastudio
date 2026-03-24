"""Router Templates — snippets et templates de prompts en YAML."""
from __future__ import annotations

import logging
from pathlib import Path

import aiofiles
import yaml
from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from app.config import settings

logger = logging.getLogger(__name__)
router = APIRouter()

DEFAULT_TEMPLATES = [
    {
        "id": "explain-code",
        "name": "Expliquer ce code",
        "category": "Code",
        "icon": "📖",
        "content": "Explique ce code de manière détaillée, ligne par ligne si nécessaire :\n\n```\n{{code}}\n```",
        "variables": ["code"],
    },
    {
        "id": "refactor",
        "name": "Refactoriser",
        "category": "Code",
        "icon": "♻️",
        "content": "Refactorise ce code en respectant les bonnes pratiques (SOLID, DRY, lisibilité) :\n\n```\n{{code}}\n```\n\nContexte : {{context}}",
        "variables": ["code", "context"],
    },
    {
        "id": "write-tests",
        "name": "Écrire des tests",
        "category": "Test",
        "icon": "🧪",
        "content": "Écris des tests unitaires complets pour ce code. Couvre les cas nominaux et les cas limites :\n\n```\n{{code}}\n```\n\nFramework de test : {{framework}}",
        "variables": ["code", "framework"],
    },
    {
        "id": "docker-optimize",
        "name": "Optimiser Dockerfile",
        "category": "DevOps",
        "icon": "🐳",
        "content": "Optimise ce Dockerfile (multi-stage build, layer caching, sécurité, taille d'image) :\n\n```dockerfile\n{{dockerfile}}\n```",
        "variables": ["dockerfile"],
    },
    {
        "id": "security-review",
        "name": "Revue de sécurité",
        "category": "Sécurité",
        "icon": "🔐",
        "content": "Effectue une revue de sécurité de ce code/configuration. Identifie les vulnérabilités selon OWASP et les bonnes pratiques ANSSI :\n\n```\n{{content}}\n```",
        "variables": ["content"],
    },
    {
        "id": "commit-message",
        "name": "Message de commit",
        "category": "Git",
        "icon": "📝",
        "content": "Génère un message de commit conventionnel (Conventional Commits) pour ces changements :\n\n{{changes}}",
        "variables": ["changes"],
    },
    {
        "id": "api-doc",
        "name": "Documenter une API",
        "category": "Documentation",
        "icon": "📚",
        "content": "Génère la documentation OpenAPI/Swagger pour cet endpoint :\n\n```\n{{code}}\n```",
        "variables": ["code"],
    },
    {
        "id": "debug-error",
        "name": "Déboguer une erreur",
        "category": "Debug",
        "icon": "🐛",
        "content": "Analyse cette erreur et propose une solution :\n\nErreur :\n```\n{{error}}\n```\n\nCode concerné :\n```\n{{code}}\n```",
        "variables": ["error", "code"],
    },
]


def _template_path(template_id: str) -> Path:
    return settings.templates_dir / f"{template_id}.yaml"


async def seed_default_templates() -> None:
    """Écrit les templates par défaut."""
    for tpl in DEFAULT_TEMPLATES:
        path = _template_path(tpl["id"])
        if not path.exists():
            data = {k: v for k, v in tpl.items() if k != "id"}
            async with aiofiles.open(path, "w", encoding="utf-8") as f:
                await f.write(yaml.dump(data, allow_unicode=True, default_flow_style=False))
    logger.info("Templates initialisés dans %s", settings.templates_dir)


class TemplateCreate(BaseModel):
    id: str
    name: str
    category: str = "Général"
    icon: str = "📝"
    content: str
    variables: list[str] = []


@router.get("/")
async def list_templates() -> list[dict]:
    templates: dict[str, dict] = {}

    for tpl in DEFAULT_TEMPLATES:
        templates[tpl["id"]] = dict(tpl)

    for path in settings.templates_dir.glob("*.yaml"):
        tpl_id = path.stem
        async with aiofiles.open(path, encoding="utf-8") as f:
            content = await f.read()
        data = yaml.safe_load(content) or {}
        data["id"] = tpl_id
        templates[tpl_id] = data

    # Tri par catégorie puis nom
    return sorted(templates.values(), key=lambda t: (t.get("category", ""), t.get("name", "")))


@router.get("/{template_id}")
async def get_template(template_id: str) -> dict:
    path = _template_path(template_id)
    if path.exists():
        async with aiofiles.open(path, encoding="utf-8") as f:
            content = await f.read()
        data = yaml.safe_load(content) or {}
        data["id"] = template_id
        return data
    for tpl in DEFAULT_TEMPLATES:
        if tpl["id"] == template_id:
            return dict(tpl)
    raise HTTPException(status_code=404, detail="Template introuvable")


@router.post("/", status_code=201)
async def create_template(body: TemplateCreate) -> dict:
    path = _template_path(body.id)
    if path.exists():
        raise HTTPException(status_code=409, detail="Template existant")
    data = body.model_dump(exclude={"id"})
    async with aiofiles.open(path, "w", encoding="utf-8") as f:
        await f.write(yaml.dump(data, allow_unicode=True, default_flow_style=False))
    return {"id": body.id, **data}


@router.put("/{template_id}")
async def update_template(template_id: str, body: TemplateCreate) -> dict:
    path = _template_path(template_id)
    data = body.model_dump(exclude={"id"})
    async with aiofiles.open(path, "w", encoding="utf-8") as f:
        await f.write(yaml.dump(data, allow_unicode=True, default_flow_style=False))
    return {"id": template_id, **data}


@router.delete("/{template_id}", status_code=204)
async def delete_template(template_id: str) -> None:
    if any(t["id"] == template_id for t in DEFAULT_TEMPLATES):
        raise HTTPException(status_code=403, detail="Impossible de supprimer un template par défaut")
    path = _template_path(template_id)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Template introuvable")
    path.unlink()
