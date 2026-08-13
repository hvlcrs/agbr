//! `agbr-mcp` — a minimal Model Context Protocol (MCP) server over stdio.
//!
//! This crate is transport/protocol only. It knows nothing about photo
//! editing; the `agbr` crate registers tools that invoke the control-plane
//! engine. Exposes a small typed surface: `tools/list`, `tools/call`, plus the
//! `initialize` handshake.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;

/// A boxed, sendable future.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// The handler signature for a tool: `arguments -> result`.
pub type ToolHandler = Box<dyn Fn(Value) -> BoxFuture<Result<Value, String>> + Send + Sync>;

/// A tool exposed over MCP.
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: ToolHandler,
}

impl Tool {
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, String>> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            handler: Box::new(move |args| Box::pin(handler(args))),
        }
    }
}

/// Identity advertised during the `initialize` handshake.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Error)]
pub enum McpError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid JSON-RPC message: {0}")]
    InvalidMessage(String),
}

#[derive(Debug, Deserialize)]
struct Request {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// Run the MCP server over stdio until stdin closes.
///
/// Each message is a single line of JSON (newline-delimited JSON-RPC 2.0).
pub async fn serve_stdio(info: ServerInfo, tools: Arc<Vec<Tool>>) -> Result<(), McpError> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = tokio::io::BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    while let Some(line) = reader.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(line) {
            Ok(req) => handle(&info, &tools, req).await,
            Err(e) => Some(error_response(
                Value::Null,
                -32700,
                format!("parse error: {e}"),
            )),
        };

        if let Some(resp) = response {
            let mut payload = serde_json::to_string(&resp).map_err(|e| {
                McpError::InvalidMessage(format!("failed to serialize response: {e}"))
            })?;
            payload.push('\n');
            writer.write_all(payload.as_bytes()).await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

async fn handle(info: &ServerInfo, tools: &[Tool], req: Request) -> Option<Value> {
    // Notifications have no id and expect no response.
    let is_notification = req.id.is_none();
    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": info.name, "version": info.version },
            });
            Some(result_response(id, result))
        }
        "notifications/initialized" | "notifications/cancelled" => None,
        "ping" => Some(result_response(id, json!({}))),
        "tools/list" => {
            let list: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                    })
                })
                .collect();
            Some(result_response(id, json!({ "tools": list })))
        }
        "tools/call" => {
            let params = req.params.unwrap_or_default();
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            match tools.iter().find(|t| t.name == name) {
                Some(tool) => {
                    let result = (tool.handler)(arguments).await;
                    Some(call_response(id, result))
                }
                None => Some(error_response(id, -32602, format!("unknown tool: {name}"))),
            }
        }
        other => {
            if is_notification {
                None
            } else {
                Some(error_response(
                    id,
                    -32601,
                    format!("method not found: {other}"),
                ))
            }
        }
    }
}

fn result_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn call_response(id: Value, result: Result<Value, String>) -> Value {
    match result {
        Ok(value) => result_response(
            id,
            json!({
                "content": [ { "type": "text", "text": value.to_string() } ],
                "isError": false,
            }),
        ),
        Err(err) => result_response(
            id,
            json!({
                "content": [ { "type": "text", "text": err } ],
                "isError": true,
            }),
        ),
    }
}
