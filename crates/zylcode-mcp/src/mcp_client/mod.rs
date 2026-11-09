//! A real MCP **client**: transports, JSON-RPC, session lifecycle and discovery.
//!
//! # Why this module exists
//!
//! Before this, `zylcode-mcp` contained a tool catalogue, in-process executors, a
//! permission gate and an evidence log — all genuinely useful — but **no MCP**.
//! `McpBridgeDescriptor` was stored in a `HashMap` and never connected to
//! anything; `transport: Stdio` in `mcp.tools.yaml` was a label, not a process.
//! This module is the protocol half.
//!
//! # Layers
//!
//! | Layer | Module | Responsibility |
//! | ----- | ------ | -------------- |
//! | Framing | [`jsonrpc`] | JSON-RPC 2.0 envelopes, id correlation, malformed-message errors |
//! | Wire | [`transport`] | stdio (spawn, frame, capture stderr) and Streamable HTTP (POST/GET, SSE, session id) |
//! | Lifecycle | [`session`] | `initialize` → `initialized` → `tools/list` → `tools/call`, `ping`, cancellation |
//! | Config | [`config`] | persisted `mcp.servers.json`, validated as a security boundary |
//! | Supervision | [`registry`] | start/restart/backoff, discovery cache, status snapshot for the inspector |
//!
//! Remote tools do **not** get a second execution framework. [`mod@remote`]
//! adapts a discovered tool onto [`crate::real_tools::dispatch_with`], so a
//! remote call passes the same [`crate::permission::PermissionGate`], the same
//! [`crate::real_tools::ToolEvidence`] and the same sink as `fs.read`.
//!
//! # Protocol revision, stated honestly
//!
//! Implemented: the handshake-era lifecycle for MCP `2025-06-18` and
//! `2025-03-26` ([`session::SUPPORTED_PROTOCOL_VERSIONS`]). Not implemented:
//! `2026-07-28`, which removed the handshake in favour of per-request metadata.
//! A server that only speaks it is refused with the versions it advertised, not
//! silently downgraded.
//!
//! # What this does not do yet
//!
//! * No `resources/*` or `prompts/*` — only tools are discovered.
//! * No client-side sampling, roots or elicitation: server requests for those are
//!   answered `-32601` and counted.
//! * OAuth is not implemented. A remote server needing it must be reached through
//!   a local gateway or configured with a static header.

pub mod config;
pub mod jsonrpc;
pub mod registry;
pub mod remote;
pub mod session;
pub mod transport;

pub use config::{
    McpServerConfig, McpServersFile, RestartPolicy, ServerTransport, StartupPolicy, CONFIG_ENV,
    CONFIG_FILENAME,
};
pub use jsonrpc::{Inbound, JsonRpcError, JsonRpcRequest, RequestId};
pub use registry::{McpRegistry, ServerSlot, ServerSnapshot, ServerState};
pub use remote::RemoteTool;
pub use session::{
    ClientInfo, Handshake, McpSession, ProtocolVersionRejected, RemoteToolDef, ServerInfo,
    ToolAnnotations, ToolCallOutcome, ToolsPage, PREFERRED_PROTOCOL_VERSION,
    SUPPORTED_PROTOCOL_VERSIONS,
};
pub use transport::{
    decode_sse, HttpOptions, SessionExpired, SseEvent, StdioOptions, StdioTransport,
    StreamableHttpTransport, Transport,
};
