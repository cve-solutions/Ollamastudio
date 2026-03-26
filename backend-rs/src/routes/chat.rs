//! Chat SSE endpoint — persists messages and streams the agentic loop.

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use std::convert::Infallible;

use crate::agents::r#loop::AgentEvent;
use crate::agents::agentic_loop;
use crate::error::AppError;
use crate::models::message::ChatRequest;
use crate::services::ollama::OllamaClient;
use crate::state::AppState;

/// POST /api/chat/stream
///
/// Accepts a ChatRequest, persists the user message, runs the agentic loop
/// as an SSE stream, and persists the assistant reply when complete.
pub async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, AppError> {
    // --- Validate session exists ---
    let session = sqlx::query_as::<_, crate::models::session::Session>(
        "SELECT * FROM sessions WHERE id = ?",
    )
    .bind(req.session_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Session not found".into()))?;

    // --- Load message history ---
    let rows = sqlx::query_as::<_, crate::models::message::Message>(
        "SELECT * FROM messages WHERE session_id = ? ORDER BY id LIMIT 500",
    )
    .bind(req.session_id)
    .fetch_all(&state.pool)
    .await?;

    let mut history: Vec<Value> = rows
        .iter()
        .map(|m| {
            let mut entry = json!({"role": m.role, "content": m.content});
            if let Some(ref tc_json) = m.tool_calls {
                if let Ok(tc) = serde_json::from_str::<Value>(tc_json) {
                    entry["tool_calls"] = tc;
                }
            }
            entry
        })
        .collect();

    // --- Load skill system_prompt if session has a skill_id ---
    let system_prompt: Option<String> = if let Some(ref skill_id) = session.skill_id {
        load_skill_system_prompt(&state, skill_id).await
    } else {
        None
    };

    // --- Persist the user message ---
    sqlx::query(
        "INSERT INTO messages (session_id, role, content, created_at) \
         VALUES (?, 'user', ?, datetime('now'))",
    )
    .bind(req.session_id)
    .bind(&req.content)
    .execute(&state.pool)
    .await?;

    history.push(json!({"role": "user", "content": req.content}));

    // --- Update session title on first message ---
    if history.len() == 1 {
        let title: String = req
            .content
            .chars()
            .take(60)
            .collect::<String>()
            .replace('\n', " ");
        sqlx::query(
            "UPDATE sessions SET title = ?, model = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&title)
        .bind(&req.model)
        .bind(req.session_id)
        .execute(&state.pool)
        .await?;
    }

    // --- Build Ollama client ---
    let client = OllamaClient::from_settings(
        &state.pool,
        req.ollama_base_url.as_deref(),
        req.ollama_api_mode.as_deref(),
    )
    .await;

    // --- Spawn the agentic loop and wrap into SSE with DB persistence ---
    let pool = state.pool.clone();
    let session_id = req.session_id;

    let inner_stream = agentic_loop(
        client,
        req.model,
        history,
        system_prompt,
        req.enabled_tools,
        req.temperature,
        req.max_tokens,
        state.config.clone(),
        pool.clone(),
    );

    let sse_stream = async_stream::stream! {
        use futures::StreamExt;

        let mut inner = inner_stream;
        let mut assistant_text = String::new();
        let mut tool_calls_for_db: Vec<Value> = Vec::new();

        while let Some(agent_event) = inner.next().await {
            // Track state for DB persistence.
            match &agent_event {
                AgentEvent::Text { content } => {
                    assistant_text.push_str(content);
                }
                AgentEvent::ToolStart { name, args, call_id } => {
                    tool_calls_for_db.push(json!({
                        "name": name,
                        "args": args,
                        "call_id": call_id,
                    }));
                }
                _ => {}
            }

            // Convert to SSE Event and yield.
            let data = serde_json::to_string(&agent_event).unwrap_or_default();
            let event: Event = Event::default().event("message").data(data);
            yield Ok::<Event, Infallible>(event);
        }

        // --- Persist assistant message after stream ends ---
        if !assistant_text.is_empty() || !tool_calls_for_db.is_empty() {
            let tc_json: Option<String> = if tool_calls_for_db.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&tool_calls_for_db).unwrap_or_default())
            };

            if let Err(e) = sqlx::query(
                "INSERT INTO messages (session_id, role, content, tool_calls, created_at) \
                 VALUES (?, 'assistant', ?, ?, datetime('now'))",
            )
            .bind(session_id)
            .bind(&assistant_text)
            .bind(&tc_json)
            .execute(&pool)
            .await
            {
                tracing::error!("Failed to persist assistant message: {e}");
            }
        }
    };

    Ok(Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

// ------------------------------------------------------------------
// Skill loader helper
// ------------------------------------------------------------------

async fn load_skill_system_prompt(state: &AppState, skill_id: &str) -> Option<String> {
    // Try YAML first, then JSON.
    let yaml_path = state.config.skills_dir().join(format!("{skill_id}.yaml"));
    if yaml_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&yaml_path).await {
            if let Ok(val) = serde_yaml::from_str::<Value>(&content) {
                return val
                    .get("system_prompt")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }
        }
    }

    let json_path = state.config.skills_dir().join(format!("{skill_id}.json"));
    if json_path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
            if let Ok(val) = serde_json::from_str::<Value>(&content) {
                return val
                    .get("system_prompt")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }
        }
    }

    // Try DB fallback (skills table if it exists).
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT system_prompt FROM skills WHERE id = ?",
    )
    .bind(skill_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    row.map(|(sp,)| sp)
}
