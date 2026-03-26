use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::state::AppState;

/// GET /api/config -- return the full application configuration object.
pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    // Read all relevant settings from DB in one query
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM app_settings")
            .fetch_all(&state.pool)
            .await?;

    // Build a lookup map
    let settings: std::collections::HashMap<String, String> =
        rows.into_iter().collect();

    let get = |key: &str, fallback: &str| -> String {
        settings
            .get(key)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };

    let get_int = |key: &str, fallback: i64| -> i64 {
        settings
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(fallback)
    };

    let get_bool = |key: &str, fallback: bool| -> bool {
        settings
            .get(key)
            .map(|v| v == "true")
            .unwrap_or(fallback)
    };

    Ok(Json(json!({
        "version": state.config.version,
        "workspace_root": state.config.workspace_root.to_string_lossy(),
        "data_dir": state.config.data_dir.to_string_lossy(),
        "documents_dir": state.config.documents_dir.to_string_lossy(),
        "ollama": {
            "base_url": get("ollama_base_url", "http://localhost:11434"),
            "api_mode": get("ollama_api_mode", "openai"),
            "default_model": get("ollama_default_model", "qwen3-coder"),
            "timeout": get_int("ollama_timeout", 300),
        },
        "shell": {
            "timeout": get_int("shell_timeout", 30),
        },
        "files": {
            "max_file_size": get_int("max_file_size", 10_485_760),
        },
        "documents": {
            "max_chunks_context": get_int("max_chunks_context", 8),
            "chunk_size": get_int("chunk_size", 1500),
            "chunk_overlap": get_int("chunk_overlap", 150),
        },
        "debug": {
            "enabled": get_bool("debug_mode", false),
        },
    })))
}
