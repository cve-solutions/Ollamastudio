use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::models::setting::{AppSetting, BulkSettingUpdate, SettingUpdate};
use crate::state::AppState;

pub async fn list_settings(State(state): State<AppState>) -> Result<Json<Vec<AppSetting>>, AppError> {
    let rows = sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings ORDER BY category, key")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn get_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings WHERE key = ?")
        .bind(&key)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Setting '{key}' introuvable")))?;
    Ok(Json(json!({ "key": row.key, "value": parse_setting_value(&row) })))
}

pub async fn update_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<SettingUpdate>,
) -> Result<Json<AppSetting>, AppError> {
    let value_str = serialize_value(&body.value);
    sqlx::query("UPDATE app_settings SET value = ? WHERE key = ?")
        .bind(&value_str)
        .bind(&key)
        .execute(&state.pool)
        .await?;
    let row = sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings WHERE key = ?")
        .bind(&key)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(row))
}

pub async fn bulk_update(
    State(state): State<AppState>,
    Json(body): Json<BulkSettingUpdate>,
) -> Result<Json<Vec<AppSetting>>, AppError> {
    for (key, value) in &body.settings {
        let value_str = serialize_value(value);
        sqlx::query("UPDATE app_settings SET value = ? WHERE key = ?")
            .bind(&value_str)
            .bind(key)
            .execute(&state.pool)
            .await?;
    }
    let rows = sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings ORDER BY category, key")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows))
}

pub async fn get_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<Vec<AppSetting>>, AppError> {
    let rows = sqlx::query_as::<_, AppSetting>(
        "SELECT * FROM app_settings WHERE category = ? ORDER BY key",
    )
    .bind(&category)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(serde::Deserialize)]
pub struct TestConnectionBody {
    pub base_url: String,
}

pub async fn test_connection(
    Json(body): Json<TestConnectionBody>,
) -> Result<Json<Value>, AppError> {
    let url = format!("{}/api/tags", body.base_url.trim_end_matches('/'));
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let data: Value = resp.json().await.unwrap_or(json!({}));
            let models = data.get("models").and_then(|m| m.as_array());
            let count = models.map(|m| m.len()).unwrap_or(0);
            let names: Vec<String> = models
                .map(|m| m.iter().filter_map(|v| v.get("name")?.as_str().map(String::from)).collect())
                .unwrap_or_default();
            Ok(Json(json!({ "connected": true, "model_count": count, "models": names })))
        }
        Ok(resp) => Ok(Json(json!({ "connected": false, "error": format!("HTTP {}", resp.status()) }))),
        Err(e) => Ok(Json(json!({ "connected": false, "error": e.to_string() }))),
    }
}

// ── Helpers ──

fn serialize_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

fn parse_setting_value(s: &AppSetting) -> Value {
    match s.value_type.as_str() {
        "bool" | "boolean" => Value::Bool(s.value == "true"),
        "int" | "integer" => s.value.parse::<i64>().map(Value::from).unwrap_or(Value::String(s.value.clone())),
        "float" | "number" => s.value.parse::<f64>().map(|f| json!(f)).unwrap_or(Value::String(s.value.clone())),
        _ => Value::String(s.value.clone()),
    }
}

/// Seed default settings (called at startup)
pub async fn seed_default_settings(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    let defaults = vec![
        ("ollama_base_url", "http://localhost:11434", "ollama", "URL Ollama", "Adresse du serveur Ollama", "string"),
        ("ollama_api_mode", "openai", "ollama", "Mode API", "openai ou anthropic", "string"),
        ("ollama_default_model", "qwen3-coder", "ollama", "Modèle par défaut", "Nom du modèle Ollama", "string"),
        ("ollama_timeout", "120", "ollama", "Timeout (s)", "Timeout des requêtes Ollama", "int"),
        ("shell_timeout", "30", "shell", "Timeout commandes", "Timeout max par commande shell (s)", "int"),
        ("max_file_size", "10485760", "files", "Taille max fichier", "Taille max upload en octets", "int"),
        ("max_chunks_context", "8", "documents", "Chunks max contexte", "Nombre max de chunks par requête RAG", "int"),
        ("chunk_size", "1500", "documents", "Taille chunk", "Nombre de caractères par chunk", "int"),
        ("chunk_overlap", "150", "documents", "Chevauchement", "Chevauchement entre chunks", "int"),
        ("debug_mode", "false", "system", "Mode debug", "Active le logging détaillé", "bool"),
    ];

    for (key, value, cat, label, desc, vtype) in defaults {
        sqlx::query(
            "INSERT OR IGNORE INTO app_settings (key, value, category, label, description, value_type) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(key).bind(value).bind(cat).bind(label).bind(desc).bind(vtype)
        .execute(pool)
        .await?;
    }
    tracing::info!("Default settings seeded");
    Ok(())
}
