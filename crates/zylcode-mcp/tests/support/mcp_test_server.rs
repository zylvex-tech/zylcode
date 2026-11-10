//! A real, deliberately imperfect MCP server for the client's integration tests.
//!
//! # Why a process and not a mock
//!
//! The client's job is to talk to *something over a transport*. A mock of the
//! client's own types would test the mock. This binary is a genuine child
//! process: it is spawned, it speaks newline-delimited JSON-RPC on stdin/stdout,
//! it can be slow, it can crash, and it can be restarted — which is exactly what
//! the supervisor claims to survive.
//!
//! # Modes
//!
//! | `--mode=` | Behaviour |
//! | --------- | --------- |
//! | *(none)* | Well-behaved: handshake, two-page `tools/list`, echoing `tools/call` |
//! | `wrong-version` | Selects protocol `2026-07-28`, which the client does not implement |
//! | `malformed` | Emits a non-JSON line before each response |
//! | `slow` | Sleeps 2s before answering `tools/call` |
//! | `die-after-call` | Answers one `tools/call`, then exits |
//!
//! # Observation log
//!
//! When `ZYLCODE_TEST_SERVER_LOG` names a file, every method received is
//! appended to it. Tests use that to prove ordering — in particular that
//! `notifications/initialized` arrived before any other request, which the spec
//! requires and which is invisible in responses alone.

use std::io::{self, BufRead, Write};
use std::time::Duration;

use serde_json::{json, Value};

fn main() {
    let mode = std::env::args()
        .find_map(|a| a.strip_prefix("--mode=").map(str::to_string))
        .unwrap_or_default();
    let log_path = std::env::var("ZYLCODE_TEST_SERVER_LOG").ok();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut calls_served = 0u32;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        if mode == "malformed" {
            // A server that writes junk to stdout is a real failure mode: the
            // framing layer must report it rather than desynchronise forever.
            let _ = writeln!(out, "this line is not json-rpc");
            let _ = out.flush();
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let _ = respond(
                    &mut out,
                    Value::Null,
                    Err((-32700, "Parse error".to_string())),
                );
                continue;
            }
        };

        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(path) = &log_path {
            let _ = append_log(path, &method);
        }

        let id = message.get("id").cloned().unwrap_or(Value::Null);

        // Notifications carry no id and take no response.
        if id.is_null() {
            continue;
        }

        match method.as_str() {
            "initialize" => {
                let version = if mode == "wrong-version" {
                    "2026-07-28"
                } else {
                    "2025-06-18"
                };
                let _ = respond(
                    &mut out,
                    id,
                    Ok(json!({
                        "protocolVersion": version,
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "zylcode-test-server", "version": "1.0.0" },
                        "instructions": "Test server. Tools are fixtures, not capabilities."
                    })),
                );
            }
            "ping" => {
                let _ = respond(&mut out, id, Ok(json!({})));
            }
            "tools/list" => {
                let cursor = message
                    .get("params")
                    .and_then(|p| p.get("cursor"))
                    .and_then(Value::as_str);
                let body = match cursor {
                    None => json!({
                        "tools": [
                            {
                                "name": "alpha",
                                "title": "Alpha",
                                "description": "Unannotated: must be treated as approval-requiring.",
                                "inputSchema": { "type": "object" }
                            },
                            {
                                "name": "beta",
                                "description": "Declared read-only.",
                                "inputSchema": { "type": "object" },
                                "annotations": { "readOnlyHint": true }
                            }
                        ],
                        "nextCursor": "page-2"
                    }),
                    Some("page-2") => json!({
                        "tools": [
                            {
                                "name": "gamma",
                                "description": "Declared destructive, and lies about nothing else.",
                                "inputSchema": { "type": "object" },
                                "annotations": { "destructiveHint": true }
                            }
                        ]
                    }),
                    Some(other) => {
                        let _ = respond(
                            &mut out,
                            id,
                            Err((-32602, format!("unknown cursor `{other}`"))),
                        );
                        continue;
                    }
                };
                let _ = respond(&mut out, id, Ok(body));
            }
            "tools/call" => {
                calls_served += 1;
                if mode == "slow" {
                    std::thread::sleep(Duration::from_secs(2));
                }
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                if name == "missing" {
                    let _ = respond(
                        &mut out,
                        id,
                        Err((-32602, "Unknown tool: missing".to_string())),
                    );
                } else {
                    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                    let _ = respond(
                        &mut out,
                        id,
                        Ok(json!({
                            "content": [{ "type": "text", "text": format!("{name} ok") }],
                            "structuredContent": { "tool": name, "arguments": arguments },
                            "isError": false
                        })),
                    );
                }
                if mode == "die-after-call" && calls_served >= 1 {
                    let _ = out.flush();
                    std::process::exit(0);
                }
            }
            "notifications/cancelled" => {}
            other => {
                let _ = respond(
                    &mut out,
                    id,
                    Err((-32601, format!("Method not found: {other}"))),
                );
            }
        }
    }
}

fn respond(
    out: &mut impl Write,
    id: Value,
    result: Result<Value, (i64, String)>,
) -> io::Result<()> {
    let body = match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    };
    writeln!(out, "{body}")?;
    out.flush()
}

fn append_log(path: &str, method: &str) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{method}")?;
    Ok(())
}
