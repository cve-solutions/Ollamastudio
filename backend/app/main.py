"""OllamaStudio - Backend FastAPI principal."""
import logging
import time
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from app.config import settings
from app.database import init_db
from app.routers import chat, documents, files, mcp, models, sessions, settings as settings_router, shell, skills, templates
from app.services.settings import seed_default_settings, get_setting_value
from app.services.debug import debug_buffer

import os
from pathlib import Path

# Version — lue depuis le fichier VERSION copié dans l'image Docker
_version_candidates = [
    Path(__file__).resolve().parent.parent / "VERSION",       # /app/VERSION (Docker)
    Path(__file__).resolve().parent.parent.parent / "VERSION", # racine projet (dev local)
]
__version__ = "0.0.1"
for _vf in _version_candidates:
    if _vf.exists():
        __version__ = _vf.read_text().strip()
        break

_log_level = getattr(logging, os.environ.get("LOG_LEVEL", "INFO").upper(), logging.INFO)
logging.basicConfig(
    level=_log_level,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Initialisation et nettoyage de l'application."""
    logger.info("Démarrage OllamaStudio backend...")
    settings.ensure_dirs()
    await init_db()
    await seed_default_settings()
    await skills.seed_default_skills()
    await templates.seed_default_templates()

    # Active le mode debug si env LOG_LEVEL=DEBUG ou si activé en BDD
    debug_enabled = os.environ.get("LOG_LEVEL", "").upper() == "DEBUG" or await get_setting_value("debug_mode", False)
    debug_buffer.enabled = bool(debug_enabled)
    if debug_enabled:
        logging.getLogger().setLevel(logging.DEBUG)
        logger.info("Mode DEBUG actif — toutes les requêtes HTTP et WS seront tracées")

    logger.info("Backend prêt — workspace: %s", settings.workspace_root)
    yield
    logger.info("Arrêt OllamaStudio backend.")


app = FastAPI(
    title="OllamaStudio API",
    description="Interface web Claude Code compatible pour Ollama",
    version=__version__,
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.middleware("http")
async def debug_trace_middleware(request: Request, call_next):
    """Trace toutes les requêtes HTTP quand le mode debug est actif."""
    if not debug_buffer.enabled:
        return await call_next(request)

    start = time.time()
    method = request.method
    path = request.url.path
    query = str(request.url.query) if request.url.query else ""

    debug_buffer.log("DEBUG", "http", f"{method} {path}", query=query)

    try:
        response = await call_next(request)
        elapsed = round((time.time() - start) * 1000, 1)
        level = "ERROR" if response.status_code >= 500 else (
            "WARN" if response.status_code >= 400 else "DEBUG"
        )
        debug_buffer.log(
            level, "http",
            f"{method} {path} → {response.status_code} ({elapsed}ms)",
            status=response.status_code, elapsed_ms=elapsed,
        )
        return response
    except Exception as exc:
        elapsed = round((time.time() - start) * 1000, 1)
        debug_buffer.log(
            "ERROR", "http",
            f"{method} {path} → EXCEPTION ({elapsed}ms): {exc}",
            error=str(exc), elapsed_ms=elapsed,
        )
        raise


# Routeurs
app.include_router(chat.router,              prefix="/api/chat",      tags=["Chat"])
app.include_router(models.router,            prefix="/api/models",    tags=["Modèles"])
app.include_router(sessions.router,          prefix="/api/sessions",  tags=["Sessions"])
app.include_router(files.router,             prefix="/api/files",     tags=["Fichiers"])
app.include_router(shell.router,             prefix="/api/shell",     tags=["Shell"])
app.include_router(templates.router,         prefix="/api/templates", tags=["Templates"])
app.include_router(skills.router,            prefix="/api/skills",    tags=["Skills"])
app.include_router(mcp.router,               prefix="/api/mcp",       tags=["MCP"])
app.include_router(documents.router,         prefix="/api/documents", tags=["Documents"])
app.include_router(settings_router.router,   prefix="/api/settings",  tags=["Paramètres"])


@app.get("/health", include_in_schema=False)
async def health() -> JSONResponse:
    return JSONResponse({"status": "ok", "version": __version__})


@app.get("/api/config", tags=["Configuration"])
async def get_config() -> dict:
    """Retourne la configuration courante depuis la BDD."""
    return {
        "ollama_base_url": await get_setting_value("ollama_base_url", "http://localhost:11434"),
        "ollama_api_mode": await get_setting_value("ollama_api_mode", "openai"),
        "ollama_default_model": await get_setting_value("ollama_default_model", "qwen3-coder"),
        "workspace_root": str(settings.workspace_root),
        "shell_timeout": await get_setting_value("shell_timeout", 30),
        "max_chunks_context": await get_setting_value("max_chunks_context", 8),
        "debug_mode": await get_setting_value("debug_mode", False),
    }


# ── Debug API ────────────────────────────────────────────────────────────────


@app.get("/api/debug/logs", tags=["Debug"])
async def get_debug_logs(
    limit: int = 100,
    category: str | None = None,
    level: str | None = None,
    since: float | None = None,
) -> dict:
    """Retourne les logs debug récents."""
    return {
        "enabled": debug_buffer.enabled,
        "entries": debug_buffer.get_entries(limit, category, level, since),
        "total": len(debug_buffer._buffer),
    }


@app.post("/api/debug/toggle", tags=["Debug"])
async def toggle_debug(enabled: bool | None = None) -> dict:
    """Active/désactive le mode debug dynamiquement."""
    from app.services.settings import get_setting_value
    if enabled is None:
        enabled = not debug_buffer.enabled
    debug_buffer.enabled = enabled
    # Ajuste le niveau de log global
    if enabled:
        logging.getLogger().setLevel(logging.DEBUG)
    else:
        logging.getLogger().setLevel(logging.INFO)
    return {"debug_mode": debug_buffer.enabled}


@app.delete("/api/debug/logs", tags=["Debug"])
async def clear_debug_logs() -> dict:
    """Vide le buffer de logs debug."""
    debug_buffer.clear()
    return {"cleared": True}
