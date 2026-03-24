"""Configuration centralisée OllamaStudio - chargée depuis variables d'environnement."""
from pathlib import Path
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    # Ollama
    ollama_base_url: str = "http://localhost:11434"
    ollama_api_mode: str = "openai"  # "openai" | "anthropic"
    ollama_default_model: str = "qwen3-coder"
    ollama_timeout: int = 300

    # Chemins
    workspace_root: Path = Path("workspace")
    data_dir: Path = Path("data")
    documents_dir: Path = Path("documents")

    # Sécurité
    cors_origins: list[str] = ["http://localhost:5173", "http://localhost:3000"]
    shell_timeout: int = 30
    shell_max_output: int = 65536       # 64 Ko max par commande
    max_file_size: int = 10 * 1024 * 1024  # 10 Mo max par fichier

    # Documents
    chunk_size: int = 1500              # Taille chunk en tokens approx
    chunk_overlap: int = 150
    max_chunks_context: int = 8         # Chunks max injectés dans le contexte

    # Sessions
    max_sessions: int = 100
    max_messages_per_session: int = 500

    # MCP
    mcp_timeout: int = 30

    @property
    def db_path(self) -> Path:
        return self.data_dir / "ollamastudio.db"

    @property
    def skills_dir(self) -> Path:
        return self.data_dir / "skills"

    @property
    def templates_dir(self) -> Path:
        return self.data_dir / "templates"

    def ensure_dirs(self) -> None:
        """Crée tous les répertoires nécessaires."""
        for d in [
            self.workspace_root,
            self.data_dir,
            self.documents_dir,
            self.skills_dir,
            self.templates_dir,
        ]:
            d.mkdir(parents=True, exist_ok=True)


settings = Settings()
