//! Agent MCP — Serveur MCP (Model Context Protocol) pour administration système.
//!
//! Expose des outils JSON-RPC pour interagir avec la machine :
//! fichiers, packages, services, cron, réseau, processus, utilisateurs...
//!
//! Tourne en tant que service systemd sur le port 9100.

mod tools;

use std::env;
use std::net::SocketAddr;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

// ═══════════════════════════════════════════════════════════════════
// MCP JSON-RPC types
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn error(id: Value, code: i32, message: String) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(JsonRpcError { code, message, data: None }) }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Routes
// ═══════════════════════════════════════════════════════════════════

/// GET / — MCP server info
async fn server_info() -> Json<Value> {
    Json(json!({
        "name": "agent-mcp",
        "version": env::var("APP_VERSION").unwrap_or_else(|_| "0.0.31".into()),
        "description": "OllamaStudio Agent MCP — Administration système complète",
        "protocol": "mcp-jsonrpc",
        "tools_count": tools::TOOL_LIST.len(),
    }))
}

/// POST / — MCP JSON-RPC endpoint
async fn jsonrpc_handler(Json(req): Json<JsonRpcRequest>) -> Json<JsonRpcResponse> {
    let id = req.id.unwrap_or(Value::Null);

    match req.method.as_str() {
        // MCP discovery
        "tools/list" => {
            let tool_defs: Vec<Value> = tools::TOOL_LIST
                .iter()
                .map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.schema(),
                }))
                .collect();
            Json(JsonRpcResponse::success(id, json!({ "tools": tool_defs })))
        }

        // MCP tool call
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            tracing::info!("Tool call: {tool_name}");

            match tools::execute(tool_name, &arguments).await {
                Ok(result) => Json(JsonRpcResponse::success(id, json!({
                    "content": [{ "type": "text", "text": result }]
                }))),
                Err(e) => Json(JsonRpcResponse::error(id, -32000, e)),
            }
        }

        // Ping
        "ping" => Json(JsonRpcResponse::success(id, json!("pong"))),

        _ => Json(JsonRpcResponse::error(id, -32601, format!("Méthode inconnue: {}", req.method))),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_env("LOG_LEVEL")
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let version = env::var("APP_VERSION").unwrap_or_else(|_| "0.0.31".into());
    tracing::info!("Agent MCP v{version} — démarrage");
    tracing::info!("{} outils système disponibles", tools::TOOL_LIST.len());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(server_info).post(jsonrpc_handler))
        .layer(cors);

    let port: u16 = env::var("MCP_PORT")
        .unwrap_or_else(|_| "9100".into())
        .parse()
        .unwrap_or(9100);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("Agent MCP prêt — http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
