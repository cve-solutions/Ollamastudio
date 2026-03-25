use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Session {
    pub id: i64,
    pub title: String,
    pub skill_id: Option<String>,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionCreate {
    pub title: Option<String>,
    pub model: String,
    pub skill_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionUpdate {
    pub title: Option<String>,
    pub skill_id: Option<String>,
}
