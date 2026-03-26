//! Tool schemas and execution for the agentic loop.

use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

use crate::config::Config;

/// Return the full set of tool schemas in OpenAI function-calling format.
pub fn tool_schemas() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative path within workspace"}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write (or create) a file in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative path within workspace"},
                        "content": {"type": "string", "description": "Content to write"}
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "patch_file",
                "description": "Replace a section of an existing file by matching old content and substituting new content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative path of the file"},
                        "old_content": {"type": "string", "description": "Exact content to find and replace"},
                        "new_content": {"type": "string", "description": "Replacement content"}
                    },
                    "required": ["path", "old_content", "new_content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files and directories at a path in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Relative path, '.' for root", "default": "."},
                        "recursive": {"type": "boolean", "default": false}
                    },
                    "required": []
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Execute a shell command in the workspace. Configurable timeout.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "Shell command to execute"},
                        "timeout": {"type": "integer", "default": 30, "description": "Timeout in seconds"}
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "grep_files",
                "description": "Search for a regex pattern in workspace files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Regex pattern"},
                        "path": {"type": "string", "default": ".", "description": "Search directory"},
                        "file_pattern": {"type": "string", "default": "*", "description": "Glob filter (e.g. *.py)"},
                        "max_results": {"type": "integer", "default": 50}
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "git_status",
                "description": "Return the git status of the workspace.",
                "parameters": {"type": "object", "properties": {}, "required": []}
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_documents",
                "description": "Search imported documents by keyword.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search terms"},
                        "max_results": {"type": "integer", "default": 5}
                    },
                    "required": ["query"]
                }
            }
        }),
    ]
}

// ------------------------------------------------------------------
// Path safety
// ------------------------------------------------------------------

/// Resolve a relative path inside the workspace root.
/// Returns an error if the resolved path escapes the workspace.
fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, Value> {
    let target = root.join(relative).canonicalize().unwrap_or_else(|_| root.join(relative));
    let root_canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if !target.starts_with(&root_canon) {
        return Err(json!({"error": format!("Access denied outside workspace: {relative}")}));
    }
    Ok(target)
}

// ------------------------------------------------------------------
// Argument normalization
// ------------------------------------------------------------------

/// Unwrap arguments that LLMs sometimes nest inside "input", "arguments", etc.
pub fn normalize_tool_args(tool_name: &str, args: Value) -> Value {
    if !args.is_object() {
        return args;
    }

    let expected = expected_params(tool_name);
    if expected.is_empty() {
        return args;
    }

    // Try unwrapping common wrapper keys.
    for wrapper in &["input", "arguments", "params", "parameters"] {
        if let Some(inner) = args.get(*wrapper) {
            if inner.is_object() && inner.as_object().unwrap().keys().any(|k| expected.contains(&k.as_str())) {
                return inner.clone();
            }
        }
    }

    // Single-key object wrapping the real args.
    if let Some(obj) = args.as_object() {
        if obj.len() == 1 {
            let only = obj.values().next().unwrap();
            if only.is_object() && only.as_object().unwrap().keys().any(|k| expected.contains(&k.as_str())) {
                return only.clone();
            }
        }
    }

    args
}

fn expected_params(tool_name: &str) -> Vec<&'static str> {
    match tool_name {
        "read_file" => vec!["path"],
        "write_file" => vec!["path", "content"],
        "patch_file" => vec!["path", "old_content", "new_content"],
        "list_files" => vec!["path", "recursive"],
        "run_command" => vec!["command", "timeout"],
        "grep_files" => vec!["pattern", "path", "file_pattern", "max_results"],
        "git_status" => vec![],
        "search_documents" => vec!["query", "max_results"],
        _ => vec![],
    }
}

// ------------------------------------------------------------------
// Dispatcher
// ------------------------------------------------------------------

/// Execute a tool by name. Always returns a JSON value.
pub async fn execute_tool(
    name: &str,
    args: Value,
    config: &Config,
    pool: &SqlitePool,
) -> Value {
    let args = normalize_tool_args(name, args);
    let root = &config.workspace_root;

    match name {
        "read_file" => tool_read_file(root, &args).await,
        "write_file" => tool_write_file(root, &args).await,
        "patch_file" => tool_patch_file(root, &args).await,
        "list_files" => tool_list_files(root, &args).await,
        "run_command" => tool_run_command(root, &args, pool).await,
        "grep_files" => tool_grep_files(root, &args).await,
        "git_status" => tool_git_status(root).await,
        "search_documents" => tool_search_documents(&args, pool).await,
        _ => json!({"error": format!("Unknown tool: {name}")}),
    }
}

// ------------------------------------------------------------------
// Tool implementations
// ------------------------------------------------------------------

async fn tool_read_file(root: &Path, args: &Value) -> Value {
    let rel = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing required argument: path"}),
    };
    let target = match safe_path(root, rel) {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !target.exists() {
        return json!({"error": format!("File not found: {rel}")});
    }
    let meta = match fs::metadata(&target).await {
        Ok(m) => m,
        Err(e) => return json!({"error": format!("{e}")}),
    };
    if meta.len() > 10 * 1024 * 1024 {
        return json!({"error": "File too large (max 10 MB)"});
    }
    match fs::read_to_string(&target).await {
        Ok(content) => {
            let lines = content.matches('\n').count() + 1;
            json!({"path": rel, "content": content, "lines": lines})
        }
        Err(e) => json!({"error": format!("{e}")}),
    }
}

async fn tool_write_file(root: &Path, args: &Value) -> Value {
    let rel = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing required argument: path"}),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing required argument: content"}),
    };
    let target = match safe_path(root, rel) {
        Ok(p) => p,
        Err(e) => return e,
    };
    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent).await {
            return json!({"error": format!("Cannot create directories: {e}")});
        }
    }
    match fs::write(&target, content.as_bytes()).await {
        Ok(()) => json!({"path": rel, "written_bytes": content.len()}),
        Err(e) => json!({"error": format!("{e}")}),
    }
}

async fn tool_patch_file(root: &Path, args: &Value) -> Value {
    let rel = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing required argument: path"}),
    };
    let old_content = match args.get("old_content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing required argument: old_content"}),
    };
    let new_content = match args.get("new_content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing required argument: new_content"}),
    };
    let target = match safe_path(root, rel) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let current = match fs::read_to_string(&target).await {
        Ok(s) => s,
        Err(e) => return json!({"error": format!("Cannot read file: {e}")}),
    };
    if !current.contains(old_content) {
        return json!({"error": "old_content not found in file — patch aborted."});
    }
    let patched = current.replacen(old_content, new_content, 1);
    match fs::write(&target, patched.as_bytes()).await {
        Ok(()) => json!({"path": rel, "written_bytes": patched.len()}),
        Err(e) => json!({"error": format!("{e}")}),
    }
}

async fn tool_list_files(root: &Path, args: &Value) -> Value {
    let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
    let target = match safe_path(root, rel) {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !target.exists() {
        return json!({"error": format!("Path not found: {rel}")});
    }

    let mut entries: Vec<Value> = Vec::new();
    if recursive {
        if let Err(e) = collect_recursive(&target, root, &mut entries).await {
            return json!({"error": format!("{e}")});
        }
    } else {
        let mut read_dir = match fs::read_dir(&target).await {
            Ok(rd) => rd,
            Err(e) => return json!({"error": format!("{e}")}),
        };
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            let rel_path = path.strip_prefix(root).unwrap_or(&path);
            let meta = entry.metadata().await.ok();
            entries.push(json!({
                "path": rel_path.display().to_string(),
                "name": entry.file_name().to_string_lossy(),
                "type": if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) { "directory" } else { "file" },
                "size": meta.and_then(|m| if m.is_file() { Some(m.len()) } else { None }),
            }));
        }
    }

    let count = entries.len();
    json!({"path": rel, "entries": entries, "count": count})
}

async fn collect_recursive(dir: &Path, root: &Path, entries: &mut Vec<Value>) -> std::io::Result<()> {
    let mut read_dir = fs::read_dir(dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let rel_path = path.strip_prefix(root).unwrap_or(&path);
        let meta = entry.metadata().await?;
        let is_dir = meta.is_dir();
        entries.push(json!({
            "path": rel_path.display().to_string(),
            "type": if is_dir { "directory" } else { "file" },
            "size": if meta.is_file() { Some(meta.len()) } else { None },
        }));
        if is_dir {
            Box::pin(collect_recursive(&path, root, entries)).await?;
        }
    }
    Ok(())
}

async fn tool_run_command(root: &Path, args: &Value, pool: &SqlitePool) -> Value {
    let cmd = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing required argument: command"}),
    };
    let shell_timeout = crate::services::ollama::get_setting_i64(pool, "shell_timeout", 30).await;
    let shell_max_output = crate::services::ollama::get_setting_i64(pool, "shell_max_output", 65536).await as usize;
    let req_timeout = args.get("timeout").and_then(|v| v.as_i64()).unwrap_or(30);
    let timeout = std::cmp::min(req_timeout, shell_timeout) as u64;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            json!({
                "command": cmd,
                "stdout": &stdout[..std::cmp::min(stdout.len(), shell_max_output)],
                "stderr": &stderr[..std::cmp::min(stderr.len(), shell_max_output)],
                "returncode": output.status.code(),
            })
        }
        Ok(Err(e)) => json!({"error": format!("{e}"), "command": cmd}),
        Err(_) => json!({"error": format!("Timeout ({timeout}s) exceeded"), "command": cmd}),
    }
}

async fn tool_grep_files(root: &Path, args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing required argument: pattern"}),
    };
    let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let file_pattern = args.get("file_pattern").and_then(|v| v.as_str()).unwrap_or("*");
    let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let target = match safe_path(root, rel) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        Command::new("grep")
            .args(["-rn", "--include", file_pattern, "-E", pattern])
            .arg(&target)
            .current_dir(root)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let root_prefix = format!("{}/", root.display());
            let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .take(max_results)
                .map(|l| l.replace(&root_prefix, ""))
                .collect();
            let count = lines.len();
            json!({"pattern": pattern, "matches": lines, "count": count})
        }
        Ok(Err(e)) => json!({"error": format!("{e}")}),
        Err(_) => json!({"error": "Timeout grep"}),
    }
}

async fn tool_git_status(root: &Path) -> Value {
    let result = Command::new("git")
        .args(["status", "--short", "--branch"])
        .current_dir(root)
        .output()
        .await;

    match result {
        Ok(output) => {
            if !output.status.success() {
                return json!({
                    "error": "Not a git repository or git not available",
                    "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }
            json!({"output": String::from_utf8_lossy(&output.stdout).to_string()})
        }
        Err(e) => json!({"error": format!("{e}")}),
    }
}

async fn tool_search_documents(args: &Value, pool: &SqlitePool) -> Value {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return json!({"error": "Missing required argument: query"}),
    };
    let max_results = args.get("max_results").and_then(|v| v.as_i64()).unwrap_or(5);

    let terms: Vec<&str> = query.split_whitespace().take(5).collect();
    if terms.is_empty() {
        return json!({"results": [], "count": 0, "query": query});
    }

    // Build a WHERE clause with LIKE for each term (OR'd together).
    let conditions: Vec<String> = terms
        .iter()
        .enumerate()
        .map(|(i, _)| format!("content LIKE '%' || ?{} || '%'", i + 1))
        .collect();
    let where_clause = conditions.join(" OR ");
    let sql = format!(
        "SELECT id, document_id, content FROM document_chunks WHERE ({}) LIMIT ?",
        where_clause
    );

    let mut q = sqlx::query_as::<_, (i64, i64, String)>(&sql);
    for term in &terms {
        q = q.bind(term.to_lowercase());
    }
    q = q.bind(max_results);

    match q.fetch_all(pool).await {
        Ok(rows) => {
            let results: Vec<Value> = rows
                .iter()
                .map(|(id, doc_id, content)| {
                    json!({
                        "chunk_id": id,
                        "document_id": doc_id,
                        "excerpt": &content[..std::cmp::min(content.len(), 300)],
                    })
                })
                .collect();
            let count = results.len();
            json!({"query": query, "results": results, "count": count})
        }
        Err(e) => {
            // Table may not exist yet — not a fatal error.
            tracing::warn!("search_documents query failed: {e}");
            json!({"query": query, "results": [], "count": 0, "note": "search unavailable"})
        }
    }
}
