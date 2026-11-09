//! MCP transports: `stdio` and Streamable HTTP.
//!
//! # What is implemented, and against which revision
//!
//! Both transports implement the **handshake-era** MCP protocol (revisions
//! `2025-03-26` through `2025-11-25`): `initialize` → `notifications/initialized`
//! → `tools/list` → `tools/call`. [`super::session::SUPPORTED_PROTOCOL_VERSIONS`]
//! lists exactly which revisions this client advertises.
//!
//! The `2026-07-28` revision replaced that handshake with per-request metadata
//! (`_meta` version, `Mcp-Method`/`Mcp-Name` headers, `-32022`
//! `UnsupportedProtocolVersionError`, no `initialize`). That is a different
//! protocol shape, not an increment, so it is **not** implemented here. When a
//! modern server rejects us, the client surfaces the supported-version list the
//! server returned instead of guessing a downgrade.
//!
//! # stdio
//!
//! Newline-delimited JSON-RPC on the child's stdin/stdout, per spec: messages are
//! single-line and the child must not write anything else to stdout. stderr is
//! *logging*, so it is captured into a bounded ring buffer for diagnosis and
//! never parsed. On Windows a bare program name (`npx`, `uvx`) is not an
//! executable path — `CreateProcess` will not resolve it — so those are launched
//! through `cmd /C`; a program that already carries an executable extension is
//! launched directly.
//!
//! # Streamable HTTP
//!
//! One MCP endpoint: POST for client→server messages, GET for the optional
//! server→client stream. A POST carrying a request answers either
//! `application/json` (one object) or `text/event-stream` (an SSE stream whose
//! final message is the response); both are handled. Notifications expect `202`.
//! `Mcp-Session-Id` is captured from the `initialize` response and echoed on every
//! later request; `MCP-Protocol-Version` is sent once negotiated; `Last-Event-ID`
//! is tracked so a GET stream can be resumed.
//!
//! # Disconnection is not cancellation
//!
//! The spec is explicit: a broken SSE stream or dropped socket is **not** a client
//! cancellation. This module only reports the disconnect; cancellation is an
//! explicit `notifications/cancelled` sent by
//! [`super::session::McpSession::cancel`].

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, Notify};

use super::jsonrpc::{classify, Inbound};

/// How many stderr lines are retained for diagnosis. Enough to explain a crash,
/// bounded so a chatty server cannot grow memory without limit.
pub const STDERR_RING_CAPACITY: usize = 200;

/// Raised when an HTTP server says the session it gave us no longer exists
/// (`404`). The spec requires the client to start a new session rather than reuse
/// it, so this is a distinct error, not a generic failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionExpired;

impl std::fmt::Display for SessionExpired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MCP session expired (HTTP 404); a new initialize is required")
    }
}

impl std::error::Error for SessionExpired {}

/// A bidirectional MCP message channel.
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    /// Write one message.
    async fn send(&self, message: &Value) -> Result<()>;
    /// Read the next inbound message. Dropping the future loses the read, not the
    /// transport.
    async fn recv(&mut self) -> Result<Inbound>;
    /// Close the channel. Idempotent.
    async fn close(&mut self) -> Result<()>;
    /// Human-readable identification for the inspector, e.g. `stdio npx -y x`.
    fn description(&self) -> String;
}

// ---------------------------------------------------------------------------
// SSE parsing
// ---------------------------------------------------------------------------

/// One decoded Server-Sent Event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The event's `id:` field, used as `Last-Event-ID` on resume.
    pub id: Option<String>,
    /// The concatenated `data:` payload.
    pub data: String,
}

/// Decode whole SSE frames out of `buffer`, returning the events and the
/// unconsumed remainder. Frames are separated by a blank line; `data:` lines
/// within a frame are joined with `\n` per the SSE standard.
pub fn decode_sse(buffer: &str) -> (Vec<SseEvent>, String) {
    let mut events = Vec::new();
    let mut rest = buffer;
    loop {
        // Frames end at a blank line, in either line-ending convention.
        let split = rest
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| rest.find("\n\n").map(|i| (i, 2)));
        let Some((idx, sep)) = split else {
            break;
        };
        let frame = &rest[..idx];
        rest = &rest[idx + sep..];

        let mut id = None;
        let mut data = String::new();
        for line in frame.lines() {
            let line = line.trim_end_matches('\r');
            if let Some(v) = line.strip_prefix("id:") {
                id = Some(v.trim().to_string());
            } else if let Some(v) = line.strip_prefix("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(v.strip_prefix(' ').unwrap_or(v));
            }
            // `event:`, `retry:` and comments are not needed by MCP: the message
            // type is inside the JSON payload.
        }
        if !data.is_empty() {
            events.push(SseEvent { id, data });
        }
    }
    (events, rest.to_string())
}

// ---------------------------------------------------------------------------
// stdio
// ---------------------------------------------------------------------------

/// Configuration for a spawned stdio server.
#[derive(Debug, Clone)]
pub struct StdioOptions {
    /// Program to run (`npx`, `uvx`, an absolute path, …).
    pub program: String,
    /// Arguments.
    pub args: Vec<String>,
    /// Extra environment variables, layered over the inherited environment.
    pub env: Vec<(String, String)>,
    /// Working directory.
    pub cwd: Option<PathBuf>,
}

impl StdioOptions {
    /// Minimal options for a program.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
        }
    }

    /// Append an argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set an environment variable.
    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }
}

/// Decide how to invoke `program` on this platform.
///
/// Windows cannot execute a bare command name through `CreateProcess`; the shell
/// resolves `npx` to `npx.cmd`. `.bat`/`.cmd` shims also cannot be spawned
/// directly by several Rust process APIs. Both cases are routed through `cmd /C`,
/// which is what a user typing the command would get. Any other platform, or a
/// program that already carries an executable extension, is invoked directly — no
/// shell, no argument re-quoting.
pub fn stdio_invocation(program: &str, args: &[String]) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let has_extension = std::path::Path::new(program)
            .extension()
            .map(|e| {
                matches!(
                    e.to_string_lossy().to_ascii_lowercase().as_str(),
                    "exe" | "cmd" | "bat" | "com"
                )
            })
            .unwrap_or(false);
        if !has_extension {
            let mut cmd_args = vec!["/C".to_string(), program.to_string()];
            cmd_args.extend(args.iter().cloned());
            return ("cmd".to_string(), cmd_args);
        }
    }
    (program.to_string(), args.to_vec())
}

/// A spawned MCP server speaking newline-delimited JSON-RPC.
pub struct StdioTransport {
    child: Child,
    /// Behind a mutex because [`Transport::send`] takes `&self`: several callers
    /// may write, and a half-written frame would corrupt the stream.
    stdin: Mutex<Option<ChildStdin>>,
    stdout: BufReader<ChildStdout>,
    stderr_tail: Arc<StdMutex<VecDeque<String>>>,
    stderr_task: Option<tokio::task::JoinHandle<()>>,
    description: String,
    closed: AtomicBool,
}

impl std::fmt::Debug for StdioTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdioTransport")
            .field("description", &self.description)
            .field("closed", &self.closed.load(Ordering::Relaxed))
            .finish()
    }
}

impl StdioTransport {
    /// Spawn the server and wire up stdin, stdout and a stderr capture task.
    pub fn spawn(options: &StdioOptions) -> Result<Self> {
        if options.program.trim().is_empty() {
            bail!("stdio server program must not be empty");
        }
        let (program, args) = stdio_invocation(&options.program, &options.args);

        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // A child that outlives its client is a leaked process; the spec's
            // shutdown is "close stdin, terminate the subprocess".
            .kill_on_drop(true);
        for (k, v) in &options.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &options.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn MCP server `{}`", options.program))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("child stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("child stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("child stderr unavailable"))?;

        let stderr_tail = Arc::new(StdMutex::new(VecDeque::with_capacity(STDERR_RING_CAPACITY)));
        let tail = Arc::clone(&stderr_tail);
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(mut ring) = tail.lock() {
                    if ring.len() == STDERR_RING_CAPACITY {
                        ring.pop_front();
                    }
                    ring.push_back(line);
                }
            }
        });

        let description = format!("stdio {} {}", options.program, options.args.join(" "));
        Ok(Self {
            child,
            stdin: Mutex::new(Some(stdin)),
            stdout: BufReader::new(stdout),
            stderr_tail,
            stderr_task: Some(stderr_task),
            description: description.trim_end().to_string(),
            closed: AtomicBool::new(false),
        })
    }

    /// The most recent stderr lines, oldest first.
    pub fn stderr_tail(&self) -> Vec<String> {
        self.stderr_tail
            .lock()
            .map(|r| r.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// True when the child has exited. `try_wait` reaps, so a crashed server is
    /// detected here rather than at the next write.
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, message: &Value) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            bail!("stdio transport is closed");
        }
        let mut line = serde_json::to_string(message).context("serialising JSON-RPC message")?;
        // Embedded newlines would split one message into two frames; serde never
        // emits them inside a JSON string, but the guard is cheap and makes the
        // framing invariant explicit.
        if line.contains('\n') {
            line = line.replace('\n', " ");
        }
        let mut guard = self.stdin.lock().await;
        let stdin = guard
            .as_mut()
            .ok_or_else(|| anyhow!("stdio transport is closed"))?;
        stdin
            .write_all(line.as_bytes())
            .await
            .context("writing to MCP server stdin")?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Inbound> {
        if self.closed.load(Ordering::SeqCst) {
            bail!("stdio transport is closed");
        }
        let mut line = String::new();
        loop {
            line.clear();
            let n = self
                .stdout
                .read_line(&mut line)
                .await
                .context("reading MCP server stdout")?;
            if n == 0 {
                // EOF: report why, using the captured stderr. Without this the
                // caller sees a bare "unexpected end of file" on every crash.
                let tail = self.stderr_tail();
                self.closed.store(true, Ordering::SeqCst);
                bail!(
                    "MCP server closed stdout{}",
                    if tail.is_empty() {
                        String::new()
                    } else {
                        format!("; stderr tail: {}", tail.join(" | "))
                    }
                );
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(trimmed).with_context(|| {
                format!(
                    "MCP server wrote a non-JSON line to stdout: {}",
                    snippet(trimmed)
                )
            })?;
            return classify(value).map_err(|e| anyhow!("malformed MCP message: {e}"));
        }
    }

    async fn close(&mut self) -> Result<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        // Closing stdin is the protocol's shutdown signal.
        self.stdin.lock().await.take();
        // Then terminate, with a bounded wait: a server that ignores EOF must not
        // hold the client open.
        let _ = self.child.start_kill();
        let _ = tokio::time::timeout(Duration::from_secs(2), self.child.wait()).await;
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
        Ok(())
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

fn snippet(s: &str) -> String {
    const MAX: usize = 160;
    if s.chars().count() <= MAX {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(MAX).collect();
        format!("{truncated}…")
    }
}

// ---------------------------------------------------------------------------
// Streamable HTTP
// ---------------------------------------------------------------------------

/// Configuration for a Streamable HTTP server.
#[derive(Debug, Clone)]
pub struct HttpOptions {
    /// The single MCP endpoint, e.g. `http://127.0.0.1:8931/mcp`.
    pub url: String,
    /// Extra headers (authorization, tenant, …).
    pub headers: Vec<(String, String)>,
    /// Per-request timeout.
    pub timeout: Duration,
}

impl HttpOptions {
    /// Options for an endpoint.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            timeout: Duration::from_secs(30),
        }
    }
}

/// The message queue and wakeup shared with the GET listener task.
#[derive(Debug, Default)]
struct InboundQueue {
    messages: Mutex<VecDeque<Inbound>>,
    notify: Notify,
}

impl InboundQueue {
    async fn push(&self, message: Inbound) {
        self.messages.lock().await.push_back(message);
        self.notify.notify_waiters();
    }

    async fn pop(&self) -> Option<Inbound> {
        self.messages.lock().await.pop_front()
    }
}

/// A Streamable HTTP client transport.
pub struct StreamableHttpTransport {
    client: reqwest::Client,
    url: String,
    headers: Vec<(String, String)>,
    timeout: Duration,
    session_id: StdMutex<Option<String>>,
    protocol_version: StdMutex<Option<String>>,
    last_event_id: StdMutex<Option<String>>,
    queue: Arc<InboundQueue>,
    listener: Mutex<Option<tokio::task::JoinHandle<()>>>,
    listener_stop: Arc<AtomicBool>,
    closed: AtomicBool,
}

impl std::fmt::Debug for StreamableHttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamableHttpTransport")
            .field("url", &self.url)
            .field(
                "session",
                &self
                    .session_id
                    .lock()
                    .ok()
                    .and_then(|s| s.as_ref().cloned()),
            )
            .finish()
    }
}

impl StreamableHttpTransport {
    /// Build a transport. No connection is made until the first `send`.
    pub fn new(options: HttpOptions) -> Result<Self> {
        if options.url.trim().is_empty() {
            bail!("HTTP server url must not be empty");
        }
        let client = reqwest::Client::builder()
            .timeout(options.timeout)
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            client,
            url: options.url,
            headers: options.headers,
            timeout: options.timeout,
            session_id: StdMutex::new(None),
            protocol_version: StdMutex::new(None),
            last_event_id: StdMutex::new(None),
            queue: Arc::new(InboundQueue::default()),
            listener: Mutex::new(None),
            listener_stop: Arc::new(AtomicBool::new(false)),
            closed: AtomicBool::new(false),
        })
    }

    /// Record the protocol version negotiated by the session, so every later
    /// request carries `MCP-Protocol-Version`.
    pub fn set_protocol_version(&self, version: &str) {
        if let Ok(mut v) = self.protocol_version.lock() {
            *v = Some(version.to_string());
        }
    }

    /// The session id assigned by the server, if any.
    pub fn session_id(&self) -> Option<String> {
        self.session_id
            .lock()
            .ok()
            .and_then(|s| s.as_ref().cloned())
    }

    fn request(&self, method: reqwest::Method) -> reqwest::RequestBuilder {
        let mut rb = self.client.request(method, &self.url);
        for (k, v) in &self.headers {
            rb = rb.header(k, v);
        }
        if let Some(s) = self.session_id() {
            rb = rb.header("Mcp-Session-Id", s);
        }
        if let Ok(version) = self.protocol_version.lock() {
            if let Some(v) = version.as_deref() {
                rb = rb.header("MCP-Protocol-Version", v);
            }
        }
        rb
    }

    async fn push_sse(&self, events: Vec<SseEvent>) -> Result<()> {
        for event in events {
            if let Some(id) = &event.id {
                if let Ok(mut last) = self.last_event_id.lock() {
                    *last = Some(id.clone());
                }
            }
            let value: Value = serde_json::from_str(&event.data)
                .with_context(|| format!("SSE data was not JSON: {}", snippet(&event.data)))?;
            let message = classify(value).map_err(|e| anyhow!("malformed MCP message: {e}"))?;
            self.queue.push(message).await;
        }
        Ok(())
    }

    /// Open the optional GET stream. Messages the server sends unsolicited arrive
    /// here; the spec forbids a server sending a *response* on this stream except
    /// when resuming, so none is expected.
    pub async fn open_listener(&self) -> Result<()> {
        let mut guard = self.listener.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        self.listener_stop.store(false, Ordering::SeqCst);

        let client = self.client.clone();
        let url = self.url.clone();
        let session = self.session_id();
        let version = self.protocol_version.lock().ok().and_then(|v| v.clone());
        let last_event_id = self.last_event_id.lock().ok().and_then(|v| v.clone());
        let extra = self.headers.clone();
        let stop = Arc::clone(&self.listener_stop);
        let queue = Arc::clone(&self.queue);

        *guard = Some(tokio::spawn(async move {
            let mut rb = client
                .request(reqwest::Method::GET, &url)
                .header("Accept", "text/event-stream");
            for (k, v) in &extra {
                rb = rb.header(k, v);
            }
            if let Some(s) = session {
                rb = rb.header("Mcp-Session-Id", s);
            }
            if let Some(v) = version {
                rb = rb.header("MCP-Protocol-Version", v);
            }
            if let Some(id) = last_event_id {
                rb = rb.header("Last-Event-ID", id);
            }

            let Ok(response) = rb.send().await else {
                return;
            };
            // 405 means the server offers no stream at this endpoint; that is a
            // valid server, not a failure.
            if !response.status().is_success() {
                return;
            }

            let mut buffer = String::new();
            let mut stream = response.bytes_stream();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                let Ok(bytes) = chunk else { return };
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                let (events, rest) = decode_sse(&buffer);
                buffer = rest;
                for event in events {
                    // A malformed frame ends this listener; it must not fail a
                    // request that has already been answered elsewhere.
                    if let Ok(value) = serde_json::from_str::<Value>(&event.data) {
                        if let Ok(message) = classify(value) {
                            queue.push(message).await;
                        }
                    }
                }
            }
        }));
        Ok(())
    }

    /// Stop the GET stream task, if any.
    pub async fn close_listener(&self) {
        self.listener_stop.store(true, Ordering::SeqCst);
        if let Some(task) = self.listener.lock().await.take() {
            task.abort();
        }
    }
}

#[async_trait]
impl Transport for StreamableHttpTransport {
    async fn send(&self, message: &Value) -> Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            bail!("HTTP transport is closed");
        }
        let is_request = message.get("id").is_some() && message.get("method").is_some();

        let rb = self
            .request(reqwest::Method::POST)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(message);

        let response = rb.send().await.context("POST to MCP endpoint")?;
        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow::Error::new(SessionExpired));
        }
        if !is_request {
            // Notifications and responses must be accepted with 202 and no body.
            if status.is_success() {
                return Ok(());
            }
            let body = response.text().await.unwrap_or_default();
            bail!(
                "MCP endpoint rejected a notification with HTTP {}: {}",
                status,
                snippet(&body)
            );
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("MCP endpoint returned HTTP {}: {}", status, snippet(&body));
        }

        // Capture a session id from the initialize response, if the server
        // assigned one. Read before the body is consumed so a streaming response
        // cannot race it.
        if let Some(id) = response
            .headers()
            .get("mcp-session-id")
            .and_then(|v| v.to_str().ok())
        {
            if let Ok(mut session) = self.session_id.lock() {
                if session.as_deref() != Some(id) {
                    *session = Some(id.to_string());
                }
            }
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        if content_type.starts_with("text/event-stream") {
            let mut buffer = String::new();
            let mut stream = response.bytes_stream();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                let bytes = chunk.context("reading SSE stream")?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                let (events, rest) = decode_sse(&buffer);
                buffer = rest;
                self.push_sse(events).await?;
            }
            return Ok(());
        }

        // A single JSON object.
        let body = response.text().await.context("reading MCP response body")?;
        if body.trim().is_empty() {
            bail!("MCP endpoint returned an empty body for a request");
        }
        let value: Value = serde_json::from_str(&body)
            .with_context(|| format!("MCP response was not JSON: {}", snippet(&body)))?;
        let parsed = classify(value).map_err(|e| anyhow!("malformed MCP message: {e}"))?;
        self.queue.push(parsed).await;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Inbound> {
        // Fast path: something already buffered (the common case: send() buffered
        // the POST response).
        if let Some(msg) = self.queue.pop().await {
            return Ok(msg);
        }
        // Otherwise a message may still arrive on the GET stream.
        let waited = tokio::time::timeout(self.timeout, async {
            loop {
                self.queue.notify.notified().await;
                if let Some(msg) = self.queue.pop().await {
                    return Some(msg);
                }
            }
        })
        .await;
        match waited {
            Ok(Some(msg)) => Ok(msg),
            _ => bail!("no MCP message available from the HTTP endpoint"),
        }
    }

    async fn close(&mut self) -> Result<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.close_listener().await;
        // Spec: terminate the session explicitly if the server assigned one. 405
        // means the server does not allow it, which is not an error.
        if self.session_id().is_some() {
            let rb = self
                .request(reqwest::Method::DELETE)
                .header("Accept", "application/json");
            if let Ok(resp) = rb.send().await {
                if !resp.status().is_success()
                    && resp.status() != reqwest::StatusCode::METHOD_NOT_ALLOWED
                {
                    tracing::debug!(status = %resp.status(), "MCP session DELETE was refused");
                }
            }
        }
        Ok(())
    }

    fn description(&self) -> String {
        format!("streamable-http {}", self.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_two_frames_and_keeps_the_tail() {
        let buffer = "id: 1\ndata: {\"a\":1}\n\ndata: {\"b\":2}\n\nid: 3\ndata: par";
        let (events, rest) = decode_sse(buffer);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id.as_deref(), Some("1"));
        assert_eq!(events[0].data, "{\"a\":1}");
        assert_eq!(events[1].id, None);
        assert_eq!(rest, "id: 3\ndata: par");
    }

    #[test]
    fn decodes_crlf_frames_and_multi_line_data() {
        let buffer = "data: {\"a\":\r\ndata: 1}\r\n\r\n";
        let (events, rest) = decode_sse(buffer);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{\"a\":\n1}");
        assert!(rest.is_empty());
    }

    #[test]
    fn ignores_comments_and_event_names() {
        let (events, _) = decode_sse(": keepalive\nevent: message\ndata: {}\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "{}");
    }

    #[test]
    fn bare_program_is_wrapped_in_cmd_on_windows_only() {
        let (program, args) = stdio_invocation("npx", &["-y".into(), "pkg".into()]);
        #[cfg(windows)]
        {
            assert_eq!(program, "cmd");
            assert_eq!(args, vec!["/C", "npx", "-y", "pkg"]);
        }
        #[cfg(not(windows))]
        {
            assert_eq!(program, "npx");
            assert_eq!(args, vec!["-y", "pkg"]);
        }
    }

    #[test]
    fn an_exe_path_is_not_wrapped_on_any_platform() {
        let (program, args) = stdio_invocation("C:\\tools\\server.exe", &["--x".into()]);
        assert_eq!(program, "C:\\tools\\server.exe");
        assert_eq!(args, vec!["--x"]);
    }

    #[test]
    fn empty_program_is_refused() {
        let err = StdioTransport::spawn(&StdioOptions::new("   ")).unwrap_err();
        assert!(err.to_string().contains("must not be empty"), "{err}");
    }
}
