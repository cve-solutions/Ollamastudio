"""Configuration centralisée OllamaStudio.

Les chemins sont lus depuis les variables d'environnement (docker-compose).
Tous les autres paramètres (URL Ollama, CORS, timeouts…) sont persistés
dans la table app_settings et gérés via l'IHM.
"""
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Paramètres de démarrage — uniquement les chemins."""
    model_config = SettingsConfigDict(env_file_encoding="utf-8", extra="ignore")

    # Chemins (nécessaires avant init_db, passés par docker-compose)
    workspace_root: Path = Path("workspace")
    data_dir: Path = Path("data")
    documents_dir: Path = Path("documents")

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
