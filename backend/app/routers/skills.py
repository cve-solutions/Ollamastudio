"""Router Skills — personas YAML avec system prompt, outils et paramètres."""
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

# ------------------------------------------------------------------
# Schéma d'une skill
# ------------------------------------------------------------------

DEFAULT_SKILLS = [
    {
        "id": "default",
        "name": "Assistant général",
        "description": "Assistant polyvalent avec accès à tous les outils",
        "icon": "🤖",
        "system_prompt": (
            "Tu es OllamaStudio, un assistant développeur expert. "
            "Tu as accès à des outils pour lire/écrire des fichiers, exécuter des commandes shell, "
            "faire des recherches dans le code et les documents importés. "
            "Réponds toujours en français sauf si l'utilisateur utilise une autre langue. "
            "Sois précis, concis et propose du code de qualité production."
        ),
        "enabled_tools": None,  # None = tous les outils
        "temperature": 0.7,
        "max_tokens": 4096,
        "color": "#6366f1",
    },
    {
        "id": "code-review",
        "name": "Code Review",
        "description": "Expert en revue de code, sécurité et bonnes pratiques",
        "icon": "🔍",
        "system_prompt": (
            "Tu es un expert en revue de code senior. Analyse le code fourni selon ces critères : "
            "1) Sécurité (OWASP, injections, secrets exposés) "
            "2) Performance et complexité algorithmique "
            "3) Maintenabilité et respect des principes SOLID "
            "4) Couverture de tests "
            "Fournis des commentaires structurés avec exemples de correction."
        ),
        "enabled_tools": ["read_file", "grep_files", "list_files"],
        "temperature": 0.3,
        "max_tokens": 8192,
        "color": "#f59e0b",
    },
    {
        "id": "devops",
        "name": "DevOps & Infrastructure",
        "description": "Expert Docker, CI/CD, Linux, configuration serveur",
        "icon": "🏗️",
        "system_prompt": (
            "Tu es un ingénieur DevOps senior spécialisé sur Linux (Debian/Fedora), Docker, "
            "et les pipelines CI/CD. Tu aides à configurer des environnements, rédiger des Dockerfiles "
            "optimisés et des scripts d'automatisation. Respecte les bonnes pratiques de sécurité "
            "système (ANSSI, CIS Benchmarks) quand applicable."
        ),
        "enabled_tools": ["read_file", "write_file", "run_command", "list_files", "grep_files", "git_status"],
        "temperature": 0.5,
        "max_tokens": 4096,
        "color": "#10b981",
    },
    {
        "id": "rust-expert",
        "name": "Expert Rust",
        "description": "Développeur Rust avancé — ownership, lifetimes, async, performances",
        "icon": "🦀",
        "system_prompt": (
            "Tu es un expert Rust de niveau avancé. Tu maîtrises le borrow checker, les lifetimes, "
            "les traits, async/await avec Tokio, les FFI et les optimisations de performances. "
            "Tu proposes du code idiomatique Rust en suivant les conventions de la communauté. "
            "Tu expliques les concepts complexes avec des exemples concrets."
        ),
        "enabled_tools": ["read_file", "write_file", "run_command", "grep_files", "git_status"],
        "temperature": 0.4,
        "max_tokens": 8192,
        "color": "#ef4444",
    },
    {
        "id": "security-audit",
        "name": "Audit Sécurité",
        "description": "Analyse de sécurité, conformité ANSSI/NIS2, hardening",
        "icon": "🛡️",
        "system_prompt": (
            "Tu es un auditeur sécurité expert (ANSSI, NIS2, ISO 27001). "
            "Tu analyses les configurations, les Dockerfiles, les scripts et le code source "
            "pour identifier les vulnérabilités. Tu fournis des recommandations de hardening "
            "conformes aux référentiels ANSSI et aux benchmarks CIS. "
            "Structure tes rapports : Risque critique / Élevé / Moyen / Faible."
        ),
        "enabled_tools": ["read_file", "grep_files", "list_files", "search_documents"],
        "temperature": 0.2,
        "max_tokens": 8192,
        "color": "#8b5cf6",
    },
]


# ------------------------------------------------------------------
# Helpers I/O
# ------------------------------------------------------------------

def _skill_path(skill_id: str) -> Path:
    return settings.skills_dir / f"{skill_id}.yaml"


async def load_skill(skill_id: str) -> dict | None:
    """Charge une skill depuis le fichier YAML ou les defaults."""
    # Cherche dans les fichiers custom d'abord
    path = _skill_path(skill_id)
    if path.exists():
        async with aiofiles.open(path, encoding="utf-8") as f:
            content = await f.read()
        data = yaml.safe_load(content)
        data["id"] = skill_id
        return data

    # Cherche dans les defaults embarqués
    for s in DEFAULT_SKILLS:
        if s["id"] == skill_id:
            return dict(s)

    return None


async def seed_default_skills() -> None:
    """Écrit les skills par défaut dans le dossier data/skills/ si inexistants."""
    for skill in DEFAULT_SKILLS:
        path = _skill_path(skill["id"])
        if not path.exists():
            skill_data = {k: v for k, v in skill.items() if k != "id"}
            async with aiofiles.open(path, "w", encoding="utf-8") as f:
                await f.write(yaml.dump(skill_data, allow_unicode=True, default_flow_style=False))
    logger.info("Skills initialisées dans %s", settings.skills_dir)


# ------------------------------------------------------------------
# Endpoints
# ------------------------------------------------------------------

class SkillCreate(BaseModel):
    id: str
    name: str
    description: str = ""
    icon: str = "🤖"
    system_prompt: str
    enabled_tools: list[str] | None = None
    temperature: float = 0.7
    max_tokens: int = 4096
    color: str = "#6366f1"


@router.get("/")
async def list_skills() -> list[dict]:
    """Retourne toutes les skills (defaults + custom)."""
    skills: dict[str, dict] = {}

    # Defaults
    for s in DEFAULT_SKILLS:
        skills[s["id"]] = dict(s)

    # Custom (écrasent les defaults si même ID)
    for path in settings.skills_dir.glob("*.yaml"):
        skill_id = path.stem
        async with aiofiles.open(path, encoding="utf-8") as f:
            content = await f.read()
        data = yaml.safe_load(content) or {}
        data["id"] = skill_id
        skills[skill_id] = data

    return list(skills.values())


@router.get("/{skill_id}")
async def get_skill(skill_id: str) -> dict:
    skill = await load_skill(skill_id)
    if skill is None:
        raise HTTPException(status_code=404, detail="Skill introuvable")
    return skill


@router.post("/", status_code=201)
async def create_skill(body: SkillCreate) -> dict:
    path = _skill_path(body.id)
    if path.exists():
        raise HTTPException(status_code=409, detail="Skill existante — utilisez PUT pour modifier")
    data = body.model_dump(exclude={"id"})
    async with aiofiles.open(path, "w", encoding="utf-8") as f:
        await f.write(yaml.dump(data, allow_unicode=True, default_flow_style=False))
    return {"id": body.id, **data}


@router.put("/{skill_id}")
async def update_skill(skill_id: str, body: SkillCreate) -> dict:
    path = _skill_path(skill_id)
    data = body.model_dump(exclude={"id"})
    async with aiofiles.open(path, "w", encoding="utf-8") as f:
        await f.write(yaml.dump(data, allow_unicode=True, default_flow_style=False))
    return {"id": skill_id, **data}


@router.delete("/{skill_id}", status_code=204)
async def delete_skill(skill_id: str) -> None:
    # Interdit la suppression des skills par défaut
    if any(s["id"] == skill_id for s in DEFAULT_SKILLS):
        raise HTTPException(status_code=403, detail="Impossible de supprimer une skill par défaut")
    path = _skill_path(skill_id)
    if not path.exists():
        raise HTTPException(status_code=404, detail="Skill introuvable")
    path.unlink()
