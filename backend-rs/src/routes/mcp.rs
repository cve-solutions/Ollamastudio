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

/// Types de serveur MCP supportés :
/// - "http"    : serveur MCP JSON-RPC générique
/// - "github"  : GitHub API (token dans auth_token)
/// - "gitlab"  : GitLab API (token dans auth_token)
/// - "command" : commande locale (ex: npx @modelcontextprotocol/server-*)
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
    /// Token d'authentification (GitHub PAT, GitLab token, etc.)
    /// Stocké en clair dans le YAML — ne pas exposer via l'API publique
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub auth_token: String,
    /// Organisation/groupe par défaut (optionnel)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub organization: String,
    /// Catégorie pour le regroupement dans l'UI
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
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
            id: "agent-mcp".into(),
            name: "Agent MCP (Système)".into(),
            description: "Administration système complète — fichiers, packages, services, cron, réseau, processus (root)".into(),
            url: "http://127.0.0.1:9100".into(),
            enabled: true,
            server_type: "http".into(),
            headers: HashMap::new(),
            auth_token: String::new(),
            organization: String::new(),
            category: "Système".into(),
        },
        McpServer {
            id: "github".into(),
            name: "GitHub".into(),
            description: "Repositories, issues, pull requests, actions, releases GitHub".into(),
            url: "https://api.github.com".into(),
            enabled: false,
            server_type: "github".into(),
            headers: HashMap::new(),
            auth_token: String::new(), // → Renseigner un Personal Access Token
            organization: String::new(),
            category: "Git".into(),
        },
        McpServer {
            id: "gitlab".into(),
            name: "GitLab".into(),
            description: "Repositories, merge requests, pipelines, registres GitLab".into(),
            url: "https://gitlab.com".into(),
            enabled: false,
            server_type: "gitlab".into(),
            headers: HashMap::new(),
            auth_token: String::new(), // → Renseigner un Personal Access Token
            organization: String::new(),
            category: "Git".into(),
        },
        McpServer {
            id: "datagouv".into(),
            name: "DataGouv.FR".into(),
            description: "APIs data.gouv.fr — portail open data français".into(),
            url: "https://mcp.data.gouv.fr/mcp".into(),
            enabled: false,
            server_type: "http".into(),
            headers: HashMap::new(),
            auth_token: String::new(),
            organization: String::new(),
            category: "Open Data".into(),
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
        // Persiste les defaults pour qu'ils soient modifiables via l'UI
        let defaults = default_servers();
        save_servers(state, &defaults).await?;
        return Ok(defaults);
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

/// Build HTTP headers for a server, including auth for GitHub/GitLab.
fn build_headers(server: &McpServer) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/json".parse().unwrap(),
    );

    // Custom headers from config
    for (k, v) in &server.headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            headers.insert(name, val);
        }
    }

    // Auth token → header selon le provider
    if !server.auth_token.is_empty() {
        match server.server_type.as_str() {
            "github" => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(
                    &format!("Bearer {}", server.auth_token),
                ) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
                // GitHub API version
                if let Ok(val) = reqwest::header::HeaderValue::from_str("2022-11-28") {
                    headers.insert(
                        reqwest::header::HeaderName::from_static("x-github-api-version"),
                        val,
                    );
                }
            }
            "gitlab" => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&server.auth_token) {
                    headers.insert(
                        reqwest::header::HeaderName::from_static("private-token"),
                        val,
                    );
                }
            }
            _ => {
                // Générique : Bearer token
                if let Ok(val) = reqwest::header::HeaderValue::from_str(
                    &format!("Bearer {}", server.auth_token),
                ) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
        }
    }

    headers
}

/// Build the ping URL based on server type.
fn ping_url(server: &McpServer) -> String {
    match server.server_type.as_str() {
        "github" => format!("{}/user", server.url.trim_end_matches('/')),
        "gitlab" => format!("{}/api/v4/user", server.url.trim_end_matches('/')),
        _ => server.url.clone(),
    }
}

/// Build tools list for GitHub/GitLab (built-in, not from JSON-RPC).
fn provider_tools(server: &McpServer) -> Option<Value> {
    match server.server_type.as_str() {
        "github" => Some(json!([
            {"name": "repos_list", "description": "Lister les repositories", "inputSchema": {"type": "object", "properties": {"org": {"type": "string"}, "per_page": {"type": "integer"}}, "required": []}},
            {"name": "repo_get", "description": "Détails d'un repository", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}}, "required": ["owner", "repo"]}},
            {"name": "issues_list", "description": "Lister les issues d'un repo", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "state": {"type": "string"}}, "required": ["owner", "repo"]}},
            {"name": "issue_create", "description": "Créer une issue", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"}}, "required": ["owner", "repo", "title"]}},
            {"name": "pulls_list", "description": "Lister les pull requests", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "state": {"type": "string"}}, "required": ["owner", "repo"]}},
            {"name": "pr_create", "description": "Créer une pull request", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"}, "head": {"type": "string"}, "base": {"type": "string"}}, "required": ["owner", "repo", "title", "head", "base"]}},
            {"name": "actions_list", "description": "Lister les workflow runs", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}}, "required": ["owner", "repo"]}},
            {"name": "releases_list", "description": "Lister les releases", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}}, "required": ["owner", "repo"]}},
            {"name": "file_read", "description": "Lire un fichier depuis un repo", "inputSchema": {"type": "object", "properties": {"owner": {"type": "string"}, "repo": {"type": "string"}, "path": {"type": "string"}, "ref": {"type": "string"}}, "required": ["owner", "repo", "path"]}},
            {"name": "search_code", "description": "Rechercher du code sur GitHub", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}},
        ])),
        "gitlab" => Some(json!([
            {"name": "projects_list", "description": "Lister les projets", "inputSchema": {"type": "object", "properties": {"group": {"type": "string"}, "per_page": {"type": "integer"}}, "required": []}},
            {"name": "project_get", "description": "Détails d'un projet", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}}, "required": ["project_id"]}},
            {"name": "issues_list", "description": "Lister les issues d'un projet", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}, "state": {"type": "string"}}, "required": ["project_id"]}},
            {"name": "issue_create", "description": "Créer une issue", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}, "title": {"type": "string"}, "description": {"type": "string"}}, "required": ["project_id", "title"]}},
            {"name": "merge_requests_list", "description": "Lister les merge requests", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}, "state": {"type": "string"}}, "required": ["project_id"]}},
            {"name": "mr_create", "description": "Créer une merge request", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}, "title": {"type": "string"}, "source_branch": {"type": "string"}, "target_branch": {"type": "string"}}, "required": ["project_id", "title", "source_branch", "target_branch"]}},
            {"name": "pipelines_list", "description": "Lister les pipelines CI", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}}, "required": ["project_id"]}},
            {"name": "file_read", "description": "Lire un fichier depuis un projet", "inputSchema": {"type": "object", "properties": {"project_id": {"type": "string"}, "path": {"type": "string"}, "ref": {"type": "string"}}, "required": ["project_id", "path"]}},
            {"name": "search_code", "description": "Rechercher du code sur GitLab", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "project_id": {"type": "string"}}, "required": ["query"]}},
        ])),
        _ => None,
    }
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

    let url = ping_url(&server);
    let headers = build_headers(&server);
    match client.get(&url).headers(headers).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            let detail: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            Ok(Json(json!({
                "reachable": true,
                "authenticated": status >= 200 && status < 300,
                "status": status,
                "url": server.url,
                "detail": detail,
            })))
        }
        Err(e) => Ok(Json(json!({
            "reachable": false,
            "authenticated": false,
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

    // GitHub/GitLab : retourne les outils built-in
    if let Some(tools) = provider_tools(&server) {
        return Ok(Json(json!({
            "server_id": server_id,
            "tools": tools,
        })));
    }

    // MCP JSON-RPC : appel tools/list
    let headers = build_headers(&server);
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

    let headers = build_headers(&server);
    let base = server.url.trim_end_matches('/');

    // GitHub/GitLab : route vers les APIs REST natives
    match server.server_type.as_str() {
        "github" => {
            let data = call_github(&client, base, &query.tool_name, &arguments, headers).await?;
            return Ok(Json(data));
        }
        "gitlab" => {
            let data = call_gitlab(&client, base, &query.tool_name, &arguments, headers).await?;
            return Ok(Json(data));
        }
        _ => {}
    }

    // MCP JSON-RPC : appel tools/call
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

// ── GitHub API adapter ──

async fn call_github(
    client: &reqwest::Client,
    base: &str,
    tool: &str,
    args: &Value,
    headers: reqwest::header::HeaderMap,
) -> Result<Value, AppError> {
    let gs = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();

    let (method, url, body) = match tool {
        "repos_list" => {
            let org = gs("org");
            let url = if org.is_empty() {
                format!("{base}/user/repos?per_page=100&sort=updated")
            } else {
                format!("{base}/orgs/{org}/repos?per_page=100&sort=updated")
            };
            ("GET", url, None)
        }
        "repo_get" => ("GET", format!("{base}/repos/{}/{}", gs("owner"), gs("repo")), None),
        "issues_list" => {
            let state = if gs("state").is_empty() { "open".to_string() } else { gs("state") };
            ("GET", format!("{base}/repos/{}/{}/issues?state={state}", gs("owner"), gs("repo")), None)
        }
        "issue_create" => ("POST", format!("{base}/repos/{}/{}/issues", gs("owner"), gs("repo")),
            Some(json!({"title": gs("title"), "body": gs("body")}))),
        "pulls_list" => {
            let state = if gs("state").is_empty() { "open".to_string() } else { gs("state") };
            ("GET", format!("{base}/repos/{}/{}/pulls?state={state}", gs("owner"), gs("repo")), None)
        }
        "pr_create" => ("POST", format!("{base}/repos/{}/{}/pulls", gs("owner"), gs("repo")),
            Some(json!({"title": gs("title"), "body": gs("body"), "head": gs("head"), "base": gs("base")}))),
        "actions_list" => ("GET", format!("{base}/repos/{}/{}/actions/runs?per_page=20", gs("owner"), gs("repo")), None),
        "releases_list" => ("GET", format!("{base}/repos/{}/{}/releases", gs("owner"), gs("repo")), None),
        "file_read" => {
            let r = if gs("ref").is_empty() { String::new() } else { format!("?ref={}", gs("ref")) };
            ("GET", format!("{base}/repos/{}/{}/contents/{}{r}", gs("owner"), gs("repo"), gs("path")), None)
        }
        "search_code" => ("GET", format!("{base}/search/code?q={}", gs("query")), None),
        _ => return Err(AppError::BadRequest(format!("Outil GitHub inconnu: {tool}"))),
    };

    let resp = match method {
        "POST" => client.post(&url).headers(headers).json(&body.unwrap_or(json!({}))).send().await,
        _ => client.get(&url).headers(headers).send().await,
    }.map_err(|e| AppError::BadGateway(format!("GitHub API: {e}")))?;

    resp.json::<Value>().await.map_err(|e| AppError::BadGateway(format!("GitHub JSON: {e}")))
}

// ── GitLab API adapter ──

async fn call_gitlab(
    client: &reqwest::Client,
    base: &str,
    tool: &str,
    args: &Value,
    headers: reqwest::header::HeaderMap,
) -> Result<Value, AppError> {
    let gs = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let api = format!("{base}/api/v4");

    let (method, url, body) = match tool {
        "projects_list" => {
            let group = gs("group");
            let url = if group.is_empty() {
                format!("{api}/projects?membership=true&per_page=100&order_by=updated_at")
            } else {
                format!("{api}/groups/{group}/projects?per_page=100&order_by=updated_at")
            };
            ("GET", url, None)
        }
        "project_get" => ("GET", format!("{api}/projects/{}", gs("project_id")), None),
        "issues_list" => {
            let state = if gs("state").is_empty() { "opened".to_string() } else { gs("state") };
            ("GET", format!("{api}/projects/{}/issues?state={state}", gs("project_id")), None)
        }
        "issue_create" => ("POST", format!("{api}/projects/{}/issues", gs("project_id")),
            Some(json!({"title": gs("title"), "description": gs("description")}))),
        "merge_requests_list" => {
            let state = if gs("state").is_empty() { "opened".to_string() } else { gs("state") };
            ("GET", format!("{api}/projects/{}/merge_requests?state={state}", gs("project_id")), None)
        }
        "mr_create" => ("POST", format!("{api}/projects/{}/merge_requests", gs("project_id")),
            Some(json!({"title": gs("title"), "source_branch": gs("source_branch"), "target_branch": gs("target_branch")}))),
        "pipelines_list" => ("GET", format!("{api}/projects/{}/pipelines?per_page=20", gs("project_id")), None),
        "file_read" => {
            let r = if gs("ref").is_empty() { "main".to_string() } else { gs("ref") };
            let path_encoded = gs("path").replace('/', "%2F");
            ("GET", format!("{api}/projects/{}/repository/files/{path_encoded}?ref={r}", gs("project_id")), None)
        }
        "search_code" => {
            let pid = gs("project_id");
            let url = if pid.is_empty() {
                format!("{api}/search?scope=blobs&search={}", gs("query"))
            } else {
                format!("{api}/projects/{pid}/search?scope=blobs&search={}", gs("query"))
            };
            ("GET", url, None)
        }
        _ => return Err(AppError::BadRequest(format!("Outil GitLab inconnu: {tool}"))),
    };

    let resp = match method {
        "POST" => client.post(&url).headers(headers).json(&body.unwrap_or(json!({}))).send().await,
        _ => client.get(&url).headers(headers).send().await,
    }.map_err(|e| AppError::BadGateway(format!("GitLab API: {e}")))?;

    resp.json::<Value>().await.map_err(|e| AppError::BadGateway(format!("GitLab JSON: {e}")))
}
