use chrono::Utc;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct DebugEntry {
    pub timestamp: f64,
    pub level: String,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct DebugBuffer {
    pub enabled: Arc<Mutex<bool>>,
    buffer: Arc<Mutex<VecDeque<DebugEntry>>>,
}

impl DebugBuffer {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(Mutex::new(false)),
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    pub fn set_enabled(&self, v: bool) {
        *self.enabled.lock().unwrap() = v;
    }

    pub fn log(&self, level: &str, category: &str, message: &str) {
        if !self.is_enabled() {
            return;
        }
        let entry = DebugEntry {
            timestamp: Utc::now().timestamp_millis() as f64 / 1000.0,
            level: level.to_string(),
            category: category.to_string(),
            message: message.to_string(),
            extra: None,
        };
        let mut buf = self.buffer.lock().unwrap();
        if buf.len() >= MAX_ENTRIES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }

    pub fn get_entries(
        &self,
        limit: usize,
        category: Option<&str>,
        level: Option<&str>,
        since: Option<f64>,
    ) -> Vec<DebugEntry> {
        let buf = self.buffer.lock().unwrap();
        buf.iter()
            .filter(|e| category.is_none_or(|c| e.category == c))
            .filter(|e| level.is_none_or(|l| e.level == l))
            .filter(|e| since.is_none_or(|s| e.timestamp >= s))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn total(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    pub fn clear(&self) {
        self.buffer.lock().unwrap().clear();
    }
}
