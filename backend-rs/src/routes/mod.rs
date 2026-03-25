pub mod health;
pub mod settings;
pub mod sessions;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health::health))
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
}
