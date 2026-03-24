"""OllamaStudio - Backend FastAPI principal."""
import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from app.config import settings
from app.database import init_db
from app.routers import chat, documents, files, mcp, models, sessions, settings as settings_router, shell, skills, templates
from app.services.settings import seed_default_settings, get_setting_value

logging.basicConfig(
    level=logging.INFO,
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
    logger.info("Backend prêt — workspace: %s", settings.workspace_root)
    yield
    logger.info("Arrêt OllamaStudio backend.")


app = FastAPI(
    title="OllamaStudio API",
    description="Interface web Claude Code compatible pour Ollama",
    version="0.1.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

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
    return JSONResponse({"status": "ok", "version": "0.1.0"})


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
    }
