//! Respan hosted gateway — a real HTTP round trip against a loopback stand-in.
//!
//! # What is real here and what is not
//!
//! `StubGateway` below is a **TEST DOUBLE**: a loopback HTTP server that answers
//! one request with canned bytes and hands back the request it received. It is
//! **not** Respan, and a passing run in this file is **not** evidence that a
//! Respan trace exists.
//!
//! What these tests do establish, without any Respan credential:
//!
//! * `RESPAN_API_KEY` alone selects the gateway — `RouterConfig::from_env`
//!   makes it the primary provider, and `RESPAN_BASE_URL` moves the endpoint
//!   (so the same code reaches Respan Enterprise, or this stub);
//! * the request the router sends is the documented gateway shape:
//!   `POST {base}/chat/completions` with `Authorization: Bearer <key>`, the
//!   selected `model`, and the system/user messages the router assembled;
//! * an OpenAI-compatible reply becomes the router's completion text and its
//!   `usage` counters are recorded;
//! * with no `RESPAN_API_KEY` the gateway is not selected at all.
//!
//! Live verification — that Respan received the call and logged it — needs the
//! real key and is a separate step; `tests/live_commissioning.rs` is the probe
//! that establishes it.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use zylcode_core::router::{
    ModelProvider, RouterConfig, TokenRouter, RESPAN_DEFAULT_MODEL, RESPAN_GATEWAY_BASE_URL,
};

/// Serialises the tests in this file: they mutate process-global environment.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A credential-shaped filler that is obviously not a real key. It must be
/// non-empty (the router treats blank as unset) and must never match a
/// secret-scanner pattern, so it carries no provider prefix and no random body.
const FILLER_KEY: &str = "placeholder-credential-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

/// A canned OpenAI-compatible reply, of the shape the gateway documents.
fn gateway_reply() -> String {
    r#"{"id":"chatcmpl-stub","object":"chat.completion","created":0,
        "model":"gpt-5.5",
        "choices":[{"index":0,"finish_reason":"stop",
                    "message":{"role":"assistant","content":"Hello from the gateway"}}],
        "usage":{"prompt_tokens":42,"completion_tokens":7,"total_tokens":49}}"#
        .to_string()
}

// ---------------------------------------------------------------------------
// TEST DOUBLE — not Respan
// ---------------------------------------------------------------------------

/// A minimal loopback HTTP server that answers exactly one request with fixed
/// bytes and reports the bytes it received. A **test double**, never evidence.
struct StubGateway {
    base_url: String,
    received: Receiver<String>,
}

impl StubGateway {
    fn spawn(body: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let (tx, received) = mpsc::channel();

        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let request = read_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
            let _ = tx.send(request);
        });

        Self {
            base_url: format!("http://{addr}"),
            received,
        }
    }

    /// The one request the router sent, as received on the wire.
    fn request(self) -> String {
        self.received
            .recv_timeout(Duration::from_secs(10))
            .expect("the router must send exactly one request")
    }
}

/// Read one HTTP request: headers, then `Content-Length` bytes of body.
fn read_request(stream: &mut TcpStream) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).expect("read request");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(head_end) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
            let expected = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buf.len() >= head_end + 4 + expected {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Environment scope
// ---------------------------------------------------------------------------

/// Process-global environment, saved and restored around a test.
struct EnvScope {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvScope {
    fn capture(names: &[&'static str]) -> Self {
        Self {
            saved: names.iter().map(|k| (*k, std::env::var(k).ok())).collect(),
        }
    }

    fn set(&self, name: &'static str, value: &str) {
        debug_assert!(self.saved.iter().any(|(n, _)| *n == name));
        std::env::set_var(name, value);
    }

    fn unset(&self, name: &'static str) {
        debug_assert!(self.saved.iter().any(|(n, _)| *n == name));
        std::env::remove_var(name);
    }
}

impl Drop for EnvScope {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Gateway selection and request shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_dispatch_reaches_the_gateway_with_the_documented_request() {
    let _guard = ENV_LOCK.lock().await;

    let stub = StubGateway::spawn(gateway_reply());
    let stub_base = stub.base_url.clone();
    let dir = tempfile::tempdir().expect("temp dir");
    let scorecard = dir
        .path()
        .join("scorecard.db")
        .to_str()
        .expect("utf-8 path")
        .to_string();

    let env = EnvScope::capture(&[
        "RESPAN_API_KEY",
        "RESPAN_MODEL",
        "RESPAN_BASE_URL",
        "ZYLCODE_PRIMARY_MODEL",
        "ZYLCODE_SCORECARD_PATH",
    ]);
    env.set("RESPAN_API_KEY", FILLER_KEY);
    // Exactly the base-URL shape the Respan docs show, trailing slash included.
    env.set("RESPAN_BASE_URL", &format!("{stub_base}/"));
    env.unset("RESPAN_MODEL");
    env.unset("ZYLCODE_PRIMARY_MODEL");
    env.set("ZYLCODE_SCORECARD_PATH", &scorecard);

    let cfg = RouterConfig::from_env();
    assert_eq!(
        cfg.primary_provider,
        ModelProvider::Respan,
        "RESPAN_API_KEY must select the gateway"
    );
    assert_eq!(cfg.primary_model, RESPAN_DEFAULT_MODEL);

    let router = TokenRouter::without_vector_cache(cfg).expect("router");
    let text = router
        .dispatch_prompt("say hello", "you are a gateway probe")
        .await
        .expect("dispatch through the gateway stand-in");
    assert_eq!(text, "Hello from the gateway");

    let snapshot = router.snapshot();
    assert_eq!(
        snapshot.input_tokens, 42,
        "usage.prompt_tokens from the gateway reply must be recorded"
    );
    assert_eq!(
        snapshot.output_tokens, 7,
        "usage.completion_tokens from the gateway reply must be recorded"
    );

    let request = stub.request();
    let (head, body) = request
        .split_once("\r\n\r\n")
        .expect("the request must have headers and a body");

    assert!(
        head.starts_with("POST /chat/completions HTTP/1.1"),
        "the gateway's OpenAI-compatible path must be used, got: {head}"
    );
    assert!(
        head.to_lowercase()
            .contains(&format!("authorization: bearer {FILLER_KEY}")),
        "RESPAN_API_KEY must travel as a bearer token"
    );

    let body: serde_json::Value = serde_json::from_str(body).expect("json request body");
    assert_eq!(body["model"], RESPAN_DEFAULT_MODEL);
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages.len(), 2, "system + user, as the router assembles");
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "you are a gateway probe");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "say hello");
}

#[tokio::test]
async fn without_the_key_the_gateway_is_not_selected() {
    let _guard = ENV_LOCK.lock().await;

    let env = EnvScope::capture(&["RESPAN_API_KEY", "RESPAN_BASE_URL"]);
    env.unset("RESPAN_API_KEY");
    env.unset("RESPAN_BASE_URL");

    let cfg = RouterConfig::from_env();
    assert_ne!(
        cfg.primary_provider,
        ModelProvider::Respan,
        "an absent key must leave provider selection untouched"
    );
    assert!(cfg.api_key(&ModelProvider::Respan).is_none());
    assert_eq!(
        cfg.base_url(&ModelProvider::Respan),
        RESPAN_GATEWAY_BASE_URL,
        "the gateway endpoint is fixed even when it is not in use"
    );
}
