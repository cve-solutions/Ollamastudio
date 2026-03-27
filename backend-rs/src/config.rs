use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub workspace_root: PathBuf,
    pub data_dir: PathBuf,
    pub documents_dir: PathBuf,
    pub version: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            workspace_root: PathBuf::from(env::var("WORKSPACE_ROOT").unwrap_or_else(|_| "workspace".into())),
            data_dir: PathBuf::from(env::var("DATA_DIR").unwrap_or_else(|_| "data".into())),
            documents_dir: PathBuf::from(env::var("DOCUMENTS_DIR").unwrap_or_else(|_| "documents".into())),
            version: env::var("APP_VERSION").unwrap_or_else(|_| "0.0.33".into()),
        }
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.data_dir.join("skills")
    }

    pub fn templates_dir(&self) -> PathBuf {
        self.data_dir.join("templates")
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("ollamastudio.db")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for dir in [
            &self.workspace_root,
            &self.data_dir,
            &self.documents_dir,
            &self.skills_dir(),
            &self.templates_dir(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}
