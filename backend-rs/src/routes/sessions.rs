use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::models::message::Message;
use crate::models::session::{Session, SessionCreate, SessionUpdate};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}
fn default_limit() -> i64 { 50 }

pub async fn list_sessions(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Session>>, AppError> {
    let rows = sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions ORDER BY updated_at DESC LIMIT ? OFFSET ?"
    )
    .bind(params.limit)
    .bind(params.offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(body): Json<SessionCreate>,
) -> Result<(StatusCode, Json<Session>), AppError> {
    let title = body.title.unwrap_or_else(|| "Nouvelle session".into());
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO sessions (title, model, skill_id) VALUES (?, ?, ?) RETURNING id"
    )
    .bind(&title)
    .bind(&body.model)
    .bind(&body.skill_id)
    .fetch_one(&state.pool)
    .await?;

    let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok((StatusCode::CREATED, Json(session)))
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Session>, AppError> {
    let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Session introuvable".into()))?;
    Ok(Json(session))
}

pub async fn update_session(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SessionUpdate>,
) -> Result<Json<Session>, AppError> {
    if let Some(title) = &body.title {
        sqlx::query("UPDATE sessions SET title = ? WHERE id = ?")
            .bind(title).bind(id).execute(&state.pool).await?;
    }
    if let Some(skill_id) = &body.skill_id {
        sqlx::query("UPDATE sessions SET skill_id = ? WHERE id = ?")
            .bind(skill_id).bind(id).execute(&state.pool).await?;
    }
    let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(session))
}

pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM messages WHERE session_id = ?")
        .bind(id).execute(&state.pool).await?;
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Message>>, AppError> {
    let rows = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC"
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
