use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_results: Option<String>,
    pub tokens_used: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: i64,
    pub content: String,
    pub model: String,
    pub ollama_base_url: Option<String>,
    pub ollama_api_mode: Option<String>,
    pub enabled_tools: Option<Vec<String>>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
}

fn default_temperature() -> f64 { 0.7 }
fn default_max_tokens() -> i64 { 4096 }
