//! Minimal Model Context Protocol (MCP) server for the emulator.
//!
//! The server speaks newline-delimited JSON-RPC 2.0 over stdin/stdout and is
//! started with the `--mcp` command line flag. It shares the running emulator
//! through `Arc<EmuShared>`: most tools lock the shared emulator core, while
//! the run control (pause / step / run-until / breakpoints) goes through
//! `EmuShared::mcp` and is polled by the emulator thread's step loop.
//!
//! stdout is reserved for JSON-RPC frames; all diagnostics go to stderr.

mod tools;

use std::io::{BufRead, Write};
use std::sync::Arc;
use std::thread;

use serde_json::{Value, json};

use crate::EmuShared;

/// Protocol revision advertised in the `initialize` handshake.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Spawns the detached MCP server thread. The thread ends when stdin is closed
/// (the client exits) or on the first stdout write error.
pub fn spawn(shared: Arc<EmuShared>) -> std::io::Result<std::thread::JoinHandle<()>> {
    let join_handle = thread::Builder::new()
        .name("mcp".to_string())
        .spawn(move || run(shared))?;
    Ok(join_handle)
}

fn run(shared: Arc<EmuShared>) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(line) {
            Ok(request) => handle(&shared, &request),
            Err(err) => Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {err}"),
            )),
        };

        if let Some(response) = response {
            if writeln!(out, "{response}").is_err() {
                break;
            }
            let _ = out.flush();
        }
    }
}

/// Dispatches one JSON-RPC message. Returns `None` for notifications.
fn handle(shared: &EmuShared, request: &Value) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let id = request.get("id").cloned();
    // A message without an `id` is a notification and must not be answered.
    let is_notification = id.is_none();
    let reply_id = || id.clone().unwrap_or(Value::Null);

    match method {
        "initialize" => Some(ok_response(
            reply_id(),
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "emu6502", "version": env!("CARGO_PKG_VERSION") },
            }),
        )),

        // Notifications: no response, ever.
        "notifications/initialized" | "notifications/cancelled" => None,

        "ping" => Some(ok_response(reply_id(), json!({}))),

        "tools/list" => Some(ok_response(reply_id(), json!({ "tools": tools::list() }))),

        "tools/call" => {
            let params = request.get("params");
            let name = params
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            let (text, is_error) = match tools::call(shared, name, &args) {
                Ok(value) => (stringify(value), false),
                Err(message) => (message, true),
            };

            Some(ok_response(
                reply_id(),
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": is_error,
                }),
            ))
        }

        other => {
            if is_notification {
                None
            } else {
                Some(error_response(
                    reply_id(),
                    -32601,
                    &format!("method not found: {other}"),
                ))
            }
        }
    }
}

/// Renders a tool result as the text payload of a `tools/call` response.
fn stringify(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => serde_json::to_string_pretty(&other).unwrap_or_default(),
    }
}

fn ok_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}
