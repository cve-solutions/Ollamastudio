//! Router Documents — file upload, folder import, chunking, and full-text search.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;

use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::AppError;
use crate::services::document_processor;
use crate::state::AppState;

/// Supported text file extensions.
static TEXT_EXTENSIONS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        ".txt", ".md", ".rst", ".py", ".rs", ".go", ".js", ".ts", ".tsx", ".jsx",
        ".java", ".c", ".cpp", ".h", ".hpp", ".cs", ".rb", ".php", ".sh", ".bash",
        ".yaml", ".yml", ".toml", ".json", ".xml", ".html", ".css", ".scss",
        ".sql", ".dockerfile", ".env", ".conf", ".cfg", ".ini",
    ])
});

// ── DB row types ──

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
struct DocumentRow {
    id: i64,
    filename: String,
    relative_path: String,
    mime_type: String,
    size_bytes: i64,
    chunk_count: i64,
    imported_at: String,
}

// ── Query params ──

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_max_results")]
    pub max_results: i64,
}

fn default_max_results() -> i64 {
    10
}

#[derive(Debug, Deserialize)]
pub struct FolderImportRequest {
    pub folder_path: String,
}

// ── Endpoints ──

/// GET /api/documents/ — list all documents.
pub async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, AppError> {
    let rows = sqlx::query_as::<_, DocumentRow>(
        "SELECT id, filename, relative_path, mime_type, size_bytes, chunk_count, imported_at \
         FROM documents ORDER BY imported_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let docs: Vec<Value> = rows
        .into_iter()
        .map(|d| {
            json!({
                "id": d.id,
                "filename": d.filename,
                "relative_path": d.relative_path,
                "mime_type": d.mime_type,
                "size_bytes": d.size_bytes,
                "chunk_count": d.chunk_count,
                "imported_at": d.imported_at,
            })
        })
        .collect();

    Ok(Json(docs))
}

/// POST /api/documents/upload — multipart file upload with chunking.
pub async fn upload_documents(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), AppError> {
    state.debug.log("INFO", "import", "Upload de fichier(s)");

    let max_file_size = get_setting_i64(&state, "max_file_size", 10 * 1024 * 1024).await;
    let chunk_size = get_setting_i64(&state, "chunk_size", 1500).await as usize;
    let chunk_overlap = get_setting_i64(&state, "chunk_overlap", 150).await as usize;

    let mut imported: Vec<Value> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Erreur multipart: {e}"))
    })? {
        let file_name = match field.file_name() {
            Some(n) => n.to_string(),
            None => continue,
        };

        let ext = file_extension(&file_name);
        if !TEXT_EXTENSIONS.contains(ext.as_str()) {
            let msg = format!("Extension non support\u{00e9}e: {ext}");
            state.debug.log("WARN", "import", &format!("Fichier {file_name} rejet\u{00e9}: {msg}"));
            errors.push(json!({"file": file_name, "error": msg}));
            continue;
        }

        let content_bytes = field.bytes().await.map_err(|e| {
            AppError::Internal(format!("Lecture fichier: {e}"))
        })?;

        if content_bytes.len() as i64 > max_file_size {
            let msg = format!(
                "Fichier trop volumineux ({} > {})",
                content_bytes.len(),
                max_file_size
            );
            state.debug.log("WARN", "import", &format!("Fichier {file_name}: {msg}"));
            errors.push(json!({"file": file_name, "error": msg}));
            continue;
        }

        let text = String::from_utf8_lossy(&content_bytes).to_string();

        // Save file to disk
        let dest = state.config.documents_dir.join(&file_name);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest, &content_bytes).await?;

        // Index
        match index_text(
            &state,
            &file_name,
            &file_name,
            &text,
            content_bytes.len() as i64,
            chunk_size,
            chunk_overlap,
        )
        .await
        {
            Ok(result) => {
                state.debug.log("INFO", "import", &format!("Index\u{00e9}: {file_name} -> {} chunks", result["chunks"]));
                imported.push(result);
            }
            Err(e) => {
                state.debug.log("ERROR", "import", &format!("Indexation \u{00e9}chou\u{00e9}e pour {file_name}: {e:?}"));
                errors.push(json!({"file": file_name, "error": format!("Indexation: {e:?}")}));
            }
        }
    }

    state.debug.log(
        "INFO",
        "import",
        &format!("Upload termin\u{00e9}: {} import\u{00e9}s, {} erreurs", imported.len(), errors.len()),
    );

    Ok((
        StatusCode::CREATED,
        Json(json!({"imported": imported, "errors": errors})),
    ))
}

/// POST /api/documents/import-folder — recursive folder scan.
pub async fn import_folder(
    State(state): State<AppState>,
    Json(body): Json<FolderImportRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let folder = PathBuf::from(&body.folder_path).canonicalize().map_err(|_| {
        AppError::NotFound(format!("Dossier inexistant: {}", body.folder_path))
    })?;

    state.debug.log("INFO", "import", &format!("Import dossier: {}", folder.display()));

    if !folder.is_dir() {
        return Err(AppError::NotFound(format!(
            "Ce n'est pas un dossier: {}",
            folder.display()
        )));
    }

    // Check readability
    let entries: Vec<_> = std::fs::read_dir(&folder)
        .map_err(|_| {
            AppError::Forbidden(format!(
                "Permission refus\u{00e9}e pour lire le dossier {}. V\u{00e9}rifiez les permissions ou ajoutez :z au volume Docker.",
                folder.display()
            ))
        })?
        .collect();

    if entries.is_empty() {
        return Err(AppError::NotFound(format!(
            "Le dossier est vide: {}",
            folder.display()
        )));
    }

    let max_file_size = get_setting_i64(&state, "max_file_size", 10 * 1024 * 1024).await;
    let chunk_size = get_setting_i64(&state, "chunk_size", 1500).await as usize;
    let chunk_overlap = get_setting_i64(&state, "chunk_overlap", 150).await as usize;

    let mut imported: Vec<Value> = Vec::new();
    let mut errors: Vec<Value> = Vec::new();
    let mut skipped: i64 = 0;

    // Recursive walk
    let all_files = walk_dir_sorted(&folder);
    state.debug.log("DEBUG", "import", &format!("Trouv\u{00e9} {} entr\u{00e9}es dans {}", all_files.len(), folder.display()));

    for path in &all_files {
        if !path.is_file() {
            continue;
        }

        let ext = file_extension(&path.to_string_lossy());
        if !TEXT_EXTENSIONS.contains(ext.as_str()) {
            skipped += 1;
            continue;
        }

        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(e) => {
                errors.push(json!({"file": path.to_string_lossy(), "error": e.to_string()}));
                continue;
            }
        };
        let file_size = metadata.len() as i64;

        if file_size > max_file_size {
            let msg = format!("Trop volumineux ({file_size} > {max_file_size})");
            errors.push(json!({"file": path.to_string_lossy(), "error": msg}));
            continue;
        }

        match tokio::fs::read_to_string(path).await {
            Ok(text) => {
                let rel_path = path
                    .strip_prefix(&folder)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| rel_path.clone());

                match index_text(
                    &state,
                    &filename,
                    &rel_path,
                    &text,
                    file_size,
                    chunk_size,
                    chunk_overlap,
                )
                .await
                {
                    Ok(result) => {
                        state.debug.log("DEBUG", "import", &format!("Index\u{00e9}: {rel_path} -> {} chunks", result["chunks"]));
                        imported.push(result);
                    }
                    Err(e) => {
                        errors.push(json!({"file": path.to_string_lossy(), "error": format!("{e:?}")}));
                    }
                }
            }
            Err(e) => {
                errors.push(json!({"file": path.to_string_lossy(), "error": e.to_string()}));
            }
        }
    }

    state.debug.log(
        "INFO",
        "import",
        &format!(
            "Import dossier termin\u{00e9}: {} import\u{00e9}s, {} ignor\u{00e9}s, {} erreurs",
            imported.len(),
            skipped,
            errors.len()
        ),
    );

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "folder": folder.to_string_lossy(),
            "imported": imported.len(),
            "skipped": skipped,
            "errors": errors,
            "files": imported,
        })),
    ))
}

/// GET /api/documents/search?q=&max_results= — full-text search in chunks.
pub async fn search_documents(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, AppError> {
    let terms: Vec<String> = params
        .q
        .to_lowercase()
        .split_whitespace()
        .filter(|t| t.len() > 2)
        .map(String::from)
        .take(5)
        .collect();

    if terms.is_empty() {
        return Ok(Json(json!({"results": [], "query": params.q})));
    }

    let mut results: Vec<Value> = Vec::new();

    // Build LIKE patterns for each search term
    let like_patterns: Vec<String> = terms.iter().map(|t| format!("%{t}%")).collect();

    // Build with up to 5 terms
    let rows: Vec<SearchResultRow> = match like_patterns.len() {
        1 => {
            sqlx::query_as::<_, SearchResultRow>(
                "SELECT dc.chunk_index, dc.content, d.filename, d.relative_path \
                 FROM document_chunks dc \
                 JOIN documents d ON dc.document_id = d.id \
                 WHERE dc.content LIKE ?1 \
                 LIMIT ?2",
            )
            .bind(&like_patterns[0])
            .bind(params.max_results)
            .fetch_all(&state.pool)
            .await?
        }
        2 => {
            sqlx::query_as::<_, SearchResultRow>(
                "SELECT dc.chunk_index, dc.content, d.filename, d.relative_path \
                 FROM document_chunks dc \
                 JOIN documents d ON dc.document_id = d.id \
                 WHERE dc.content LIKE ?1 OR dc.content LIKE ?2 \
                 LIMIT ?3",
            )
            .bind(&like_patterns[0])
            .bind(&like_patterns[1])
            .bind(params.max_results)
            .fetch_all(&state.pool)
            .await?
        }
        3 => {
            sqlx::query_as::<_, SearchResultRow>(
                "SELECT dc.chunk_index, dc.content, d.filename, d.relative_path \
                 FROM document_chunks dc \
                 JOIN documents d ON dc.document_id = d.id \
                 WHERE dc.content LIKE ?1 OR dc.content LIKE ?2 OR dc.content LIKE ?3 \
                 LIMIT ?4",
            )
            .bind(&like_patterns[0])
            .bind(&like_patterns[1])
            .bind(&like_patterns[2])
            .bind(params.max_results)
            .fetch_all(&state.pool)
            .await?
        }
        4 => {
            sqlx::query_as::<_, SearchResultRow>(
                "SELECT dc.chunk_index, dc.content, d.filename, d.relative_path \
                 FROM document_chunks dc \
                 JOIN documents d ON dc.document_id = d.id \
                 WHERE dc.content LIKE ?1 OR dc.content LIKE ?2 OR dc.content LIKE ?3 OR dc.content LIKE ?4 \
                 LIMIT ?5",
            )
            .bind(&like_patterns[0])
            .bind(&like_patterns[1])
            .bind(&like_patterns[2])
            .bind(&like_patterns[3])
            .bind(params.max_results)
            .fetch_all(&state.pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, SearchResultRow>(
                "SELECT dc.chunk_index, dc.content, d.filename, d.relative_path \
                 FROM document_chunks dc \
                 JOIN documents d ON dc.document_id = d.id \
                 WHERE dc.content LIKE ?1 OR dc.content LIKE ?2 OR dc.content LIKE ?3 OR dc.content LIKE ?4 OR dc.content LIKE ?5 \
                 LIMIT ?6",
            )
            .bind(&like_patterns[0])
            .bind(&like_patterns[1])
            .bind(&like_patterns[2])
            .bind(&like_patterns[3])
            .bind(&like_patterns[4])
            .bind(params.max_results)
            .fetch_all(&state.pool)
            .await?
        }
    };

    for row in &rows {
        let excerpt: String = row.content.chars().take(400).collect();
        results.push(json!({
            "document": row.filename,
            "path": row.relative_path,
            "chunk_index": row.chunk_index,
            "excerpt": excerpt,
        }));
    }

    let count = results.len();
    Ok(Json(json!({
        "query": params.q,
        "results": results,
        "count": count,
    })))
}

/// DELETE /api/documents/{id} — delete document and all chunks.
pub async fn delete_document(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<i64>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM document_chunks WHERE document_id = ?")
        .bind(document_id)
        .execute(&state.pool)
        .await?;

    sqlx::query("DELETE FROM documents WHERE id = ?")
        .bind(document_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ──

#[derive(Debug, sqlx::FromRow)]
struct SearchResultRow {
    chunk_index: i64,
    content: String,
    filename: String,
    relative_path: String,
}

/// Index text content: chunk it and persist to documents + document_chunks tables.
async fn index_text(
    state: &AppState,
    filename: &str,
    relative_path: &str,
    text: &str,
    size_bytes: i64,
    chunk_size: usize,
    chunk_overlap: usize,
) -> Result<Value, AppError> {
    let mime = mime_guess::from_path(filename)
        .first_or_octet_stream()
        .to_string();

    let chunks = document_processor::process_file(filename, text, chunk_size, chunk_overlap);

    let chunk_count = chunks.len() as i64;

    // Insert document
    let doc_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO documents (filename, relative_path, mime_type, size_bytes, chunk_count) \
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(filename)
    .bind(relative_path)
    .bind(&mime)
    .bind(size_bytes)
    .bind(chunk_count)
    .fetch_one(&state.pool)
    .await?;

    // Insert chunks
    for (i, (chunk_content, token_count)) in chunks.iter().enumerate() {
        sqlx::query(
            "INSERT INTO document_chunks (document_id, chunk_index, content, tokens) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(doc_id)
        .bind(i as i64)
        .bind(chunk_content)
        .bind(*token_count as i64)
        .execute(&state.pool)
        .await?;
    }

    Ok(json!({
        "id": doc_id,
        "filename": filename,
        "chunks": chunk_count,
    }))
}

/// Get a setting value from DB as i64, with a fallback default.
async fn get_setting_i64(state: &AppState, key: &str, default: i64) -> i64 {
    sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}

/// Extract file extension (lowercase, including the dot).
fn file_extension(filename: &str) -> String {
    match filename.rfind('.') {
        Some(i) => filename[i..].to_lowercase(),
        None => String::new(),
    }
}

/// Recursively walk a directory, returning sorted file paths.
fn walk_dir_sorted(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir_recursive(dir, &mut files);
    files.sort();
    files
}

fn walk_dir_recursive(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir_recursive(&path, out);
            } else {
                out.push(path);
            }
        }
    }
}
