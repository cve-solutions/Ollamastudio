pub mod chat;
pub mod config_route;
pub mod debug;
pub mod documents;
pub mod files;
pub mod health;
pub mod mcp;
pub mod models;
pub mod sessions;
pub mod settings;
pub mod shell;
pub mod skills;
pub mod templates;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health::health))
        // Config
        .route("/api/config", get(config_route::get_config))
        // Settings
        .route("/api/settings/", get(settings::list_settings).put(settings::bulk_update))
        .route("/api/settings/ollama/test-connection", post(settings::test_connection))
        .route("/api/settings/category/{category}", get(settings::get_by_category))
        .route("/api/settings/{key}", get(settings::get_setting).put(settings::update_setting))
        // Sessions
        .route("/api/sessions/", get(sessions::list_sessions).post(sessions::create_session))
        .route(
            "/api/sessions/{id}",
            get(sessions::get_session)
                .patch(sessions::update_session)
                .delete(sessions::delete_session),
        )
        .route("/api/sessions/{id}/messages", get(sessions::get_messages))
        // Files
        .route("/api/files/tree", get(files::get_tree))
        .route("/api/files/read", get(files::read_file))
        .route("/api/files/write", post(files::write_file))
        .route("/api/files/delete", delete(files::delete_path))
        .route("/api/files/rename", post(files::rename_path))
        .route("/api/files/mkdir", post(files::mkdir))
        .route("/api/files/upload", post(files::upload_file))
        // Models
        .route("/api/models/", get(models::list_models))
        .route("/api/models/config", get(models::get_ollama_config))
        // MCP
        .route("/api/mcp/servers", get(mcp::list_servers).post(mcp::add_server))
        .route(
            "/api/mcp/servers/{id}",
            put(mcp::update_server).delete(mcp::delete_server),
        )
        .route("/api/mcp/servers/{id}/ping", get(mcp::ping_server))
        .route("/api/mcp/servers/{id}/tools", get(mcp::list_server_tools))
        .route("/api/mcp/servers/{id}/call", post(mcp::call_tool))
        // Debug
        .route("/api/debug/logs", get(debug::get_logs).delete(debug::clear_logs))
        .route("/api/debug/toggle", post(debug::toggle_debug))
        // Shell WebSocket
        .route("/api/shell/ws", get(shell::shell_ws))
        // Documents
        .route("/api/documents/", get(documents::list_documents))
        .route("/api/documents/upload", post(documents::upload_documents))
        .route("/api/documents/import-folder", post(documents::import_folder))
        .route("/api/documents/search", get(documents::search_documents))
        .route("/api/documents/{id}", delete(documents::delete_document))
        // Skills
        .route("/api/skills/", get(skills::list_skills).post(skills::create_skill))
        .route("/api/skills/import", post(skills::import_skills))
        .route(
            "/api/skills/{id}",
            get(skills::get_skill)
                .put(skills::update_skill)
                .delete(skills::delete_skill),
        )
        // Chat
        .route("/api/chat/stream", post(chat::chat_stream))
        // Templates
        .route("/api/templates/", get(templates::list_templates).post(templates::create_template))
        .route(
            "/api/templates/{id}",
            get(templates::get_template)
                .put(templates::update_template)
                .delete(templates::delete_template),
        )
}
