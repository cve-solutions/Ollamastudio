//! Agentic loop — streams tool_call -> execution -> response cycles as SSE events.

use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::pin::Pin;

use crate::config::Config;
use crate::services::mcp_proxy::{self, McpTool};
use crate::services::ollama::{get_setting_i64, OllamaClient};

use super::tools::{execute_tool, tool_schemas};

const MAX_ITERATIONS: usize = 20;

/// A typed event emitted by the agentic loop.
/// The chat route converts these into axum SSE Events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    #[serde(rename = "text")]
    Text { content: String },

    #[serde(rename = "tool_start")]
    ToolStart {
        name: String,
        args: Value,
        call_id: String,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        result: Value,
        call_id: String,
    },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "done")]
    Done { iterations: usize },
}

/// Run the agentic loop, yielding `AgentEvent` items.
///
/// The caller is responsible for converting these to SSE Events and
/// tracking state for DB persistence.
pub fn agentic_loop(
    client: OllamaClient,
    model: String,
    messages: Vec<Value>,
    system: Option<String>,
    enabled_tools: Option<Vec<String>>,
    temperature: f64,
    max_tokens: i64,
    config: Config,
    pool: SqlitePool,
) -> Pin<Box<dyn Stream<Item = AgentEvent> + Send>> {
    let stream = async_stream::stream! {
        // ── 1. Charge les outils internes ──
        let all_schemas = tool_schemas();
        let mut tools: Vec<Value> = match &enabled_tools {
            Some(names) => all_schemas
                .into_iter()
                .filter(|t| {
                    t.get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|n| names.contains(&n.to_string()))
                        .unwrap_or(false)
                })
                .collect(),
            None => all_schemas,
        };

        // ── 2. Charge les outils MCP des serveurs activés ──
        let (mcp_schemas, mcp_registry) = mcp_proxy::load_mcp_tools(&config).await;
        if !mcp_schemas.is_empty() {
            tracing::info!("{} outils MCP injectés dans la boucle agentique", mcp_schemas.len());
            tools.extend(mcp_schemas);
        }

        let timeout_secs = get_setting_i64(&pool, "ollama_timeout", 300).await as u64;

        let mut history = messages;
        let mut iterations: usize = 0;

        loop {
            if iterations >= MAX_ITERATIONS {
                yield AgentEvent::Error {
                    message: format!("Iteration limit reached ({MAX_ITERATIONS})"),
                };
                break;
            }
            iterations += 1;
            tracing::debug!("Agent iteration #{iterations}");

            let tools_ref = if tools.is_empty() { None } else { Some(tools.as_slice()) };

            // Accumulate text and tool calls from the streaming response.
            let mut text_buffer = String::new();
            let mut tc_buffer: Vec<ToolCallAccum> = Vec::new();

            {
                use futures::StreamExt;
                let mut chunk_stream = client.stream_chat(
                    &model,
                    &history,
                    tools_ref,
                    system.as_deref(),
                    temperature,
                    max_tokens,
                    timeout_secs,
                );

                while let Some(chunk_result) = chunk_stream.next().await {
                    let raw = match chunk_result {
                        Ok(r) => r,
                        Err(e) => {
                            yield AgentEvent::Error { message: e };
                            break;
                        }
                    };

                    let chunk: Value = match serde_json::from_str(&raw) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                        for choice in choices {
                            let delta = match choice.get("delta") {
                                Some(d) => d,
                                None => continue,
                            };

                            // Text content.
                            if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                                if !text.is_empty() {
                                    text_buffer.push_str(text);
                                    yield AgentEvent::Text {
                                        content: text.to_string(),
                                    };
                                }
                            }

                            // Tool calls (streamed incrementally).
                            if let Some(tcs) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                for tc in tcs {
                                    let idx = tc
                                        .get("index")
                                        .and_then(|i| i.as_u64())
                                        .unwrap_or(tc_buffer.len() as u64)
                                        as usize;

                                    while tc_buffer.len() <= idx {
                                        tc_buffer.push(ToolCallAccum::default());
                                    }
                                    let entry = &mut tc_buffer[idx];

                                    if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                        if entry.id.is_empty() {
                                            entry.id = id.to_string();
                                        }
                                    }
                                    if let Some(func) = tc.get("function") {
                                        if let Some(name) =
                                            func.get("name").and_then(|n| n.as_str())
                                        {
                                            if entry.name.is_empty() {
                                                entry.name = name.to_string();
                                            }
                                        }
                                        if let Some(args_frag) =
                                            func.get("arguments").and_then(|a| a.as_str())
                                        {
                                            entry.arguments.push_str(args_frag);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // -- The assistant finished streaming for this iteration --

            if tc_buffer.is_empty() {
                // Pure text response — done.
                if !text_buffer.is_empty() {
                    history.push(json!({"role": "assistant", "content": text_buffer}));
                }
                break;
            }

            // Build finalized tool calls with stable IDs.
            let finalized: Vec<(String, String, String)> = tc_buffer
                .iter()
                .enumerate()
                .filter(|(_, tc)| !tc.name.is_empty())
                .map(|(i, tc)| {
                    let id = if tc.id.is_empty() {
                        format!("call_{i}")
                    } else {
                        tc.id.clone()
                    };
                    (id, tc.name.clone(), tc.arguments.clone())
                })
                .collect();

            // Append assistant message (with tool_calls) to history.
            let tc_values: Vec<Value> = finalized
                .iter()
                .map(|(id, name, arguments)| {
                    json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": arguments,
                        }
                    })
                })
                .collect();

            history.push(json!({
                "role": "assistant",
                "content": text_buffer,
                "tool_calls": tc_values,
            }));

            // Execute each tool and feed results back.
            for (call_id, name, raw_args) in &finalized {
                let args: Value = serde_json::from_str(raw_args).unwrap_or(json!({}));

                yield AgentEvent::ToolStart {
                    name: name.clone(),
                    args: args.clone(),
                    call_id: call_id.clone(),
                };

                // Route : outil MCP ou outil interne ?
                let result = if name.starts_with("mcp__") {
                    // Trouve l'outil MCP dans le registre
                    if let Some(mcp_tool) = mcp_registry.iter().find(|t| t.qualified_name == *name) {
                        mcp_proxy::execute_mcp_tool(mcp_tool, &args).await
                    } else {
                        json!({"error": format!("MCP tool not found: {name}")})
                    }
                } else {
                    execute_tool(name, args, &config, &pool).await
                };

                yield AgentEvent::ToolResult {
                    name: name.clone(),
                    result: result.clone(),
                    call_id: call_id.clone(),
                };

                history.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "name": name,
                    "content": serde_json::to_string(&result).unwrap_or_default(),
                }));
            }
        }

        yield AgentEvent::Done { iterations };
    };

    Box::pin(stream)
}

// ------------------------------------------------------------------
// Internal accumulator for streamed tool calls
// ------------------------------------------------------------------

#[derive(Default, Clone)]
struct ToolCallAccum {
    id: String,
    name: String,
    arguments: String,
}
