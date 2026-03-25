use crate::config::Config;
use crate::services::debug::DebugBuffer;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub debug: DebugBuffer,
}
