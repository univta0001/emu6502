//! Minimal Model Context Protocol (MCP) server for the emulator.
//!
//! The server speaks newline-delimited JSON-RPC 2.0 and has two transports:
//! stdio (`--mcp`, the framing an MCP client expects when it spawns us) and a
//! plain TCP socket (`--mcp-tcp ADDR`, for clients that drive the emulator
//! over the network). Both share the same dispatcher and the running emulator
//! through `Arc<EmuShared>`: most tools lock `cpu` briefly, while work that
//! needs the SDL main thread (a screenshot) is deferred through `EmuShared::mcp`.
//!
//! On stdio, stdout is reserved for JSON-RPC frames; all diagnostics go to
//! stderr. On TCP, stderr carries the same diagnostics plus the bound address.

mod tools;

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use serde_json::{Value, json};

use crate::EmuShared;

/// Protocol revision advertised in the `initialize` handshake.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Largest request frame accepted on either transport. Tool calls are tiny
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

/// Binds the TCP listener for the MCP server. Split from [`spawn_tcp`] so the
/// caller can bind (and report a bad address or an already used port) before
/// it creates the window or the emulator, then hand the listener over.
pub fn bind_tcp(addr: &str) -> std::io::Result<(SocketAddr, TcpListener)> {
    let listener = TcpListener::bind(addr)?;
    let bound = listener.local_addr()?;
    eprintln!("MCP server listening on tcp://{bound}");
    Ok((bound, listener))
}

/// Spawns the detached TCP MCP server thread on an already bound listener.
///
/// Unlike stdio the listener outlives a single client: connections are served
/// one at a time and a disconnect returns to `accept`, so a client can
/// reconnect. Only the process exit (or the `quit` tool) stops the server.
pub fn spawn_tcp(
    shared: Arc<EmuShared>,
    listener: TcpListener,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let join_handle = thread::Builder::new()
        .name("mcp-tcp".to_string())
        .spawn(move || run_tcp(shared, listener))?;
    Ok(join_handle)
}

fn run_tcp(shared: Arc<EmuShared>, listener: TcpListener) {
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                // Interactive request/response: small frames should not sit in
                // the Nagle buffer waiting for a coalescing window.
                let _ = stream.set_nodelay(true);
                if let Err(err) = serve_conn(&shared, stream) {
                    eprintln!("MCP tcp connection ended: {err}");
                }
            }
            // A transient accept error (too many open files, an aborted
            // connection) must not take the whole server down.
            Err(err) => eprintln!("MCP tcp accept error: {err}"),
        }
    }
}

fn serve_conn(shared: &EmuShared, stream: TcpStream) -> std::io::Result<()> {
    let out = stream.try_clone()?;
    serve(BufReader::new(stream), out, |request| {
        handle(shared, request)
    })
}

/// Serves one connection: reads frames from `reader`, answers through `out`
/// using `dispatch`, and returns at EOF or on the first write/read error.
///
/// Transport-agnostic on purpose: stdio and TCP differ only in the stream pair
/// they hand in, which keeps the framing testable without an emulator.
fn serve<R, W, F>(mut reader: R, mut out: W, mut dispatch: F) -> std::io::Result<()>
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

        // The frame is not necessarily valid UTF-8 (a raw TCP peer can send
        // anything); lossy decoding just turns it into a parse error.
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
    use std::time::Duration;

    /// Dispatcher standing in for `handle` so the framing can be exercised
    /// without a `EmuShared` (and therefore without SDL).
    fn fake_dispatch(request: &Value) -> Option<Value> {
        match request.get("method").and_then(Value::as_str) {
            Some("ping") => Some(ok_response(
                request.get("id").cloned().unwrap_or(Value::Null),
                json!({}),
            )),
            _ => None, // notifications, unknown methods
        }
    }

    /// Starts a one-connection server on an ephemeral port.
    fn serve_one() -> (SocketAddr, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let out = stream.try_clone().unwrap();
            let _ = serve(BufReader::new(stream), out, fake_dispatch);
        });
        (addr, handle)
    }

    #[test]
    fn tcp_frames_requests_answers_and_ignores_notifications() {
        let (addr, server) = serve_one();

        let client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut out = client.try_clone().unwrap();

        // A notification must never be answered, so anything read back below
        // can only be the reply to the `ping`.
        writeln!(
            out,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .unwrap();
        writeln!(out, "  ").unwrap(); // blank frames are skipped
        writeln!(out, r#"{{"jsonrpc":"2.0","id":1,"method":"ping"}}"#).unwrap();

        let mut line = String::new();
        BufReader::new(client.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let reply: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(reply["jsonrpc"], "2.0");
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"], json!({}));
        assert!(reply.get("error").is_none());

        // Dropping the client closes the connection, which ends the server.
        drop(out);
        drop(client);
        server.join().unwrap();
    }

    #[test]
    fn oversized_frame_is_reported_and_framing_survives() {
        let (addr, server) = serve_one();

        let client = TcpStream::connect(addr).unwrap();
        let mut out = client.try_clone().unwrap();

        let mut oversized = vec![b'a'; MAX_FRAME_BYTES + 64];
        oversized.push(b'\n');
        out.write_all(&oversized).unwrap();
        writeln!(out, r#"{{"jsonrpc":"2.0","id":7,"method":"ping"}}"#).unwrap();

        let mut line = String::new();
        BufReader::new(client.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let reply: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(reply["error"]["code"], -32700);

        // The oversized frame was drained up to its newline, so the next
        // request is still framed correctly.
        line.clear();
        BufReader::new(client.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        let reply: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(reply["id"], 7);
        assert_eq!(reply["result"], json!({}));

        drop(out);
        drop(client);
        server.join().unwrap();
    }
}
