use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub category: String,
    pub label: String,
    pub description: String,
    pub value_type: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SettingUpdate {
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BulkSettingUpdate {
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}
