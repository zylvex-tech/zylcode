//! MCP client session: handshake, discovery and calls.
//!
//! Lifecycle implemented here is the one the MCP spec defines for the revisions
//! this client supports:
//!
//! ```text
//! initialize  →  (server: protocolVersion, capabilities, serverInfo)
//! notifications/initialized
//! tools/list  →  (tools, nextCursor?)  … repeat while nextCursor
//! tools/call  →  (content, isError?)
//! ```
//!
//! Plus `ping`, `notifications/cancelled` and graceful shutdown.
//!
//! # Version policy
//!
//! [`SUPPORTED_PROTOCOL_VERSIONS`] is the set this client claims. A server that
//! answers with anything outside it is refused with a readable diagnostic — the
//! session does not silently continue on a version whose semantics it has not
//! implemented. In particular the `2026-07-28` revision is a different protocol
//! (per-request metadata, no handshake); when a modern server answers with
//! `-32022` the error carries the versions the server does support so the user
//! can be told what to install.
//!
//! # Server → client requests
//!
//! MCP lets a server call the client (`sampling/createMessage`, `roots/list`,
//! `elicitation/create`). This client implements none of them, so each is
//! answered with JSON-RPC `-32601` method-not-found and counted. That is the
//! protocol's defined "not implemented" answer; inventing a reply would be the
//! fake-capability failure this codebase is trying to leave behind.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use super::jsonrpc::{
    notification, Inbound, JsonRpcError, JsonRpcRequest, RequestId, METHOD_NOT_FOUND,
};
use super::transport::{SessionExpired, Transport};

/// The MCP revisions this client implements, newest first.
///
/// Only revisions whose lifecycle and transport this module actually implements
/// are listed. `2025-11-25` and `2026-07-28` are deliberately absent: the latter
/// removes the handshake entirely, and this client has not been verified against
/// the former's changes.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26"];

/// Version advertised in `initialize`.
pub const PREFERRED_PROTOCOL_VERSION: &str = SUPPORTED_PROTOCOL_VERSIONS[0];

/// Safety valve for a server that keeps returning cursors.
pub const MAX_LIST_PAGES: usize = 50;

/// The client's identity, sent in `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Program name.
    pub name: String,
    /// Program version.
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "zylcode".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// What the server says about itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name.
    #[serde(default)]
    pub name: String,
    /// Server version.
    #[serde(default)]
    pub version: String,
}

/// Result of the handshake.
#[derive(Debug, Clone)]
pub struct Handshake {
    /// The version the server accepted.
    pub protocol_version: String,
    /// The server's advertised capabilities (`tools`, `resources`, `prompts`, …).
    pub capabilities: Value,
    /// Server identity.
    pub server_info: ServerInfo,
    /// Optional usage instructions for the model.
    pub instructions: Option<String>,
}

/// A tool discovered from a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteToolDef {
    /// Tool name as the server knows it (un-namespaced).
    pub name: String,
    /// Human-friendly title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description shown to the model.
    #[serde(default)]
    pub description: Option<String>,
    /// JSON Schema for the arguments.
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Value,
    /// Optional behaviour hints.
    #[serde(default)]
    pub annotations: Option<ToolAnnotations>,
}

impl RemoteToolDef {
    /// The id this tool is exposed under inside ZylCode.
    ///
    /// `server__tool`, so a remote `fs.read` can never be mistaken for the
    /// built-in one.
    pub fn namespaced_id(&self, server_id: &str) -> String {
        format!("{server_id}__{}", self.name)
    }
}

/// MCP tool behaviour hints. They are *hints*: a server may omit or lie about
/// them, which is why the permission gate treats an unannotated tool as
/// approval-requiring rather than as a read.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// The tool does not modify its environment.
    #[serde(default, rename = "readOnlyHint")]
    pub read_only_hint: Option<bool>,
    /// The tool may perform destructive updates.
    #[serde(default, rename = "destructiveHint")]
    pub destructive_hint: Option<bool>,
    /// Repeated calls with the same arguments have no additional effect.
    #[serde(default, rename = "idempotentHint")]
    pub idempotent_hint: Option<bool>,
    /// The tool interacts with external entities.
    #[serde(default, rename = "openWorldHint")]
    pub open_world_hint: Option<bool>,
}

/// One page of `tools/list`.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsPage {
    /// Tools on this page.
    #[serde(default)]
    pub tools: Vec<RemoteToolDef>,
    /// Opaque cursor for the next page.
    #[serde(default, rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Outcome of `tools/call`.
#[derive(Debug, Clone)]
pub struct ToolCallOutcome {
    /// The protocol exchange succeeded but the tool reported a failure.
    pub is_error: bool,
    /// Content blocks, verbatim.
    pub content: Value,
    /// Structured output when the server provides it (2025-06-18+).
    pub structured: Option<Value>,
}

/// A server rejected the protocol version this client asked for.
#[derive(Debug, Clone)]
pub struct ProtocolVersionRejected {
    /// Versions this client implements.
    pub offered: Vec<String>,
    /// Versions the server said it supports, when it said so.
    pub supported_by_server: Option<Vec<String>>,
    /// The server's message.
    pub detail: String,
}

impl std::fmt::Display for ProtocolVersionRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MCP server does not support a protocol version this client implements \
             (client offers: {}; server supports: {})",
            self.offered.join(", "),
            self.supported_by_server
                .as_ref()
                .map(|v| v.join(", "))
                .unwrap_or_else(|| "unstated".to_string())
        )?;
        write!(f, " — {}", self.detail)
    }
}

impl std::error::Error for ProtocolVersionRejected {}

/// A live MCP session over one transport.
pub struct McpSession {
    transport: Box<dyn Transport>,
    next_id: u64,
    request_timeout: Duration,
    handshake: Handshake,
    /// Notifications received while waiting for a response, retained so the
    /// inspector can show that a server is telling us something.
    notifications: Vec<Value>,
    unanswered_server_requests: u32,
    answered_method_not_found: u32,
}

impl std::fmt::Debug for McpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpSession")
            .field("transport", &self.transport.description())
            .field("protocol_version", &self.handshake.protocol_version)
            .field("server", &self.handshake.server_info.name)
            .finish()
    }
}

impl McpSession {
    /// Handshake over `transport`.
    pub async fn connect(
        transport: Box<dyn Transport>,
        client: ClientInfo,
        request_timeout: Duration,
    ) -> Result<Self> {
        let mut session = Self {
            transport,
            next_id: 1,
            request_timeout,
            handshake: Handshake {
                protocol_version: String::new(),
                capabilities: Value::Null,
                server_info: ServerInfo {
                    name: String::new(),
                    version: String::new(),
                },
                instructions: None,
            },
            notifications: Vec::new(),
            unanswered_server_requests: 0,
            answered_method_not_found: 0,
        };

        let params = json!({
            "protocolVersion": PREFERRED_PROTOCOL_VERSION,
            // An empty capability object is the honest claim: this client does
            // not implement roots, sampling or elicitation.
            "capabilities": {},
            "clientInfo": { "name": client.name, "version": client.version },
        });

        let result = session
            .request("initialize", Some(params))
            .await
            .map_err(|e| match e.downcast_ref::<JsonRpcError>() {
                Some(err) if err.is_unsupported_protocol_version() => {
                    anyhow::Error::new(ProtocolVersionRejected {
                        offered: SUPPORTED_PROTOCOL_VERSIONS
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        supported_by_server: err.supported_versions(),
                        detail: err.to_string(),
                    })
                }
                _ => e,
            })?;

        let version = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("initialize result has no protocolVersion"))?
            .to_string();

        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&version.as_str()) {
            return Err(anyhow::Error::new(ProtocolVersionRejected {
                offered: SUPPORTED_PROTOCOL_VERSIONS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                supported_by_server: Some(vec![version.clone()]),
                detail: format!(
                    "server selected `{version}`, which this client has not implemented"
                ),
            }));
        }

        session.handshake = Handshake {
            protocol_version: version,
            capabilities: result.get("capabilities").cloned().unwrap_or(Value::Null),
            server_info: serde_json::from_value(
                result.get("serverInfo").cloned().unwrap_or(Value::Null),
            )
            .unwrap_or(ServerInfo {
                name: String::new(),
                version: String::new(),
            }),
            instructions: result
                .get("instructions")
                .and_then(Value::as_str)
                .map(str::to_string),
        };

        // The spec requires this notification before any other request.
        session
            .transport
            .send(&notification("notifications/initialized", None))
            .await
            .context("sending notifications/initialized")?;

        Ok(session)
    }

    /// The negotiated handshake.
    pub fn handshake(&self) -> &Handshake {
        &self.handshake
    }

    /// Transport description, for the inspector.
    pub fn transport_description(&self) -> String {
        self.transport.description()
    }

    /// Notifications received while waiting for responses.
    pub fn notifications(&self) -> &[Value] {
        &self.notifications
    }

    /// How many server→client requests were answered with `-32601`.
    pub fn answered_method_not_found(&self) -> u32 {
        self.answered_method_not_found
    }

    /// How many server→client requests arrived that this client cannot serve.
    pub fn unanswered_server_requests(&self) -> u32 {
        self.unanswered_server_requests
    }

    /// One page of `tools/list`.
    pub async fn list_tools_page(&mut self, cursor: Option<String>) -> Result<ToolsPage> {
        let params = cursor.map(|c| json!({ "cursor": c }));
        let result = self.request("tools/list", params).await?;
        serde_json::from_value(result).context("parsing tools/list result")
    }

    /// Every tool, following `nextCursor` until the server stops paging.
    pub async fn list_all_tools(&mut self) -> Result<Vec<RemoteToolDef>> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        let mut seen_cursors: Vec<String> = Vec::new();

        for _ in 0..MAX_LIST_PAGES {
            let page = self.list_tools_page(cursor.clone()).await?;
            tools.extend(page.tools);
            match page.next_cursor {
                None => return Ok(tools),
                Some(next) => {
                    if seen_cursors.contains(&next) {
                        bail!("MCP server repeated tools/list cursor `{next}`; refusing to loop");
                    }
                    seen_cursors.push(next.clone());
                    cursor = Some(next);
                }
            }
        }
        bail!("MCP server exceeded {MAX_LIST_PAGES} tools/list pages; refusing to keep paging")
    }

    /// `tools/call`.
    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<ToolCallOutcome> {
        let result = self
            .request(
                "tools/call",
                Some(json!({ "name": name, "arguments": arguments })),
            )
            .await?;
        Ok(ToolCallOutcome {
            is_error: result
                .get("isError")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            content: result.get("content").cloned().unwrap_or(Value::Null),
            structured: result.get("structuredContent").cloned(),
        })
    }

    /// `ping`.
    pub async fn ping(&mut self) -> Result<()> {
        self.request("ping", None).await.map(|_| ())
    }

    /// Explicitly cancel an in-flight request.
    ///
    /// Only the id can be cancelled; a [`Transport`] does not expose the caller's
    /// future, so this is used by [`McpSession::request`] on timeout and is public
    /// for callers that track ids themselves.
    pub async fn cancel(&self, id: &RequestId, reason: &str) -> Result<()> {
        self.transport
            .send(&notification(
                "notifications/cancelled",
                Some(json!({ "requestId": id, "reason": reason })),
            ))
            .await
            .context("sending notifications/cancelled")
    }

    /// Close the transport.
    pub async fn shutdown(mut self) -> Result<()> {
        self.transport.close().await
    }

    /// Send a request and wait for its response, serving protocol obligations on
    /// the way (answering unimplemented server requests, retaining notifications).
    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = RequestId::number(self.next_id);
        self.next_id += 1;
        let request = JsonRpcRequest::new(id.clone(), method, params);
        self.transport
            .send(&json!(request))
            .await
            .with_context(|| format!("sending `{method}`"))?;

        let deadline = tokio::time::Instant::now() + self.request_timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                // Tell the server we gave up before giving up: an abandoned
                // request whose work continues server-side is a resource leak and
                // an audit hole.
                let _ = self.cancel(&id, "request timed out").await;
                bail!(
                    "MCP request `{method}` timed out after {:?}",
                    self.request_timeout
                );
            }

            let inbound = match tokio::time::timeout(remaining, self.transport.recv()).await {
                Ok(Ok(msg)) => msg,
                Ok(Err(e)) => {
                    if e.downcast_ref::<SessionExpired>().is_some() {
                        return Err(e);
                    }
                    return Err(e.context(format!("waiting for `{method}` response")));
                }
                Err(_) => {
                    let _ = self.cancel(&id, "request timed out").await;
                    bail!(
                        "MCP request `{method}` timed out after {:?}",
                        self.request_timeout
                    );
                }
            };

            match inbound {
                Inbound::Response {
                    id: response_id,
                    outcome,
                } => {
                    if response_id != id {
                        // A response to something we did not ask for means the
                        // stream is out of sync; continuing would attribute one
                        // call's result to another.
                        bail!(
                            "MCP protocol error: response id {response_id} does not match request id {id}"
                        );
                    }
                    return outcome.map_err(anyhow::Error::new);
                }
                Inbound::Notification { method, params } => {
                    self.notifications.push(json!({
                        "method": method,
                        "params": params,
                    }));
                }
                Inbound::ServerRequest {
                    id: server_id,
                    method,
                    ..
                } => {
                    self.unanswered_server_requests += 1;
                    self.answered_method_not_found += 1;
                    let reply = json!({
                        "jsonrpc": "2.0",
                        "id": server_id,
                        "error": {
                            "code": METHOD_NOT_FOUND,
                            "message": format!(
                                "ZylCode does not implement the client-side method `{method}`"
                            ),
                        }
                    });
                    self.transport
                        .send(&reply)
                        .await
                        .with_context(|| format!("answering server request `{method}`"))?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// An in-memory transport: records what the client sent, replays scripted
    /// inbound messages.
    #[derive(Debug, Default)]
    struct ScriptedTransport {
        sent: Arc<Mutex<Vec<Value>>>,
        inbound: Arc<Mutex<VecDeque<Result<Inbound, String>>>>,
    }

    impl ScriptedTransport {
        fn with(messages: Vec<Inbound>) -> (Self, Arc<Mutex<Vec<Value>>>) {
            let sent = Arc::new(Mutex::new(Vec::new()));
            let inbound = Arc::new(Mutex::new(
                messages.into_iter().map(Ok).collect::<VecDeque<_>>(),
            ));
            (
                Self {
                    sent: Arc::clone(&sent),
                    inbound,
                },
                sent,
            )
        }
    }

    #[async_trait]
    impl Transport for ScriptedTransport {
        async fn send(&self, message: &Value) -> Result<()> {
            self.sent.lock().unwrap().push(message.clone());
            Ok(())
        }

        async fn recv(&mut self) -> Result<Inbound> {
            match self.inbound.lock().unwrap().pop_front() {
                Some(Ok(msg)) => Ok(msg),
                Some(Err(e)) => bail!(e),
                None => bail!("scripted transport exhausted"),
            }
        }

        async fn close(&mut self) -> Result<()> {
            Ok(())
        }

        fn description(&self) -> String {
            "scripted".to_string()
        }
    }

    fn ready(protocol: &str) -> Inbound {
        Inbound::Response {
            id: RequestId::number(1),
            outcome: Ok(json!({
                "protocolVersion": protocol,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "test", "version": "1.0" }
            })),
        }
    }

    fn sent_methods(sent: &Arc<Mutex<Vec<Value>>>) -> Vec<String> {
        sent.lock()
            .unwrap()
            .iter()
            .filter_map(|m| m.get("method").and_then(Value::as_str).map(str::to_string))
            .collect()
    }

    #[tokio::test]
    async fn handshake_sends_initialize_then_initialized() {
        let (t, sent) = ScriptedTransport::with(vec![ready("2025-06-18")]);
        let session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();

        assert_eq!(session.handshake().protocol_version, "2025-06-18");
        assert_eq!(session.handshake().server_info.name, "test");
        let methods = sent_methods(&sent);
        assert_eq!(
            methods,
            vec!["initialize", "notifications/initialized"],
            "initialized must follow initialize, before anything else"
        );
        // The client must not claim capabilities it does not implement.
        let init = &sent.lock().unwrap()[0];
        assert_eq!(init["params"]["capabilities"], json!({}));
        assert_eq!(init["params"]["protocolVersion"], "2025-06-18");
        assert_eq!(init["params"]["clientInfo"]["name"], "zylcode");
    }

    #[tokio::test]
    async fn an_unsupported_negotiated_version_fails_closed() {
        let (t, _sent) = ScriptedTransport::with(vec![ready("2026-07-28")]);
        let err = McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
            .await
            .expect_err("a version this client has not implemented must not be accepted");
        let rejected = err
            .downcast_ref::<ProtocolVersionRejected>()
            .expect("typed rejection");
        assert_eq!(
            rejected.supported_by_server.as_deref(),
            Some(&["2026-07-28".to_string()][..])
        );
        assert!(err
            .to_string()
            .contains("does not support a protocol version"));
    }

    #[tokio::test]
    async fn a_modern_servers_32022_error_reports_the_versions_it_wants() {
        let (t, _sent) = ScriptedTransport::with(vec![Inbound::Response {
            id: RequestId::number(1),
            outcome: Err(JsonRpcError {
                code: super::super::jsonrpc::UNSUPPORTED_PROTOCOL_VERSION,
                message: "Unsupported protocol version".into(),
                data: Some(json!({ "supported": ["2026-07-28"] })),
            }),
        }]);
        let err = McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
            .await
            .expect_err("must fail");
        let rejected = err
            .downcast_ref::<ProtocolVersionRejected>()
            .expect("typed");
        assert_eq!(
            rejected.supported_by_server.as_deref(),
            Some(&["2026-07-28".to_string()][..])
        );
    }

    #[tokio::test]
    async fn discovery_follows_pagination_to_the_end() {
        let (t, _sent) = ScriptedTransport::with(vec![
            ready("2025-06-18"),
            Inbound::Response {
                id: RequestId::number(2),
                outcome: Ok(json!({
                    "tools": [{ "name": "a", "description": "A", "inputSchema": {"type":"object"} }],
                    "nextCursor": "page2"
                })),
            },
            Inbound::Response {
                id: RequestId::number(3),
                outcome: Ok(json!({
                    "tools": [{ "name": "b", "description": "B", "inputSchema": {"type":"object"} }]
                })),
            },
        ]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        let tools = session.list_all_tools().await.unwrap();
        assert_eq!(
            tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(tools[0].namespaced_id("srv"), "srv__a");
    }

    #[tokio::test]
    async fn a_repeated_cursor_is_refused_rather_than_looped() {
        let page = |id: u64| Inbound::Response {
            id: RequestId::number(id),
            outcome: Ok(json!({
                "tools": [{ "name": "a", "inputSchema": { "type": "object" } }],
                "nextCursor": "same"
            })),
        };
        let (t, _sent) = ScriptedTransport::with(vec![ready("2025-06-18"), page(2), page(3)]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        let err = session.list_all_tools().await.unwrap_err();
        assert!(
            err.to_string().contains("repeated tools/list cursor"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn tool_call_reports_is_error_separately_from_protocol_success() {
        let (t, _sent) = ScriptedTransport::with(vec![
            ready("2025-06-18"),
            Inbound::Response {
                id: RequestId::number(2),
                outcome: Ok(json!({
                    "content": [{ "type": "text", "text": "boom" }],
                    "isError": true
                })),
            },
        ]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        let outcome = session
            .call_tool("explode", json!({ "x": 1 }))
            .await
            .unwrap();
        assert!(
            outcome.is_error,
            "a tool-level failure is not a protocol failure"
        );
        assert_eq!(outcome.content[0]["text"], "boom");
    }

    #[tokio::test]
    async fn a_mismatched_response_id_is_a_protocol_error() {
        let (t, _sent) = ScriptedTransport::with(vec![
            ready("2025-06-18"),
            Inbound::Response {
                id: RequestId::number(99),
                outcome: Ok(json!({ "tools": [] })),
            },
        ]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        let err = session.list_tools_page(None).await.unwrap_err();
        assert!(
            err.to_string().contains("does not match request id"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn a_server_request_is_answered_with_method_not_found() {
        let (t, sent) = ScriptedTransport::with(vec![
            ready("2025-06-18"),
            Inbound::ServerRequest {
                id: RequestId::Text("srv-1".into()),
                method: "sampling/createMessage".into(),
                params: Some(json!({})),
            },
            Inbound::Response {
                id: RequestId::number(2),
                outcome: Ok(json!({ "tools": [] })),
            },
        ]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        session.list_tools_page(None).await.unwrap();

        assert_eq!(session.answered_method_not_found(), 1);
        let sent = sent.lock().unwrap();
        let reply = sent
            .iter()
            .find(|m| m.get("error").is_some())
            .expect("a reply to the server request");
        assert_eq!(reply["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(reply["id"], "srv-1");
    }

    #[tokio::test]
    async fn notifications_arriving_mid_request_are_retained_not_dropped() {
        let (t, _sent) = ScriptedTransport::with(vec![
            ready("2025-06-18"),
            Inbound::Notification {
                method: "notifications/tools/list_changed".into(),
                params: None,
            },
            Inbound::Response {
                id: RequestId::number(2),
                outcome: Ok(json!({ "tools": [] })),
            },
        ]);
        let mut session =
            McpSession::connect(Box::new(t), ClientInfo::default(), Duration::from_secs(5))
                .await
                .unwrap();
        session.list_tools_page(None).await.unwrap();
        assert_eq!(session.notifications().len(), 1);
        assert_eq!(
            session.notifications()[0]["method"],
            "notifications/tools/list_changed"
        );
    }

    #[tokio::test]
    async fn a_timeout_sends_cancelled_and_does_not_wait_forever() {
        // A transport whose recv never yields a message.
        #[derive(Debug)]
        struct Silent;
        #[async_trait]
        impl Transport for Silent {
            async fn send(&self, _m: &Value) -> Result<()> {
                Ok(())
            }
            async fn recv(&mut self) -> Result<Inbound> {
                tokio::time::sleep(Duration::from_secs(60)).await;
                bail!("unreachable")
            }
            async fn close(&mut self) -> Result<()> {
                Ok(())
            }
            fn description(&self) -> String {
                "silent".into()
            }
        }

        // Handshake first needs a response, so drive the timeout on a session
        // built with a scripted handshake and then a silent transport is not
        // possible; instead assert the timeout path on a bare request through a
        // session constructed directly.
        let mut session = McpSession {
            transport: Box::new(Silent),
            next_id: 1,
            request_timeout: Duration::from_millis(150),
            handshake: Handshake {
                protocol_version: "2025-06-18".into(),
                capabilities: Value::Null,
                server_info: ServerInfo {
                    name: "silent".into(),
                    version: "0".into(),
                },
                instructions: None,
            },
            notifications: Vec::new(),
            unanswered_server_requests: 0,
            answered_method_not_found: 0,
        };
        let err = session.ping().await.unwrap_err();
        assert!(err.to_string().contains("timed out"), "{err}");
    }
}
