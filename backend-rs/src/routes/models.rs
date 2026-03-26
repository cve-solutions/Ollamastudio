use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::state::AppState;

// ── Types ──

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ListModelsQuery {
    pub base_url: Option<String>,
}

// ── Helpers ──

/// Read a single setting value from the DB, returning the fallback if not found.
async fn get_setting_value(
    pool: &sqlx::SqlitePool,
    key: &str,
    fallback: &str,
) -> Result<String, AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0).unwrap_or_else(|| fallback.to_string()))
}

// ── Handlers ──

/// GET / -- list all Ollama models (proxies to /api/tags).
pub async fn list_models(
    State(state): State<AppState>,
    Query(query): Query<ListModelsQuery>,
) -> Result<Json<Vec<ModelInfo>>, AppError> {
    let base = match query.base_url {
        Some(ref u) => u.trim_end_matches('/').to_string(),
        None => get_setting_value(&state.pool, "ollama_base_url", "http://localhost:11434")
            .await?
            .trim_end_matches('/')
            .to_string(),
    };

    let url = format!("{base}/api/tags");

    let resp = reqwest::get(&url)
        .await
        .map_err(|e| AppError::BadGateway(format!("Impossible de joindre Ollama: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::BadGateway(format!(
            "Ollama responded with HTTP {}",
            resp.status()
        )));
    }

    let data: Value = resp
        .json()
        .await
        .map_err(|e| AppError::BadGateway(format!("Invalid JSON from Ollama: {e}")))?;

    let models = data
        .get("models")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let result: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| ModelInfo {
            name: m
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            size: m.get("size").and_then(|v| v.as_i64()),
            modified_at: m
                .get("modified_at")
                .and_then(|v| v.as_str())
                .map(String::from),
            digest: m
                .get("digest")
                .and_then(|v| v.as_str())
                .map(String::from),
            details: m.get("details").cloned(),
        })
        .collect();

    Ok(Json(result))
}

/// GET /config -- return the active Ollama configuration from DB.
pub async fn get_ollama_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let base_url =
        get_setting_value(&state.pool, "ollama_base_url", "http://localhost:11434").await?;
    let api_mode = get_setting_value(&state.pool, "ollama_api_mode", "openai").await?;
    let default_model =
        get_setting_value(&state.pool, "ollama_default_model", "qwen3-coder").await?;
    let timeout_str = get_setting_value(&state.pool, "ollama_timeout", "300").await?;
    let timeout: i64 = timeout_str.parse().unwrap_or(300);

    Ok(Json(json!({
        "base_url": base_url,
        "api_mode": api_mode,
        "default_model": default_model,
        "timeout": timeout,
    })))
}
