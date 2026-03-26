//! Router Templates — YAML-backed prompt snippets with variable placeholders.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;
use crate::models::template::{default_templates, Template, TemplateCreate};
use crate::state::AppState;

// ── Helpers I/O ──

fn template_path(config: &Config, template_id: &str) -> std::path::PathBuf {
    config.templates_dir().join(format!("{template_id}.yaml"))
}

/// Seed default templates to YAML files on startup.
pub async fn seed_default_templates(config: &Config) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(config.templates_dir()).await?;

    for tpl in default_templates() {
        let path = template_path(config, &tpl.id);
        if !path.exists() {
            let data = template_to_yaml_value(&tpl);
            let yaml_str = serde_yaml::to_string(&data).unwrap_or_default();
            tokio::fs::write(&path, yaml_str.as_bytes()).await?;
        }
    }
    tracing::info!(
        "Templates initialis\u{00e9}s dans {}",
        config.templates_dir().display()
    );
    Ok(())
}

// ── Endpoints ──

/// GET /api/templates/ — list all templates (defaults + custom YAML files).
pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, AppError> {
    let mut templates: std::collections::HashMap<String, Template> =
        std::collections::HashMap::new();

    // Load defaults first
    for tpl in default_templates() {
        templates.insert(tpl.id.clone(), tpl);
    }

    // Override / add from YAML files
    let templates_dir = state.config.templates_dir();
    if let Ok(mut entries) = tokio::fs::read_dir(&templates_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let tpl_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(mut tpl) = serde_yaml::from_str::<Template>(&content) {
                        tpl.id = tpl_id.clone();
                        templates.insert(tpl_id, tpl);
                    }
                }
            }
        }
    }

    // Sort by category then name
    let mut result: Vec<Value> = templates
        .into_values()
        .collect::<Vec<_>>()
        .into_iter()
        .map(|t| serde_json::to_value(t).unwrap_or_default())
        .collect();

    result.sort_by(|a, b| {
        let cat_a = a.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let cat_b = b.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let name_a = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_b = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        cat_a.cmp(cat_b).then(name_a.cmp(name_b))
    });

    Ok(Json(result))
}

/// GET /api/templates/{id} — get a single template.
pub async fn get_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Check YAML file first
    let path = template_path(&state.config, &template_id);
    if path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            if let Ok(mut tpl) = serde_yaml::from_str::<Template>(&content) {
                tpl.id = template_id;
                return Ok(Json(serde_json::to_value(tpl).unwrap_or_default()));
            }
        }
    }

    // Fall back to defaults
    for tpl in default_templates() {
        if tpl.id == template_id {
            return Ok(Json(serde_json::to_value(tpl).unwrap_or_default()));
        }
    }

    Err(AppError::NotFound("Template introuvable".to_string()))
}

/// POST /api/templates/ — create a new template (write YAML).
pub async fn create_template(
    State(state): State<AppState>,
    Json(body): Json<TemplateCreate>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let path = template_path(&state.config, &body.id);
    if path.exists() {
        return Err(AppError::Conflict("Template existant".to_string()));
    }

    let tpl = Template {
        id: body.id.clone(),
        name: body.name,
        category: body.category,
        icon: body.icon,
        content: body.content,
        variables: body.variables,
    };

    let data = template_to_yaml_value(&tpl);
    let yaml_str =
        serde_yaml::to_string(&data).map_err(|e| AppError::Internal(format!("YAML: {e}")))?;
    tokio::fs::write(&path, yaml_str.as_bytes()).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(&tpl).unwrap_or_default()),
    ))
}

/// PUT /api/templates/{id} — update an existing template (write YAML).
pub async fn update_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
    Json(body): Json<TemplateCreate>,
) -> Result<Json<Value>, AppError> {
    let path = template_path(&state.config, &template_id);

    let tpl = Template {
        id: template_id,
        name: body.name,
        category: body.category,
        icon: body.icon,
        content: body.content,
        variables: body.variables,
    };

    let data = template_to_yaml_value(&tpl);
    let yaml_str =
        serde_yaml::to_string(&data).map_err(|e| AppError::Internal(format!("YAML: {e}")))?;
    tokio::fs::write(&path, yaml_str.as_bytes()).await?;

    Ok(Json(serde_json::to_value(&tpl).unwrap_or_default()))
}

/// DELETE /api/templates/{id} — delete a custom template (403 for defaults).
pub async fn delete_template(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
) -> Result<StatusCode, AppError> {
    // Prevent deleting built-in templates
    if default_templates().iter().any(|t| t.id == template_id) {
        return Err(AppError::Forbidden(
            "Impossible de supprimer un template par d\u{00e9}faut".to_string(),
        ));
    }

    let path = template_path(&state.config, &template_id);
    if !path.exists() {
        return Err(AppError::NotFound("Template introuvable".to_string()));
    }

    tokio::fs::remove_file(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Internal helpers ──

/// Convert a Template to a YAML-serializable Value, excluding the `id` field.
fn template_to_yaml_value(tpl: &Template) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("name".into(), json!(tpl.name));
    map.insert("category".into(), json!(tpl.category));
    map.insert("icon".into(), json!(tpl.icon));
    map.insert("content".into(), json!(tpl.content));
    map.insert("variables".into(), json!(tpl.variables));
    Value::Object(map)
}
