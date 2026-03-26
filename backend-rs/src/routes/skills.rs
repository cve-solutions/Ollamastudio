//! Router Skills — YAML-backed personas with system prompt, tools, and parameters.

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;
use crate::models::skill::{default_skills, slugify, Skill, SkillCreate};
use crate::state::AppState;

// ── Helpers I/O ──

fn skill_path(config: &Config, skill_id: &str) -> std::path::PathBuf {
    config.skills_dir().join(format!("{skill_id}.yaml"))
}

/// Load a skill by ID — checks YAML file first, then defaults.
/// Exported for use by the chat router.
pub async fn load_skill(skill_id: &str, config: &Config) -> Option<Skill> {
    let path = skill_path(config, skill_id);
    if path.exists() {
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            if let Ok(mut skill) = serde_yaml::from_str::<Skill>(&content) {
                skill.id = skill_id.to_string();
                return Some(skill);
            }
        }
    }

    // Fall back to built-in defaults
    default_skills()
        .into_iter()
        .find(|s| s.id == skill_id)
}

/// Seed default skills to YAML files on startup.
pub async fn seed_default_skills(config: &Config) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(config.skills_dir()).await?;

    for skill in default_skills() {
        let path = skill_path(config, &skill.id);
        if !path.exists() {
            let data = skill_to_yaml_value(&skill);
            let yaml_str = serde_yaml::to_string(&data)
                .unwrap_or_default();
            tokio::fs::write(&path, yaml_str.as_bytes()).await?;
        }
    }
    tracing::info!("Skills initialis\u{00e9}es dans {}", config.skills_dir().display());
    Ok(())
}

// ── Endpoints ──

/// GET /api/skills/ — list all skills (defaults + custom YAML files).
pub async fn list_skills(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, AppError> {
    let mut skills: std::collections::HashMap<String, Skill> = std::collections::HashMap::new();

    // Load defaults first
    for s in default_skills() {
        skills.insert(s.id.clone(), s);
    }

    // Override / add from YAML files
    let skills_dir = state.config.skills_dir();
    if let Ok(mut entries) = tokio::fs::read_dir(&skills_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let skill_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(mut skill) = serde_yaml::from_str::<Skill>(&content) {
                        skill.id = skill_id.clone();
                        skills.insert(skill_id, skill);
                    }
                }
            }
        }
    }

    let result: Vec<Value> = skills
        .into_values()
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .collect();

    Ok(Json(result))
}

/// GET /api/skills/{id} — get a single skill.
pub async fn get_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let skill = load_skill(&skill_id, &state.config)
        .await
        .ok_or_else(|| AppError::NotFound("Skill introuvable".to_string()))?;
    Ok(Json(serde_json::to_value(skill).unwrap_or_default()))
}

/// POST /api/skills/ — create a new skill (write YAML).
pub async fn create_skill(
    State(state): State<AppState>,
    Json(body): Json<SkillCreate>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let path = skill_path(&state.config, &body.id);
    if path.exists() {
        return Err(AppError::Conflict(
            "Skill existante \u{2014} utilisez PUT pour modifier".to_string(),
        ));
    }

    let skill = skill_from_create(body.id.clone(), body);

    let data = skill_to_yaml_value(&skill);
    let yaml_str =
        serde_yaml::to_string(&data).map_err(|e| AppError::Internal(format!("YAML: {e}")))?;
    tokio::fs::write(&path, yaml_str.as_bytes()).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(&skill).unwrap_or_default()),
    ))
}

/// PUT /api/skills/{id} — update an existing skill (write YAML).
pub async fn update_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
    Json(body): Json<SkillCreate>,
) -> Result<Json<Value>, AppError> {
    let path = skill_path(&state.config, &skill_id);

    let skill = skill_from_create(skill_id, body);

    let data = skill_to_yaml_value(&skill);
    let yaml_str =
        serde_yaml::to_string(&data).map_err(|e| AppError::Internal(format!("YAML: {e}")))?;
    tokio::fs::write(&path, yaml_str.as_bytes()).await?;

    Ok(Json(serde_json::to_value(&skill).unwrap_or_default()))
}

/// DELETE /api/skills/{id} — delete a custom skill (403 for defaults).
pub async fn delete_skill(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, AppError> {
    // Prevent deleting built-in skills
    if default_skills().iter().any(|s| s.id == skill_id) {
        return Err(AppError::Forbidden(
            "Impossible de supprimer une skill par d\u{00e9}faut".to_string(),
        ));
    }

    let path = skill_path(&state.config, &skill_id);
    if !path.exists() {
        return Err(AppError::NotFound("Skill introuvable".to_string()));
    }

    tokio::fs::remove_file(&path).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/skills/import — import skills from a JSON file (multipart).
/// Supports Claude, OllamaStudio, and generic formats.
pub async fn import_skills(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), AppError> {
    state.debug.log("INFO", "skill", "Import skills depuis fichier");

    // Read the single file field
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Erreur multipart: {e}")))?
        .ok_or_else(|| AppError::BadRequest("Aucun fichier fourni".to_string()))?;

    let file_name = field
        .file_name()
        .unwrap_or("unknown")
        .to_string();

    if !file_name.to_lowercase().ends_with(".json") {
        return Err(AppError::BadRequest(
            "Fichier JSON attendu (.json)".to_string(),
        ));
    }

    let content_bytes = field
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("Lecture fichier: {e}")))?;

    if content_bytes.len() > 5 * 1024 * 1024 {
        return Err(AppError::PayloadTooLarge(
            "Fichier trop volumineux (max 5 Mo)".to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&content_bytes);
    let mut data: Value =
        serde_json::from_str(&text).map_err(|e| AppError::BadRequest(format!("JSON invalide: {e}")))?;

    // Accept array or object with known keys
    if let Some(obj) = data.as_object() {
        let mut found = false;
        for key in &["skills", "personas", "agents", "items", "data"] {
            if let Some(val) = obj.get(*key) {
                if val.is_array() {
                    // Direct array: {"skills": [...]}
                    data = val.clone();
                    found = true;
                    break;
                } else if let Some(inner_obj) = val.as_object() {
                    // Nested object: {"skills": {"public": [...], "user": [...], ...}}
                    // Flatten all sub-arrays into one list
                    let mut all_items = Vec::new();
                    for (_sub_key, sub_val) in inner_obj {
                        if let Some(arr) = sub_val.as_array() {
                            all_items.extend(arr.iter().cloned());
                        }
                    }
                    if !all_items.is_empty() {
                        data = Value::Array(all_items);
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            // Single object → wrap in array
            data = Value::Array(vec![data]);
        }
    }

    let items = data
        .as_array()
        .ok_or_else(|| {
            AppError::BadRequest(
                "Format attendu: tableau JSON ou objet avec cl\u{00e9} 'skills'".to_string(),
            )
        })?;

    let mut imported: Vec<Value> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();

    for (i, raw) in items.iter().enumerate() {
        let Some(obj) = raw.as_object() else {
            errors.push(json!({"index": i, "error": "Entr\u{00e9}e non-objet ignor\u{00e9}e"}));
            continue;
        };

        let Some(skill) = normalize_skill(obj, i) else {
            skipped.push(json!({
                "index": i,
                "name": obj.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                "reason": "Pas de prompt/contenu",
            }));
            continue;
        };

        let mut skill_id = skill.id.clone();
        let mut path = skill_path(&state.config, &skill_id);

        // Handle ID collision
        if path.exists() {
            let mut counter = 2u32;
            loop {
                let candidate = format!("{skill_id}-{counter}");
                let candidate_path = skill_path(&state.config, &candidate);
                if !candidate_path.exists() {
                    skill_id = candidate;
                    path = candidate_path;
                    break;
                }
                counter += 1;
            }
        }

        let mut final_skill = skill;
        final_skill.id = skill_id.clone();

        let data = skill_to_yaml_value(&final_skill);
        match serde_yaml::to_string(&data) {
            Ok(yaml_str) => match tokio::fs::write(&path, yaml_str.as_bytes()).await {
                Ok(()) => {
                    state.debug.log("INFO", "skill", &format!("Skill import\u{00e9}e: {skill_id} ({})", final_skill.name));
                    imported.push(json!({"id": skill_id, "name": final_skill.name}));
                }
                Err(e) => {
                    errors.push(json!({"index": i, "name": final_skill.name, "error": e.to_string()}));
                }
            },
            Err(e) => {
                errors.push(json!({"index": i, "name": final_skill.name, "error": e.to_string()}));
            }
        }
    }

    state.debug.log(
        "INFO",
        "skill",
        &format!(
            "Import termin\u{00e9}: {} import\u{00e9}s, {} ignor\u{00e9}s, {} erreurs",
            imported.len(),
            skipped.len(),
            errors.len()
        ),
    );

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "imported": imported.len(),
            "skipped": skipped.len(),
            "errors": errors.len(),
            "details": {
                "imported": imported,
                "skipped": skipped,
                "errors": errors,
            },
        })),
    ))
}

// ── Internal helpers ──

/// Normalize a raw JSON object into a Skill, supporting Claude / OllamaStudio / generic formats.
fn normalize_skill(
    raw: &serde_json::Map<String, Value>,
    index: usize,
) -> Option<Skill> {
    let name = raw
        .get("name")
        .or(raw.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("Skill import\u{00e9}e #{}", index + 1))
        .to_string();

    let skill_id = raw
        .get("id")
        .or(raw.get("slug"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| slugify(&name));

    // Detect the main prompt field — description accepted as last-resort fallback
    let system_prompt = [
        "system_prompt",
        "content",
        "instructions",
        "prompt",
        "system",
        "description",
    ]
    .iter()
    .find_map(|key| raw.get(*key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
    .map(String::from)?;

    // enabled_tools
    let enabled_tools = raw
        .get("enabled_tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<String>>()
        });

    // If system_prompt came from description, keep description as-is
    let description = raw
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(Skill {
        id: skill_id,
        name,
        description,
        icon: raw
            .get("icon")
            .or(raw.get("emoji"))
            .and_then(|v| v.as_str())
            .unwrap_or("\u{1F4E6}")
            .to_string(),
        system_prompt,
        enabled_tools,
        temperature: raw
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7),
        max_tokens: raw
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096) as u32,
        color: raw
            .get("color")
            .and_then(|v| v.as_str())
            .unwrap_or("#6366f1")
            .to_string(),
        category: raw
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        format_tag: raw
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn skill_from_create(id: String, body: SkillCreate) -> Skill {
    Skill {
        id,
        name: body.name,
        description: body.description,
        icon: body.icon,
        system_prompt: body.system_prompt,
        enabled_tools: body.enabled_tools,
        temperature: body.temperature,
        max_tokens: body.max_tokens,
        color: body.color,
        category: body.category,
        format_tag: body.format_tag,
    }
}

/// Convert a Skill to a YAML-serializable Value, excluding the `id` field
/// (the id is encoded in the filename).
fn skill_to_yaml_value(skill: &Skill) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("name".into(), json!(skill.name));
    map.insert("description".into(), json!(skill.description));
    map.insert("icon".into(), json!(skill.icon));
    map.insert("system_prompt".into(), json!(skill.system_prompt));
    map.insert("enabled_tools".into(), json!(skill.enabled_tools));
    map.insert("temperature".into(), json!(skill.temperature));
    map.insert("max_tokens".into(), json!(skill.max_tokens));
    map.insert("color".into(), json!(skill.color));
    if !skill.category.is_empty() {
        map.insert("category".into(), json!(skill.category));
    }
    if !skill.format_tag.is_empty() {
        map.insert("format_tag".into(), json!(skill.format_tag));
    }
    Value::Object(map)
}
