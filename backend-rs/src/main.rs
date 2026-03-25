mod config;
mod db;
mod error;
mod models;
mod routes;
mod services;
mod state;
mod agents;

use std::env;
use std::net::SocketAddr;

use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::services::debug::DebugBuffer;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Logging
    let filter = EnvFilter::try_from_env("LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    // Config
    let config = Config::from_env();
    if let Err(e) = config.ensure_dirs() {
        tracing::warn!("Impossible de créer les dossiers: {e}");
    }

    tracing::info!("Démarrage OllamaStudio backend v{}", config.version);

    // Database
    let pool = db::init_pool(&config.db_path())
        .await
        .expect("Impossible d'initialiser la base de données");

    // Seed default settings
    routes::settings::seed_default_settings(&pool)
        .await
        .expect("Impossible de seeder les settings");

    // Debug buffer
    let debug = DebugBuffer::new();
    let log_level = env::var("LOG_LEVEL").unwrap_or_default();
    if log_level.eq_ignore_ascii_case("debug") {
        debug.set_enabled(true);
        tracing::info!("Mode DEBUG actif");
    }

    // State
    let state = AppState {
        pool,
        config: config.clone(),
        debug,
    };

    // CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Router
    let app = routes::api_router()
        .layer(cors)
        .with_state(state);

    // Start server
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8000".into())
        .parse()
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Backend prêt — http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
