"""Configuration centralisée OllamaStudio.

Les chemins (workspace, data, documents) restent chargés depuis l'env/.env
car ils sont nécessaires avant l'init de la BDD.
Tous les autres paramètres sont désormais persistés dans la table app_settings
et lus dynamiquement via get_setting_value().
"""
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Paramètres de démarrage — uniquement les chemins et le CORS initial."""
    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    # Chemins (nécessaires avant init_db)
    workspace_root: Path = Path("workspace")
    data_dir: Path = Path("data")
    documents_dir: Path = Path("documents")

    # CORS initial (lu au démarrage, avant la BDD)
    cors_origins: list[str] = ["http://localhost:5173", "http://localhost:3000"]

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
            try:
                d.mkdir(parents=True, exist_ok=True)
            except PermissionError:
                pass  # Volume monté avec permissions restrictives


settings = Settings()
