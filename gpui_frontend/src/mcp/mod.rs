//! Minimal Model Context Protocol (MCP) server for the emulator.
//!
//! The server speaks newline-delimited JSON-RPC 2.0 over stdio (`--mcp`, the
//! framing an MCP client expects when it spawns us). It shares its dispatcher
//! and the running emulator through `Arc<EmuShared>`: most tools lock the
//! shared emulator core, while the run control (pause / step / run-until /
//! breakpoints) goes through `EmuShared::mcp` and is polled by the emulator
//! thread's step loop.
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

/// Largest request frame accepted. Tool calls are tiny
/// (`mem_write` payloads are the biggest), so this is only a guard against a
/// buggy or hostile peer streaming an unbounded line into memory. A larger
/// frame is drained (so framing stays in sync) and answered with a parse error.
const MAX_FRAME_BYTES: usize = 1 << 20; // 1 MiB

/// Spawns the detached stdio MCP server thread. The thread ends when stdin is
/// closed (the client exits) or on the first stdout write error.
pub fn spawn(shared: Arc<EmuShared>) -> std::io::Result<std::thread::JoinHandle<()>> {
    let join_handle = thread::Builder::new()
        .name("mcp".to_string())
        .spawn(move || run(shared))?;
    Ok(join_handle)
}

fn run(shared: Arc<EmuShared>) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let _ = serve(stdin.lock(), stdout.lock(), |request| {
        handle(&shared, request)
    });
}

/// Serves one client: reads frames from `reader`, answers through `out` using
/// `dispatch`, and returns at EOF (the client closed stdin) or on the first
/// write/read error.
///
/// Generic over the stream pair on purpose, which keeps the framing testable
/// with in-memory buffers and without an emulator at all.
fn serve<R, W, F>(
    mut reader: R,
    mut out: W,
    mut dispatch: F,
) -> std::io::Result<()>
where
    R: BufRead,
    W: Write,
    F: FnMut(&Value) -> Option<Value>,
{
    let mut frame = Vec::new();

    loop {
        let Some(overflow) = read_frame(&mut reader, &mut frame)? else {
            return Ok(()); // EOF: the client is gone
        };

        // The frame is not necessarily valid UTF-8; lossy decoding just turns
        // it into a parse error.
        let text = String::from_utf8_lossy(&frame);
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let response = if overflow {
            Some(error_response(
                Value::Null,
                -32700,
                &format!("request exceeds {MAX_FRAME_BYTES} bytes"),
            ))
        } else {
            match serde_json::from_str::<Value>(text) {
                Ok(request) => dispatch(&request),
                Err(err) => Some(error_response(
                    Value::Null,
                    -32700,
                    &format!("parse error: {err}"),
                )),
            }
        };

        if let Some(response) = response {
            writeln!(out, "{response}")?;
            out.flush()?;
        }
    }
}

/// Reads one newline-terminated frame into `frame`, discarding anything past
/// `MAX_FRAME_BYTES` so the cap also bounds the memory a peer can force us to
/// allocate. Returns `Ok(None)` at a clean EOF (nothing buffered), otherwise
/// `Ok(Some(overflowed))`.
fn read_frame<R: BufRead>(reader: &mut R, frame: &mut Vec<u8>) -> std::io::Result<Option<bool>> {
    frame.clear();
    let mut overflow = false;

    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            // EOF. A frame cut short mid-line is still handed back so the
            // caller reports it instead of silently dropping it.
            return Ok((!frame.is_empty() || overflow).then_some(overflow));
        }

        let newline = chunk.iter().position(|&byte| byte == b'\n');
        let consume_up_to = newline.map_or(chunk.len(), |pos| pos + 1);

        if !overflow {
            // Only the payload counts against the cap; the terminator does not.
            let payload = newline.unwrap_or(consume_up_to);
            let room = MAX_FRAME_BYTES - frame.len();
            let take = payload.min(room);
            frame.extend_from_slice(&chunk[..take]);
            overflow = take < payload;
        }

        reader.consume(consume_up_to);
        if newline.is_some() {
            return Ok(Some(overflow));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Dispatcher standing in for `handle` so the framing can be exercised
    /// without a `EmuShared` (and therefore without a running emulator).
    fn fake_dispatch(request: &Value) -> Option<Value> {
        match request.get("method").and_then(Value::as_str) {
            Some("ping") => Some(ok_response(
                request.get("id").cloned().unwrap_or(Value::Null),
                json!({}),
            )),
            _ => None, // notifications, unknown methods
        }
    }

    /// Serves a complete request stream from memory and returns everything
    /// `serve` wrote back. The reader starts at EOF, so the call runs to
    /// completion without any client.
    fn serve_buffered(requests: impl AsRef<[u8]>) -> Vec<u8> {
        let mut out = Vec::new();
        serve(Cursor::new(requests.as_ref()), &mut out, fake_dispatch).unwrap();
        out
    }

    /// Parses the newline-delimited responses `serve` produced.
    fn responses(out: &[u8]) -> Vec<Value> {
        String::from_utf8_lossy(out)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    #[test]
    fn frames_requests_answers_and_ignores_notifications() {
        let mut requests = [
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "  ", // blank frames are skipped
            r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
        ]
        .join("\n");
        requests.push('\n');

        let replies = responses(&serve_buffered(requests));
        // A notification must never be answered, so the only reply is the one
        // to the `ping`.
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["jsonrpc"], "2.0");
        assert_eq!(replies[0]["id"], 1);
        assert_eq!(replies[0]["result"], json!({}));
        assert!(replies[0].get("error").is_none());
    }

    #[test]
    fn oversized_frame_is_reported_and_framing_survives() {
        let mut requests = vec![b'a'; MAX_FRAME_BYTES + 64];
        requests.push(b'\n');
        requests.extend_from_slice(br#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#);
        requests.push(b'\n');

        let replies = responses(&serve_buffered(requests));
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], -32700);

        // The oversized frame was drained up to its newline, so the next
        // request is still framed correctly.
        assert_eq!(replies[1]["id"], 7);
        assert_eq!(replies[1]["result"], json!({}));
    }
}
