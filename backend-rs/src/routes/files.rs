use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs;

use crate::error::AppError;
use crate::state::AppState;

// ── Path safety ──

/// Resolve a user-supplied relative path against workspace_root,
/// rejecting any traversal that escapes the workspace.
fn safe_path(workspace_root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let root = workspace_root
        .canonicalize()
        .map_err(|e| AppError::Internal(format!("Cannot resolve workspace root: {e}")))?;

    // Join and canonicalize — for new files the parent must exist,
    // so we canonicalize the parent then re-append the final component.
    let raw = root.join(relative);

    let resolved = if raw.exists() {
        raw.canonicalize()
            .map_err(|e| AppError::Internal(format!("Cannot resolve path: {e}")))?
    } else {
        // File does not exist yet (write / mkdir / rename destination).
        // Canonicalize the parent directory and append the file name.
        let parent = raw
            .parent()
            .ok_or_else(|| AppError::BadRequest("Invalid path".into()))?;
        let parent_canon = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        let file_name = raw
            .file_name()
            .ok_or_else(|| AppError::BadRequest("Invalid path".into()))?;
        parent_canon.join(file_name)
    };

    if !resolved.starts_with(&root) {
        return Err(AppError::Forbidden("Access denied outside workspace".into()));
    }

    Ok(resolved)
}

/// Return the path relative to workspace_root for JSON responses.
fn relative_display(workspace_root: &Path, abs: &Path) -> String {
    let root = workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf());
    abs.strip_prefix(&root)
        .unwrap_or(abs)
        .to_string_lossy()
        .into_owned()
}

// ── Shared request / response types ──

#[derive(Deserialize)]
pub struct PathQuery {
    #[serde(default = "default_dot")]
    pub path: String,
}

fn default_dot() -> String {
    ".".into()
}

#[derive(Serialize)]
struct EntryInfo {
    name: String,
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
    size: Option<u64>,
    modified: Option<f64>,
    mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_children: Option<bool>,
}

fn entry_info(workspace_root: &Path, path: &Path, meta: &std::fs::Metadata) -> EntryInfo {
    let is_dir = meta.is_dir();
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64());
    let mime = if !is_dir {
        mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())
    } else {
        None
    };

    EntryInfo {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: relative_display(workspace_root, path),
        entry_type: if is_dir {
            "directory".into()
        } else {
            "file".into()
        },
        size: if is_dir { None } else { Some(meta.len()) },
        modified,
        mime,
        has_children: None,
    }
}

// ── GET /tree ──

pub async fn get_tree(
    State(state): State<AppState>,
    Query(q): Query<PathQuery>,
) -> Result<Json<Value>, AppError> {
    let target = safe_path(&state.config.workspace_root, &q.path)?;

    if !target.exists() {
        return Err(AppError::NotFound("Path not found".into()));
    }
    if !target.is_dir() {
        return Err(AppError::BadRequest("Path is not a directory".into()));
    }

    let mut read_dir = fs::read_dir(&target).await?;
    let mut entries: Vec<EntryInfo> = Vec::new();

    while let Some(de) = read_dir.next_entry().await? {
        let path = de.path();
        let meta = de.metadata().await?;
        let mut info = entry_info(&state.config.workspace_root, &path, &meta);

        if meta.is_dir() {
            // Check whether directory has children
            let has = match fs::read_dir(&path).await {
                Ok(mut rd) => rd.next_entry().await?.is_some(),
                Err(_) => false,
            };
            info.has_children = Some(has);
        }

        entries.push(info);
    }

    // Sort: directories first, then alphabetical case-insensitive
    entries.sort_by(|a, b| {
        let a_is_file = a.entry_type == "file";
        let b_is_file = b.entry_type == "file";
        a_is_file
            .cmp(&b_is_file)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(Json(json!({
        "path": q.path,
        "entries": entries,
        "workspace_root": state.config.workspace_root.to_string_lossy(),
    })))
}

// ── GET /read ──

#[derive(Deserialize)]
pub struct ReadQuery {
    pub path: String,
}

const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

pub async fn read_file(
    State(state): State<AppState>,
    Query(q): Query<ReadQuery>,
) -> Result<Json<Value>, AppError> {
    let target = safe_path(&state.config.workspace_root, &q.path)?;

    if !target.exists() {
        return Err(AppError::NotFound("File not found".into()));
    }
    if target.is_dir() {
        return Err(AppError::BadRequest("Path is a directory".into()));
    }

    let max_size = get_max_file_size(&state).await;
    let meta = fs::metadata(&target).await?;
    if meta.len() > max_size {
        return Err(AppError::PayloadTooLarge("File too large".into()));
    }

    let bytes = fs::read(&target).await?;
    let content = String::from_utf8_lossy(&bytes).into_owned();
    let mime = mime_guess::from_path(&target)
        .first()
        .map(|m| m.to_string());

    Ok(Json(json!({
        "path": q.path,
        "content": content,
        "mime": mime,
        "size": meta.len(),
    })))
}

// ── POST /write ──

#[derive(Deserialize)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

pub async fn write_file(
    State(state): State<AppState>,
    Json(body): Json<WriteRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let target = safe_path(&state.config.workspace_root, &body.path)?;

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::write(&target, &body.content).await?;
    let size = body.content.len();

    Ok((
        StatusCode::CREATED,
        Json(json!({ "path": body.path, "size": size })),
    ))
}

// ── DELETE /delete ──

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub path: String,
}

pub async fn delete_path(
    State(state): State<AppState>,
    Json(body): Json<DeleteRequest>,
) -> Result<Json<Value>, AppError> {
    let target = safe_path(&state.config.workspace_root, &body.path)?;

    if !target.exists() {
        return Err(AppError::NotFound("Path not found".into()));
    }

    if target.is_dir() {
        fs::remove_dir(&target).await.map_err(|_| {
            AppError::Conflict("Directory not empty".into())
        })?;
    } else {
        fs::remove_file(&target).await?;
    }

    Ok(Json(json!({ "deleted": body.path })))
}

// ── POST /rename ──

#[derive(Deserialize)]
pub struct RenameRequest {
    pub old_path: String,
    pub new_path: String,
}

pub async fn rename_path(
    State(state): State<AppState>,
    Json(body): Json<RenameRequest>,
) -> Result<Json<Value>, AppError> {
    let src = safe_path(&state.config.workspace_root, &body.old_path)?;
    let dst = safe_path(&state.config.workspace_root, &body.new_path)?;

    if !src.exists() {
        return Err(AppError::NotFound("Source not found".into()));
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).await?;
    }

    fs::rename(&src, &dst).await?;

    Ok(Json(json!({
        "old_path": body.old_path,
        "new_path": body.new_path,
    })))
}

// ── POST /mkdir ──

#[derive(Deserialize)]
pub struct MkdirRequest {
    pub path: String,
}

pub async fn mkdir(
    State(state): State<AppState>,
    Json(body): Json<MkdirRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let target = safe_path(&state.config.workspace_root, &body.path)?;
    fs::create_dir_all(&target).await?;
    Ok((StatusCode::CREATED, Json(json!({ "created": body.path }))))
}

// ── POST /upload ──

#[derive(Deserialize)]
pub struct UploadQuery {
    #[serde(default = "default_dot")]
    pub path: String,
}

pub async fn upload_file(
    State(state): State<AppState>,
    Query(q): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let max_size = get_max_file_size(&state).await;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {e}")))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".into());

        let dest_relative = if q.path == "." {
            filename.clone()
        } else {
            format!("{}/{}", q.path, filename)
        };

        let target = safe_path(&state.config.workspace_root, &dest_relative)?;

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?;

        if data.len() as u64 > max_size {
            return Err(AppError::PayloadTooLarge("File too large".into()));
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&target, &data).await?;

        let rel = relative_display(&state.config.workspace_root, &target);
        return Ok((
            StatusCode::CREATED,
            Json(json!({ "path": rel, "size": data.len() })),
        ));
    }

    Err(AppError::BadRequest("No file provided".into()))
}

// ── Helpers ──

/// Read the max_file_size setting from the database, falling back to the default.
async fn get_max_file_size(state: &AppState) -> u64 {
    let result: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'max_file_size'")
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

    result
        .and_then(|(v,)| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_FILE_SIZE)
}
