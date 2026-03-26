//! MCP proxy service — loads tools from enabled MCP servers
//! and routes tool calls to them at runtime.

use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

/// An MCP tool loaded from a remote server, ready to be injected into the LLM.
#[derive(Debug, Clone)]
pub struct McpTool {
    /// Full tool name for the LLM: "mcp__{server_id}__{tool_name}"
    pub qualified_name: String,
    /// Original tool name on the MCP server
    pub remote_name: String,
    /// Server ID (to route the call)
    pub server_id: String,
    /// Server URL
    pub server_url: String,
    /// Server type (http, github, gitlab)
    pub server_type: String,
    /// Auth token
    pub auth_token: String,
    /// Custom headers
    pub headers: HashMap<String, String>,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: Value,
}

/// Load all tools from all enabled MCP servers.
/// Returns (tool_schemas_for_llm, mcp_tool_registry).
pub async fn load_mcp_tools(
    config: &crate::config::Config,
) -> (Vec<Value>, Vec<McpTool>) {
    let servers = load_enabled_servers(config).await;
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    let mut schemas: Vec<Value> = Vec::new();
    let mut registry: Vec<McpTool> = Vec::new();

    for server in &servers {
        let tools = match fetch_server_tools(&client, server).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "MCP server '{}' ({}) — impossible de charger les outils: {e}",
                    server.name, server.url
                );
                continue;
            }
        };

        tracing::info!(
            "MCP '{}': {} outils chargés",
            server.name,
            tools.len()
        );

        for tool in tools {
            let tool_name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if tool_name.is_empty() {
                continue;
            }

            let qualified = format!("mcp__{}__{}", server.id, tool_name);
            let type_hint = match server.server_type.as_str() {
                "github" => "API GitHub distante",
                "gitlab" => "API GitLab distante",
                _ => "Serveur MCP système",
            };
            let description = format!(
                "[{}:{}] {}",
                type_hint,
                server.name,
                tool.get("description").and_then(|d| d.as_str()).unwrap_or("")
            );
            let input_schema = tool.get("inputSchema")
                .or(tool.get("input_schema"))
                .cloned()
                .unwrap_or(json!({"type": "object", "properties": {}, "required": []}));

            // Schema pour le LLM (format OpenAI function calling)
            schemas.push(json!({
                "type": "function",
                "function": {
                    "name": qualified,
                    "description": description,
                    "parameters": input_schema,
                }
            }));

            registry.push(McpTool {
                qualified_name: qualified,
                remote_name: tool_name.to_string(),
                server_id: server.id.clone(),
                server_url: server.url.clone(),
                server_type: server.server_type.clone(),
                auth_token: server.auth_token.clone(),
                headers: server.headers.clone(),
                description,
                input_schema,
            });
        }
    }

    (schemas, registry)
}

/// Execute an MCP tool call by finding it in the registry and routing to the server.
pub async fn execute_mcp_tool(
    tool: &McpTool,
    arguments: &Value,
) -> Value {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap_or_default();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/json".parse().unwrap(),
    );

    // Custom headers
    for (k, v) in &tool.headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            headers.insert(name, val);
        }
    }

    // Auth
    if !tool.auth_token.is_empty() {
        match tool.server_type.as_str() {
            "github" => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(
                    &format!("Bearer {}", tool.auth_token),
                ) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
            "gitlab" => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&tool.auth_token) {
                    headers.insert(
                        reqwest::header::HeaderName::from_static("private-token"),
                        val,
                    );
                }
            }
            _ => {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(
                    &format!("Bearer {}", tool.auth_token),
                ) {
                    headers.insert(reqwest::header::AUTHORIZATION, val);
                }
            }
        }
    }

    // Route according to server type
    match tool.server_type.as_str() {
        "github" => call_github_tool(&client, &tool.server_url, &tool.remote_name, arguments, headers).await,
        "gitlab" => call_gitlab_tool(&client, &tool.server_url, &tool.remote_name, arguments, headers).await,
        _ => call_mcp_jsonrpc(&client, &tool.server_url, &tool.remote_name, arguments, headers).await,
    }
}

// ── Internal helpers ──

struct ServerInfo {
    id: String,
    name: String,
    url: String,
    server_type: String,
    auth_token: String,
    headers: HashMap<String, String>,
}

async fn load_enabled_servers(config: &crate::config::Config) -> Vec<ServerInfo> {
    let path = config.data_dir.join("mcp_servers.yaml");
    if !path.exists() {
        return Vec::new();
    }
    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    #[derive(serde::Deserialize)]
    struct YamlServer {
        id: String,
        name: String,
        url: String,
        #[serde(default)]
        enabled: bool,
        #[serde(default, rename = "type")]
        server_type: String,
        #[serde(default)]
        auth_token: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    }

    let servers: Vec<YamlServer> = serde_yaml::from_str(&content).unwrap_or_default();
    servers
        .into_iter()
        .filter(|s| s.enabled)
        .map(|s| ServerInfo {
            id: s.id,
            name: s.name,
            url: s.url,
            server_type: if s.server_type.is_empty() { "http".into() } else { s.server_type },
            auth_token: s.auth_token,
            headers: s.headers,
        })
        .collect()
}

async fn fetch_server_tools(client: &Client, server: &ServerInfo) -> Result<Vec<Value>, String> {
    // GitHub/GitLab : built-in tools
    match server.server_type.as_str() {
        "github" => return Ok(github_tools()),
        "gitlab" => return Ok(gitlab_tools()),
        _ => {}
    }

    // MCP JSON-RPC: tools/list
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
    if !server.auth_token.is_empty() {
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", server.auth_token)) {
            headers.insert(reqwest::header::AUTHORIZATION, val);
        }
    }

    let resp = client
        .post(&server.url)
        .headers(headers)
        .json(&json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1}))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;

    let data: Value = resp.json().await.map_err(|e| format!("Invalid JSON: {e}"))?;

    let tools = data
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(tools)
}

async fn call_mcp_jsonrpc(
    client: &Client, url: &str, tool_name: &str, arguments: &Value,
    headers: reqwest::header::HeaderMap,
) -> Value {
    let resp = client
        .post(url)
        .headers(headers)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": arguments},
            "id": 1
        }))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let data: Value = r.json().await.unwrap_or(json!({"error": "Invalid response"}));
            // Extract text from MCP response
            if let Some(content) = data.get("result").and_then(|r| r.get("content")).and_then(|c| c.as_array()) {
                let text: String = content.iter()
                    .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                json!({"result": text})
            } else if let Some(err) = data.get("error") {
                json!({"error": err})
            } else {
                data
            }
        }
        Err(e) => json!({"error": format!("MCP call failed: {e}")}),
    }
}

async fn call_github_tool(
    client: &Client, base: &str, tool: &str, args: &Value,
    headers: reqwest::header::HeaderMap,
) -> Value {
    let gs = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let base = base.trim_end_matches('/');

    let (method, url, body): (&str, String, Option<Value>) = match tool {
        "repos_list" => {
            let org = gs("org");
            let url = if org.is_empty() { format!("{base}/user/repos?per_page=100") } else { format!("{base}/orgs/{org}/repos?per_page=100") };
            ("GET", url, None)
        }
        "repo_get" => ("GET", format!("{base}/repos/{}/{}", gs("owner"), gs("repo")), None),
        "issues_list" => ("GET", format!("{base}/repos/{}/{}/issues?state={}", gs("owner"), gs("repo"), if gs("state").is_empty() { "open".into() } else { gs("state") }), None),
        "issue_create" => ("POST", format!("{base}/repos/{}/{}/issues", gs("owner"), gs("repo")), Some(json!({"title": gs("title"), "body": gs("body")}))),
        "pulls_list" => ("GET", format!("{base}/repos/{}/{}/pulls?state={}", gs("owner"), gs("repo"), if gs("state").is_empty() { "open".into() } else { gs("state") }), None),
        "pr_create" => ("POST", format!("{base}/repos/{}/{}/pulls", gs("owner"), gs("repo")), Some(json!({"title": gs("title"), "body": gs("body"), "head": gs("head"), "base": gs("base")}))),
        "actions_list" => ("GET", format!("{base}/repos/{}/{}/actions/runs?per_page=20", gs("owner"), gs("repo")), None),
        "releases_list" => ("GET", format!("{base}/repos/{}/{}/releases", gs("owner"), gs("repo")), None),
        "file_read" => ("GET", format!("{base}/repos/{}/{}/contents/{}", gs("owner"), gs("repo"), gs("path")), None),
        "search_code" => ("GET", format!("{base}/search/code?q={}", gs("query")), None),
        _ => return json!({"error": format!("Unknown GitHub tool: {tool}")}),
    };

    let resp = match method {
        "POST" => client.post(&url).headers(headers).json(&body.unwrap_or(json!({}))).send().await,
        _ => client.get(&url).headers(headers).send().await,
    };

    match resp {
        Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "Invalid response"})),
        Err(e) => json!({"error": format!("GitHub API: {e}")}),
    }
}

async fn call_gitlab_tool(
    client: &Client, base: &str, tool: &str, args: &Value,
    headers: reqwest::header::HeaderMap,
) -> Value {
    let gs = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let api = format!("{}/api/v4", base.trim_end_matches('/'));

    let (method, url, body): (&str, String, Option<Value>) = match tool {
        "projects_list" => {
            let group = gs("group");
            let url = if group.is_empty() { format!("{api}/projects?membership=true&per_page=100") } else { format!("{api}/groups/{group}/projects?per_page=100") };
            ("GET", url, None)
        }
        "project_get" => ("GET", format!("{api}/projects/{}", gs("project_id")), None),
        "issues_list" => ("GET", format!("{api}/projects/{}/issues?state={}", gs("project_id"), if gs("state").is_empty() { "opened".into() } else { gs("state") }), None),
        "issue_create" => ("POST", format!("{api}/projects/{}/issues", gs("project_id")), Some(json!({"title": gs("title"), "description": gs("description")}))),
        "merge_requests_list" => ("GET", format!("{api}/projects/{}/merge_requests?state={}", gs("project_id"), if gs("state").is_empty() { "opened".into() } else { gs("state") }), None),
        "mr_create" => ("POST", format!("{api}/projects/{}/merge_requests", gs("project_id")), Some(json!({"title": gs("title"), "source_branch": gs("source_branch"), "target_branch": gs("target_branch")}))),
        "pipelines_list" => ("GET", format!("{api}/projects/{}/pipelines?per_page=20", gs("project_id")), None),
        "file_read" => ("GET", format!("{api}/projects/{}/repository/files/{}?ref={}", gs("project_id"), gs("path").replace('/', "%2F"), if gs("ref").is_empty() { "main".into() } else { gs("ref") }), None),
        "search_code" => ("GET", format!("{api}/search?scope=blobs&search={}", gs("query")), None),
        _ => return json!({"error": format!("Unknown GitLab tool: {tool}")}),
    };

    let resp = match method {
        "POST" => client.post(&url).headers(headers).json(&body.unwrap_or(json!({}))).send().await,
        _ => client.get(&url).headers(headers).send().await,
    };

    match resp {
        Ok(r) => r.json::<Value>().await.unwrap_or(json!({"error": "Invalid response"})),
        Err(e) => json!({"error": format!("GitLab API: {e}")}),
    }
}

fn github_tools() -> Vec<Value> {
    vec![
        json!({"name": "repos_list", "description": "Lister les repositories GitHub", "inputSchema": {"type":"object","properties":{"org":{"type":"string"}},"required":[]}}),
        json!({"name": "repo_get", "description": "Détails d'un repository", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"}},"required":["owner","repo"]}}),
        json!({"name": "issues_list", "description": "Lister les issues", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"},"state":{"type":"string"}},"required":["owner","repo"]}}),
        json!({"name": "issue_create", "description": "Créer une issue", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}},"required":["owner","repo","title"]}}),
        json!({"name": "pulls_list", "description": "Lister les pull requests", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"},"state":{"type":"string"}},"required":["owner","repo"]}}),
        json!({"name": "pr_create", "description": "Créer une pull request", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"},"head":{"type":"string"},"base":{"type":"string"}},"required":["owner","repo","title","head","base"]}}),
        json!({"name": "actions_list", "description": "Lister les workflow runs GitHub Actions", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"}},"required":["owner","repo"]}}),
        json!({"name": "releases_list", "description": "Lister les releases", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"}},"required":["owner","repo"]}}),
        json!({"name": "file_read", "description": "Lire un fichier du repo", "inputSchema": {"type":"object","properties":{"owner":{"type":"string"},"repo":{"type":"string"},"path":{"type":"string"},"ref":{"type":"string"}},"required":["owner","repo","path"]}}),
        json!({"name": "search_code", "description": "Rechercher du code", "inputSchema": {"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}),
    ]
}

fn gitlab_tools() -> Vec<Value> {
    vec![
        json!({"name": "projects_list", "description": "Lister les projets GitLab", "inputSchema": {"type":"object","properties":{"group":{"type":"string"}},"required":[]}}),
        json!({"name": "project_get", "description": "Détails d'un projet", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name": "issues_list", "description": "Lister les issues", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string"}},"required":["project_id"]}}),
        json!({"name": "issue_create", "description": "Créer une issue", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"description":{"type":"string"}},"required":["project_id","title"]}}),
        json!({"name": "merge_requests_list", "description": "Lister les merge requests", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string"}},"required":["project_id"]}}),
        json!({"name": "mr_create", "description": "Créer une merge request", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"source_branch":{"type":"string"},"target_branch":{"type":"string"}},"required":["project_id","title","source_branch","target_branch"]}}),
        json!({"name": "pipelines_list", "description": "Lister les pipelines CI", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name": "file_read", "description": "Lire un fichier du projet", "inputSchema": {"type":"object","properties":{"project_id":{"type":"string"},"path":{"type":"string"},"ref":{"type":"string"}},"required":["project_id","path"]}}),
        json!({"name": "search_code", "description": "Rechercher du code", "inputSchema": {"type":"object","properties":{"query":{"type":"string"},"project_id":{"type":"string"}},"required":["query"]}}),
    ]
}
