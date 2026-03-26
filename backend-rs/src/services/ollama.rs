//! Ollama HTTP client — supports OpenAI (/v1/chat/completions) and Anthropic (/v1/messages) modes.

use futures::stream::Stream;
use reqwest::Client;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Resolved configuration for an Ollama connection.
#[derive(Debug, Clone)]
pub struct OllamaClient {
    pub base_url: String,
    pub api_mode: String,
    http: Client,
}

impl OllamaClient {
    /// Create a new client with explicit values.
    pub fn new(base_url: String, api_mode: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_mode,
            http: Client::new(),
        }
    }

    /// Resolve settings from DB, using overrides when provided.
    pub async fn from_settings(
        pool: &SqlitePool,
        base_url_override: Option<&str>,
        api_mode_override: Option<&str>,
    ) -> Self {
        let base_url = match base_url_override {
            Some(u) if !u.is_empty() => u.to_string(),
            _ => get_setting(pool, "ollama_base_url", "http://localhost:11434").await,
        };
        let api_mode = match api_mode_override {
            Some(m) if !m.is_empty() => m.to_string(),
            _ => get_setting(pool, "ollama_api_mode", "openai").await,
        };
        Self::new(base_url, api_mode)
    }

    // ------------------------------------------------------------------
    // List models
    // ------------------------------------------------------------------

    /// Fetch available models from Ollama's /api/tags endpoint.
    pub async fn list_models(&self) -> Result<Vec<Value>, reqwest::Error> {
        let url = format!("{}/api/tags", self.base_url);
        let resp: Value = self
            .http
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let models = resp
            .get("models")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(models)
    }

    // ------------------------------------------------------------------
    // Streaming chat — unified interface
    // ------------------------------------------------------------------

    /// Stream chat completions. Returns a stream of raw JSON strings in
    /// **OpenAI delta format** regardless of the underlying api_mode.
    /// Each yielded string is one parsed SSE `data:` payload (not `[DONE]`).
    pub fn stream_chat(
        &self,
        model: &str,
        messages: &[Value],
        tools: Option<&[Value]>,
        system: Option<&str>,
        temperature: f64,
        max_tokens: i64,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Stream<Item = Result<String, String>> + Send>> {
        if self.api_mode == "anthropic" {
            self.stream_anthropic(model, messages, tools, system, temperature, max_tokens, timeout_secs)
        } else {
            self.stream_openai(model, messages, tools, system, temperature, max_tokens, timeout_secs)
        }
    }

    // ------------------------------------------------------------------
    // OpenAI mode
    // ------------------------------------------------------------------

    fn stream_openai(
        &self,
        model: &str,
        messages: &[Value],
        tools: Option<&[Value]>,
        system: Option<&str>,
        temperature: f64,
        max_tokens: i64,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Stream<Item = Result<String, String>> + Send>> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        // Build messages list, prepending system if provided.
        let mut msgs: Vec<Value> = Vec::with_capacity(messages.len() + 1);
        if let Some(sys) = system {
            msgs.push(json!({"role": "system", "content": sys}));
        }
        msgs.extend_from_slice(messages);

        let mut payload = json!({
            "model": model,
            "messages": msgs,
            "stream": true,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
        if let Some(t) = tools {
            if !t.is_empty() {
                payload["tools"] = Value::Array(t.to_vec());
            }
        }

        let client = self.http.clone();
        let (tx, rx) = mpsc::channel::<Result<String, String>>(64);

        tokio::spawn(async move {
            let result = client
                .post(&url)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .send()
                .await;

            let resp = match result {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Ollama HTTP error: {e}"))).await;
                        return;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(format!("Ollama connection error: {e}"))).await;
                    return;
                }
            };

            // Read the SSE byte-stream line by line.
            use futures::StreamExt;
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Stream read error: {e}"))).await;
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Process complete lines.
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            return;
                        }
                        if tx.send(Ok(data.to_string())).await.is_err() {
                            return; // receiver dropped
                        }
                    }
                }
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }

    // ------------------------------------------------------------------
    // Anthropic mode
    // ------------------------------------------------------------------

    fn stream_anthropic(
        &self,
        model: &str,
        messages: &[Value],
        tools: Option<&[Value]>,
        system: Option<&str>,
        temperature: f64,
        max_tokens: i64,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Stream<Item = Result<String, String>> + Send>> {
        let url = format!("{}/v1/messages", self.base_url);

        let mut payload = json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "max_tokens": max_tokens,
        });
        if let Some(sys) = system {
            payload["system"] = Value::String(sys.to_string());
        }
        if let Some(t) = tools {
            if !t.is_empty() {
                let anthropic_tools: Vec<Value> = t.iter().map(openai_tool_to_anthropic).collect();
                payload["tools"] = Value::Array(anthropic_tools);
            }
        }

        let client = self.http.clone();
        let (tx, rx) = mpsc::channel::<Result<String, String>>(64);

        tokio::spawn(async move {
            let result = client
                .post(&url)
                .header("anthropic-version", "2023-06-01")
                .header("x-api-key", "ollama")
                .header("content-type", "application/json")
                .json(&payload)
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .send()
                .await;

            let resp = match result {
                Ok(r) => match r.error_for_status() {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Ollama HTTP error: {e}"))).await;
                        return;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(format!("Ollama connection error: {e}"))).await;
                    return;
                }
            };

            use futures::StreamExt;
            let mut byte_stream = resp.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Stream read error: {e}"))).await;
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if let Some(normalized) = anthropic_event_to_openai(data) {
                            if tx.send(Ok(normalized)).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });

        Box::pin(ReceiverStream::new(rx))
    }
}

// ------------------------------------------------------------------
// Helpers: Anthropic <-> OpenAI conversion
// ------------------------------------------------------------------

/// Convert an OpenAI function-calling tool schema to Anthropic format.
fn openai_tool_to_anthropic(tool: &Value) -> Value {
    let func = tool.get("function").unwrap_or(tool);
    json!({
        "name": func.get("name").and_then(|v| v.as_str()).unwrap_or(""),
        "description": func.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        "input_schema": func.get("parameters").cloned().unwrap_or(json!({"type": "object", "properties": {}})),
    })
}

/// Convert a single Anthropic SSE event JSON string to OpenAI delta format.
/// Returns `None` for events we don't need to forward.
fn anthropic_event_to_openai(data: &str) -> Option<String> {
    let evt: Value = serde_json::from_str(data).ok()?;
    let event_type = evt.get("type")?.as_str()?;

    match event_type {
        "content_block_delta" => {
            let delta = evt.get("delta")?;
            let text = delta.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                // Could be partial_json for tool input
                let partial = delta.get("partial_json").and_then(|v| v.as_str()).unwrap_or("");
                if !partial.is_empty() {
                    // Forward as tool call arguments fragment
                    let idx = evt.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                    return Some(
                        json!({
                            "choices": [{
                                "delta": {
                                    "tool_calls": [{
                                        "index": idx,
                                        "function": { "arguments": partial }
                                    }]
                                },
                                "finish_reason": null
                            }]
                        })
                        .to_string(),
                    );
                }
                return None;
            }
            Some(
                json!({
                    "choices": [{"delta": {"content": text}, "finish_reason": null}]
                })
                .to_string(),
            )
        }
        "content_block_start" => {
            let block = evt.get("content_block")?;
            if block.get("type")?.as_str()? == "tool_use" {
                Some(
                    json!({
                        "choices": [{
                            "delta": {
                                "tool_calls": [{
                                    "id": block.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                                    "type": "function",
                                    "function": {
                                        "name": block.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                                        "arguments": ""
                                    }
                                }]
                            },
                            "finish_reason": null
                        }]
                    })
                    .to_string(),
                )
            } else {
                None
            }
        }
        "message_stop" => Some(
            json!({
                "choices": [{"delta": {}, "finish_reason": "stop"}]
            })
            .to_string(),
        ),
        _ => None,
    }
}

// ------------------------------------------------------------------
// DB helper for settings
// ------------------------------------------------------------------

/// Read a single setting value from the database, returning `default` on miss.
pub async fn get_setting(pool: &SqlitePool, key: &str, default: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

/// Read a setting as i64.
pub async fn get_setting_i64(pool: &SqlitePool, key: &str, default: i64) -> i64 {
    get_setting(pool, key, &default.to_string())
        .await
        .parse()
        .unwrap_or(default)
}
