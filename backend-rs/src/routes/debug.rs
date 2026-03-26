use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::state::AppState;

// ── Query types ──

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub level: Option<String>,
    pub since: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleQuery {
    pub enabled: Option<bool>,
}

// ── Handlers ──

/// GET /logs -- read debug log entries from the in-memory buffer.
pub async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<Value>, AppError> {
    let limit = query.limit.unwrap_or(100);
    let entries = state.debug.get_entries(
        limit,
        query.category.as_deref(),
        query.level.as_deref(),
        query.since,
    );
    let total = state.debug.total();

    Ok(Json(json!({
        "entries": entries,
        "total": total,
        "enabled": state.debug.is_enabled(),
    })))
}

/// POST /toggle -- enable or disable debug mode.
pub async fn toggle_debug(
    State(state): State<AppState>,
    Query(query): Query<ToggleQuery>,
) -> Result<Json<Value>, AppError> {
    let new_state = query.enabled.unwrap_or(!state.debug.is_enabled());
    state.debug.set_enabled(new_state);

    // Also persist in DB so it survives restart
    let _ = sqlx::query("UPDATE app_settings SET value = ? WHERE key = 'debug_mode'")
        .bind(if new_state { "true" } else { "false" })
        .execute(&state.pool)
        .await;

    tracing::info!("Debug mode toggled to {new_state}");

    Ok(Json(json!({
        "enabled": new_state,
    })))
}

/// DELETE /logs -- clear the debug log buffer.
pub async fn clear_logs(
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    state.debug.clear();
    Ok(StatusCode::NO_CONTENT)
}
