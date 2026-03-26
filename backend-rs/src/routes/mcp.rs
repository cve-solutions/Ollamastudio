use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs;

use crate::error::AppError;
use crate::state::AppState;

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub url: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_server_type", rename = "type")]
    pub server_type: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_server_type() -> String {
    "http".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CallToolQuery {
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
}

// ── Default servers ──

fn default_servers() -> Vec<McpServer> {
    vec![
        McpServer {
            id: "filesystem".into(),
            name: "Filesystem MCP".into(),
            description: "Acces etendu au systeme de fichiers via MCP".into(),
            url: "http://localhost:3001".into(),
            enabled: false,
            server_type: "http".into(),
            headers: HashMap::new(),
        },
        McpServer {
            id: "git".into(),
            name: "Git MCP".into(),
            description: "Operations Git avancees via MCP".into(),
            url: "http://localhost:3002".into(),
            enabled: false,
            server_type: "http".into(),
            headers: HashMap::new(),
        },
        McpServer {
            id: "datagouv".into(),
            name: "DataGouv.FR".into(),
            description: "APIs data.gouv.fr -- portail open data francais".into(),
            url: "https://mcp.data.gouv.fr/mcp".into(),
            enabled: false,
            server_type: "http".into(),
            headers: HashMap::new(),
        },
    ]
}

// ── YAML persistence ──

fn config_path(state: &AppState) -> PathBuf {
    state.config.data_dir.join("mcp_servers.yaml")
}

async fn load_servers(state: &AppState) -> Result<Vec<McpServer>, AppError> {
    let path = config_path(state);
    if !path.exists() {
        return Ok(default_servers());
    }
    let content = fs::read_to_string(&path).await?;
    let servers: Vec<McpServer> =
        serde_yaml::from_str(&content).map_err(|e| AppError::Internal(format!("YAML parse error: {e}")))?;
    Ok(servers)
}

async fn save_servers(state: &AppState, servers: &[McpServer]) -> Result<(), AppError> {
    let path = config_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let yaml =
        serde_yaml::to_string(servers).map_err(|e| AppError::Internal(format!("YAML serialize error: {e}")))?;
    fs::write(&path, yaml.as_bytes()).await?;
    Ok(())
}

/// Read the MCP timeout setting from DB.
async fn mcp_timeout(pool: &sqlx::SqlitePool) -> u64 {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM app_settings WHERE key = 'mcp_timeout'")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    row.and_then(|r| r.0.parse().ok()).unwrap_or(30)
}

/// Find a server by id or return 404.
fn find_server(servers: &[McpServer], id: &str) -> Result<McpServer, AppError> {
    servers
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("Serveur MCP introuvable".into()))
}

// ── Handlers ──

/// GET /servers -- list all MCP servers.
pub async fn list_servers(
    State(state): State<AppState>,
) -> Result<Json<Vec<McpServer>>, AppError> {
    Ok(Json(load_servers(&state).await?))
}

/// POST /servers -- create a new MCP server.
pub async fn add_server(
    State(state): State<AppState>,
    Json(body): Json<McpServer>,
) -> Result<(StatusCode, Json<McpServer>), AppError> {
    let mut servers = load_servers(&state).await?;
    if servers.iter().any(|s| s.id == body.id) {
        return Err(AppError::Conflict("Serveur MCP deja configure".into()));
    }
    servers.push(body.clone());
    save_servers(&state, &servers).await?;
    Ok((StatusCode::CREATED, Json(body)))
}

/// PUT /servers/{id} -- update an existing MCP server.
pub async fn update_server(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
    Json(body): Json<McpServer>,
) -> Result<Json<McpServer>, AppError> {
    let mut servers = load_servers(&state).await?;
    let pos = servers
        .iter()
        .position(|s| s.id == server_id)
        .ok_or_else(|| AppError::NotFound("Serveur MCP introuvable".into()))?;
    servers[pos] = body.clone();
    save_servers(&state, &servers).await?;
    Ok(Json(body))
}

/// DELETE /servers/{id} -- delete an MCP server.
pub async fn delete_server(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let mut servers = load_servers(&state).await?;
    servers.retain(|s| s.id != server_id);
    save_servers(&state, &servers).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /servers/{id}/ping -- test connectivity to an MCP server.
pub async fn ping_server(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let servers = load_servers(&state).await?;
    let server = find_server(&servers, &server_id)?;
    let timeout = mcp_timeout(&state.pool).await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {e}")))?;

    match client.get(&server.url).send().await {
        Ok(resp) => Ok(Json(json!({
            "reachable": true,
            "status": resp.status().as_u16(),
            "url": server.url,
        }))),
        Err(e) => Ok(Json(json!({
            "reachable": false,
            "error": e.to_string(),
            "url": server.url,
        }))),
    }
}

/// GET /servers/{id}/tools -- list tools exposed by an MCP server (JSON-RPC tools/list).
pub async fn list_server_tools(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let servers = load_servers(&state).await?;
    let server = find_server(&servers, &server_id)?;
    let timeout = mcp_timeout(&state.pool).await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {e}")))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    for (k, v) in &server.headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            headers.insert(name, val);
        }
    }

    let resp = client
        .post(&server.url)
        .headers(headers)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| AppError::BadGateway(format!("Erreur MCP: {e}")))?;

    let data: Value = resp
        .json()
        .await
        .map_err(|e| AppError::BadGateway(format!("Invalid JSON from MCP server: {e}")))?;

    let tools = data
        .get("result")
        .and_then(|r| r.get("tools"))
        .cloned()
        .unwrap_or(json!([]));

    Ok(Json(json!({
        "server_id": server_id,
        "tools": tools,
    })))
}

/// POST /servers/{id}/call -- invoke a tool on an MCP server (JSON-RPC tools/call).
pub async fn call_tool(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
    Query(query): Query<CallToolQuery>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let servers = load_servers(&state).await?;
    let server = find_server(&servers, &server_id)?;
    let timeout = mcp_timeout(&state.pool).await;

    let arguments = query.arguments.unwrap_or(body);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {e}")))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    for (k, v) in &server.headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            headers.insert(name, val);
        }
    }

    let resp = client
        .post(&server.url)
        .headers(headers)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": query.tool_name,
                "arguments": arguments,
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| AppError::BadGateway(format!("Erreur appel MCP: {e}")))?;

    let data: Value = resp
        .json()
        .await
        .map_err(|e| AppError::BadGateway(format!("Invalid JSON from MCP server: {e}")))?;

    Ok(Json(data))
}
