//! Supervision for external MCP servers: states, backoff, discovery, status.
//!
//! # Why a supervisor and not "connect and hope"
//!
//! `mcp.tools.yaml` used to carry a `transport: Stdio` label that nothing read.
//! Now that a transport is a real child process or a real HTTP endpoint, the
//! interesting cases are the unhappy ones: a server that fails to start, one
//! that dies mid-session, one that keeps dying. A supervisor is what lets the
//! rest of the system ask "is this server usable?" and get an answer instead of
//! a hang.
//!
//! # States
//!
//! ```text
//! Disabled ──────────────────────────────┐
//! Stopped ──start──▶ Starting ──ok──▶ Ready
//!    ▲                  │              │
//!    │                fail           crash
//!    │                  ▼              ▼
//!    └────────────── Failed ◀──── Restarting ──ok──▶ Ready
//! ```
//!
//! `Failed` is terminal for the current configuration: the supervisor stops
//! retrying once `restart.max_attempts` is exhausted, so a broken server cannot
//! spin forever and flood the evidence log. A later explicit start resets it.
//!
//! # No blind retry of a tool call
//!
//! A transport failure during `tools/call` is genuinely ambiguous: the server
//! may have executed the tool and lost the response. Retrying automatically
//! would run a side-effecting tool twice, which is worse than failing. So a
//! failed call marks the session dead and reconnects **for the next
//! invocation**, and the failed call is reported as failed.
//!
//! `ping` is the only operation this supervisor retries, because it has no
//! effect: one transport failure is followed by one reconnect and one more
//! `ping`. An error *reply* to a ping is not retried — the server answered, and
//! its answer is the failure.
//!
//! # What counts as an attempt
//!
//! One attempt is a handshake **and** a complete `tools/list` walk. A server
//! that greets the client and then fails discovery is not ready, so that failure
//! is charged to `restart.max_attempts`, retained as `last_error`, and retried
//! with the same backoff as a failed connect. It does not escape the supervisor
//! and leave the slot reporting `Starting`.
//!
//! # Locking
//!
//! No lock is held across a network exchange. The status mutex covers
//! bookkeeping only, the live session has its own mutex, and `connect_gate`
//! makes attempts sequential. That is what lets the inspector answer while a
//! slow tool runs, and it is also what stops two callers from opening two
//! connections to one server.
//!
//! # Status snapshot
//!
//! [`McpRegistry::snapshot`] is what a UI renders. It reports observed facts —
//! negotiated protocol version, server name, discovered tool ids, attempt count,
//! last error — and nothing inferred. A server that has never connected reports
//! `Stopped` with no protocol version, not a plausible-looking one. The handshake
//! facts are recorded when the session is installed, so a snapshot never waits
//! on a call in flight, and a lost session stops being described.

use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use super::config::{McpServerConfig, McpServersFile, StartupPolicy};
use super::jsonrpc::JsonRpcError;
use super::remote::RemoteTool;
use super::session::{ClientInfo, McpSession, RemoteToolDef};
use super::transport::{StdioTransport, StreamableHttpTransport, Transport};
use crate::real_tools::RealTool;

/// What the supervisor currently believes about one server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    /// Configured but `enabled: false`.
    Disabled,
    /// Configured, not yet connected.
    Stopped,
    /// A connection attempt is in flight.
    Starting,
    /// Handshake completed and tools discovered.
    Ready,
    /// A previous session died; reconnection in progress.
    Restarting,
    /// Retries exhausted for the current configuration.
    Failed,
}

impl ServerState {
    /// Whether a call may be attempted in this state.
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Stopped | Self::Ready | Self::Restarting)
    }
}

/// Observed status of one server, for the inspector.
#[derive(Debug, Clone, Serialize)]
pub struct ServerSnapshot {
    /// Configured id (also the tool namespace).
    pub id: String,
    /// `stdio` or `http`.
    pub transport: String,
    /// Startup policy from configuration.
    pub startup: StartupPolicy,
    /// Current state.
    pub state: ServerState,
    /// Protocol version actually negotiated, if a handshake completed.
    pub protocol_version: Option<String>,
    /// Server's self-reported name, if a handshake completed.
    pub server_name: Option<String>,
    /// Server's self-reported version.
    pub server_version: Option<String>,
    /// Discovered tool names, un-namespaced.
    pub tools: Vec<String>,
    /// Consecutive failed attempts since the last success.
    pub attempts: u32,
    /// Last error, verbatim.
    pub last_error: Option<String>,
    /// Instructions the server supplied in its handshake, if any.
    pub instructions: Option<String>,
}

impl ServerSnapshot {
    /// Whether this server can serve a call right now.
    pub fn is_ready(&self) -> bool {
        self.state == ServerState::Ready
    }
}

/// What the handshake said, recorded when the session is installed.
///
/// Cached here rather than read back through the live session so that
/// [`ServerSlot::snapshot`] answers without waiting on a call in flight. It is
/// filled only from a real [`McpSession::handshake`] and cleared with the session
/// it describes, so it never describes a connection that no longer exists.
#[derive(Debug, Clone)]
struct SessionFacts {
    protocol_version: String,
    server_name: String,
    server_version: String,
    instructions: Option<String>,
}

impl SessionFacts {
    fn of(session: &McpSession) -> Self {
        let handshake = session.handshake();
        Self {
            protocol_version: handshake.protocol_version.clone(),
            server_name: handshake.server_info.name.clone(),
            server_version: handshake.server_info.version.clone(),
            instructions: handshake.instructions.clone(),
        }
    }
}

/// Mutable per-server status, guarded by one mutex.
///
/// This mutex is never held across a network exchange; see the module note on
/// locking.
#[derive(Debug)]
struct SlotInner {
    state: ServerState,
    facts: Option<SessionFacts>,
    tools: Vec<RemoteToolDef>,
    attempts: u32,
    last_error: Option<String>,
}

/// One supervised server.
///
/// `Arc<ServerSlot>` is shared with every [`RemoteTool`] discovered on it, so a
/// tool outlives a restart of the process that serves it.
#[derive(Debug)]
pub struct ServerSlot {
    config: McpServerConfig,
    client: ClientInfo,
    inner: Mutex<SlotInner>,
    /// The live session, held only for the duration of one exchange.
    live: Mutex<Option<McpSession>>,
    /// Serialises connect attempts, so two callers never open two sessions.
    connect_gate: Mutex<()>,
}

impl ServerSlot {
    /// Create a slot in the state its configuration implies.
    pub fn new(config: McpServerConfig, client: ClientInfo) -> Self {
        let state = if config.enabled {
            ServerState::Stopped
        } else {
            ServerState::Disabled
        };
        Self {
            config,
            client,
            inner: Mutex::new(SlotInner {
                state,
                facts: None,
                tools: Vec::new(),
                attempts: 0,
                last_error: None,
            }),
            live: Mutex::new(None),
            connect_gate: Mutex::new(()),
        }
    }

    /// Configured id.
    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// Current state, without waiting on a connection.
    pub async fn state(&self) -> ServerState {
        self.inner.lock().await.state
    }

    /// Observed status.
    ///
    /// Reads the slot's own record only: it never waits on the lock a call in
    /// flight holds, so the inspector keeps answering while a tool runs.
    pub async fn snapshot(&self) -> ServerSnapshot {
        let inner = self.inner.lock().await;
        ServerSnapshot {
            id: self.config.id.clone(),
            transport: self.config.transport.to_string(),
            startup: self.config.startup,
            state: inner.state,
            protocol_version: inner.facts.as_ref().map(|f| f.protocol_version.clone()),
            server_name: inner
                .facts
                .as_ref()
                .map(|f| f.server_name.clone())
                .filter(|n| !n.is_empty()),
            server_version: inner
                .facts
                .as_ref()
                .map(|f| f.server_version.clone())
                .filter(|v| !v.is_empty()),
            tools: inner.tools.iter().map(|t| t.name.clone()).collect(),
            attempts: inner.attempts,
            last_error: inner.last_error.clone(),
            instructions: inner.facts.as_ref().and_then(|f| f.instructions.clone()),
        }
    }

    /// Open the transport this configuration describes and handshake over it.
    async fn connect_once(&self) -> Result<McpSession> {
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let transport: Box<dyn Transport> = match self.config.transport {
            super::config::ServerTransport::Stdio => {
                let options = self.config.stdio_options()?;
                Box::new(StdioTransport::spawn(&options)?)
            }
            super::config::ServerTransport::Http => {
                let options = self.config.http_options()?;
                let http = StreamableHttpTransport::new(options)?;
                Box::new(http)
            }
        };
        McpSession::connect(transport, self.client.clone(), timeout).await
    }

    /// Whether the slot is already usable, or must refuse outright.
    ///
    /// `Ready` implies a live session: a session is installed before the state
    /// says `Ready`, and every path that forgets a session also leaves `Ready`.
    async fn already_ready(&self) -> Result<bool> {
        let inner = self.inner.lock().await;
        match inner.state {
            ServerState::Ready => Ok(true),
            ServerState::Disabled => bail!(
                "MCP server `{}` is disabled in configuration",
                self.config.id
            ),
            _ => Ok(false),
        }
    }

    /// Connect if needed, retrying with the configured backoff.
    ///
    /// `max_attempts` counts **consecutive** failures, so one successful start
    /// restores the budget. No lock is held across a connect: `connect_gate`
    /// serialises attempts, which keeps two callers from opening two sessions
    /// while leaving status reads free.
    pub async fn ensure_ready(&self) -> Result<()> {
        if self.already_ready().await? {
            return Ok(());
        }

        let _gate = self.connect_gate.lock().await;
        // Whoever held the gate may have finished the job while we waited.
        if self.already_ready().await? {
            return Ok(());
        }
        self.connect_and_discover().await
    }

    /// The supervised connect loop. The caller holds `connect_gate`.
    async fn connect_and_discover(&self) -> Result<()> {
        let policy = self.config.restart.clone();
        loop {
            let attempt = {
                let mut inner = self.inner.lock().await;
                inner.state = if inner.attempts == 0 {
                    ServerState::Starting
                } else {
                    ServerState::Restarting
                };
                inner.attempts = inner.attempts.saturating_add(1);
                inner.attempts
            };

            // Handshake and discovery are one attempt. A server that greets the
            // client and then fails `tools/list` is not ready, so the failure
            // takes the same path as a failed connect — charged to the budget,
            // retained as `last_error`, retried with backoff — instead of
            // escaping through `?` and leaving the slot reporting `Starting`.
            let outcome = match self.connect_once().await {
                Ok(mut session) => match session.list_all_tools().await {
                    Ok(tools) => Ok((session, tools)),
                    Err(error) => Err(error.context("discovering tools")),
                },
                Err(error) => Err(error),
            };

            match outcome {
                Ok((session, tools)) => {
                    let facts = SessionFacts::of(&session);
                    // Install before announcing readiness: a caller that sees
                    // `Ready` must always find a session.
                    *self.live.lock().await = Some(session);
                    let mut inner = self.inner.lock().await;
                    inner.state = ServerState::Ready;
                    inner.facts = Some(facts);
                    inner.tools = tools;
                    inner.attempts = 0;
                    inner.last_error = None;
                    return Ok(());
                }
                Err(error) => {
                    let exhausted = attempt >= policy.max_attempts.max(1);
                    *self.live.lock().await = None;
                    {
                        let mut inner = self.inner.lock().await;
                        inner.facts = None;
                        inner.tools.clear();
                        // `{:#}` keeps the cause chain: an outer "discovering
                        // tools" alone would tell the inspector nothing.
                        inner.last_error = Some(format!("{error:#}"));
                        if exhausted {
                            inner.state = ServerState::Failed;
                        }
                    }
                    if exhausted {
                        return Err(anyhow!(
                            "MCP server `{}` failed after {} attempt(s): {error:#}",
                            self.config.id,
                            attempt
                        ));
                    }
                    tokio::time::sleep(policy.backoff_for(attempt)).await;
                }
            }
        }
    }

    /// Discovered tools, as adapters bound to this slot.
    pub async fn remote_tools(self: &Arc<Self>) -> Vec<Arc<RemoteTool>> {
        let inner = self.inner.lock().await;
        inner
            .tools
            .iter()
            .map(|def| {
                Arc::new(RemoteTool::new(
                    &self.config.id,
                    def.clone(),
                    Arc::clone(self),
                ))
            })
            .collect()
    }

    /// Call a tool on this server.
    ///
    /// Connects first if the startup policy allows it. A transport failure is
    /// **not** retried here: see the module note on double execution. The
    /// session is dropped so the next call reconnects.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<super::session::ToolCallOutcome> {
        self.ensure_ready().await?;
        let mut live = self.live.lock().await;
        if live.is_none() {
            // The session died between the readiness check and this lock. One
            // reconnect, then the call either runs or fails for a reason.
            drop(live);
            self.ensure_ready().await?;
            live = self.live.lock().await;
        }

        let outcome = {
            let session = live
                .as_mut()
                .ok_or_else(|| anyhow!("MCP server `{}` has no live session", self.config.id))?;
            session.call_tool(name, arguments).await
        };

        match outcome {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                let message = error.to_string();
                live.take();
                drop(live);
                self.forget_session(message.clone()).await;
                Err(anyhow!(
                    "MCP tool `{name}` on server `{}` failed at the transport: {message}",
                    self.config.id
                ))
            }
        }
    }

    /// Round-trip check: the one operation the supervisor retries.
    ///
    /// `ping` has no effect, so a lost response is safe to repeat: one transport
    /// failure is followed by one reconnect and one more `ping`. A JSON-RPC error
    /// reply is **not** retried — the server answered, and that answer is the
    /// failure.
    pub async fn ping(&self) -> Result<()> {
        self.ensure_ready().await?;
        match self.ping_once().await {
            Ok(()) => Ok(()),
            Err(error) if error.downcast_ref::<JsonRpcError>().is_some() => Err(error),
            Err(first) => {
                // `ping_once` forgot the dead session, so this reconnects.
                if let Err(reconnect) = self.ensure_ready().await {
                    return Err(anyhow!(
                        "MCP server `{}`: ping failed ({first}) and the reconnect failed: {reconnect}",
                        self.config.id
                    ));
                }
                self.ping_once().await.map_err(|second| {
                    anyhow!(
                        "MCP server `{}` failed ping twice: first {first}; after reconnect {second}",
                        self.config.id
                    )
                })
            }
        }
    }

    /// One `ping` round trip against the live session.
    async fn ping_once(&self) -> Result<()> {
        let mut live = self.live.lock().await;
        if live.is_none() {
            drop(live);
            self.ensure_ready().await?;
            live = self.live.lock().await;
        }

        let outcome = {
            let session = live
                .as_mut()
                .ok_or_else(|| anyhow!("MCP server `{}` has no live session", self.config.id))?;
            session.ping().await
        };

        match outcome {
            Ok(()) => Ok(()),
            Err(error) => {
                // An error reply means the server is alive and answered; only a
                // wire failure costs the session.
                if error.downcast_ref::<JsonRpcError>().is_none() {
                    live.take();
                    drop(live);
                    self.forget_session(error.to_string()).await;
                }
                Err(error)
            }
        }
    }

    /// Record that the session is gone, and why, so the next call reconnects and
    /// the inspector shows the reason instead of a stale handshake.
    async fn forget_session(&self, message: String) {
        let mut inner = self.inner.lock().await;
        inner.state = ServerState::Restarting;
        inner.facts = None;
        inner.tools.clear();
        inner.last_error = Some(message);
    }
}

/// The set of configured servers.
#[derive(Debug, Default)]
pub struct McpRegistry {
    slots: Vec<Arc<ServerSlot>>,
}

impl McpRegistry {
    /// Build a registry from configuration. Disabled servers are kept so the
    /// inspector can show them as disabled rather than hiding them.
    pub fn new(configs: impl IntoIterator<Item = McpServerConfig>, client: ClientInfo) -> Self {
        let slots = configs
            .into_iter()
            .map(|config| Arc::new(ServerSlot::new(config, client.clone())))
            .collect();
        Self { slots }
    }

    /// Load the default configuration file and build a registry.
    ///
    /// A missing file is not an error: most installations have no external
    /// servers, and inventing one would be worse than having none.
    pub fn from_default(client: ClientInfo) -> Result<Self> {
        match McpServersFile::load_default()? {
            Some(file) => Ok(Self::new(file.servers, client)),
            None => Ok(Self::default()),
        }
    }

    /// Every supervised server.
    pub fn slots(&self) -> &[Arc<ServerSlot>] {
        &self.slots
    }

    /// Look up a slot by configured id.
    pub fn slot(&self, id: &str) -> Option<&Arc<ServerSlot>> {
        self.slots.iter().find(|slot| slot.id() == id)
    }

    /// Connect every eager server.
    ///
    /// Failures do not abort the others: one broken server must not prevent the
    /// rest of the workspace from starting. Each failure is retained in its
    /// slot's status for the inspector.
    pub async fn start_eager(&self) {
        for slot in &self.slots {
            if slot.config.startup == StartupPolicy::Eager && slot.config.enabled {
                let _ = slot.ensure_ready().await;
            }
        }
    }

    /// Adapters for every tool on every **ready** server.
    ///
    /// Tools from a server that is configured but not connected are not listed:
    /// advertising a tool that cannot run is the behaviour this repository
    /// treats as a stub.
    pub async fn remote_tools(&self) -> Vec<Arc<RemoteTool>> {
        let mut tools = Vec::new();
        for slot in &self.slots {
            if slot.state().await == ServerState::Ready {
                tools.extend(slot.remote_tools().await);
            }
        }
        tools
    }

    /// Find a remote tool by its namespaced id.
    pub async fn resolve(&self, namespaced_id: &str) -> Option<Arc<RemoteTool>> {
        self.remote_tools()
            .await
            .into_iter()
            .find(|tool| tool.id() == namespaced_id)
    }

    /// Status of every server, in configuration order.
    pub async fn snapshot(&self) -> Vec<ServerSnapshot> {
        let mut snapshots = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            snapshots.push(slot.snapshot().await);
        }
        snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_client::config::RestartPolicy;

    fn client() -> ClientInfo {
        ClientInfo {
            name: "zylcode-test".to_string(),
            version: "0.0.0".to_string(),
        }
    }

    #[test]
    fn disabled_server_starts_disabled_and_says_so() {
        let mut config = McpServerConfig::stdio("disabled-one", "does-not-matter.exe");
        config.enabled = false;
        let slot = ServerSlot::new(config, client());
        assert_eq!(
            futures::executor::block_on(slot.state()),
            ServerState::Disabled
        );
    }

    #[tokio::test]
    async fn enabled_server_that_cannot_start_reports_failed_without_hanging() {
        let mut config = McpServerConfig::stdio(
            "missing-binary",
            "definitely-not-a-real-binary-zylcode-test",
        );
        // One attempt, no meaningful backoff: this test is about the state
        // machine, not about patience.
        config.restart = RestartPolicy {
            max_attempts: 1,
            initial_backoff_ms: 1,
            max_backoff_ms: 1,
        };
        let slot = ServerSlot::new(config, client());
        let error = slot.ensure_ready().await.expect_err("must fail to connect");
        assert!(error.to_string().contains("missing-binary"));
        let snapshot = slot.snapshot().await;
        assert_eq!(snapshot.state, ServerState::Failed);
        assert_eq!(snapshot.attempts, 1);
        assert!(
            snapshot.last_error.is_some(),
            "a failure must be retained for the inspector"
        );
        assert!(
            snapshot.protocol_version.is_none(),
            "a server that never handshook must not report a protocol version"
        );
        assert!(!snapshot.is_ready());
    }

    #[tokio::test]
    async fn disabled_server_refuses_to_connect() {
        let mut config = McpServerConfig::stdio("disabled-two", "cmd.exe");
        config.enabled = false;
        let slot = ServerSlot::new(config, client());
        let error = slot
            .ensure_ready()
            .await
            .expect_err("disabled must not run");
        assert!(error.to_string().contains("disabled"));
    }

    #[tokio::test]
    async fn registry_without_servers_is_empty_and_quietly_valid() {
        let registry = McpRegistry::new(Vec::new(), client());
        assert!(registry.slots().is_empty());
        assert!(registry.remote_tools().await.is_empty());
        assert!(registry.snapshot().await.is_empty());
        registry.start_eager().await;
    }
}
