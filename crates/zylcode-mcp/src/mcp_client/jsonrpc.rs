//! JSON-RPC 2.0 envelopes for the MCP **client** side.
//!
//! ZylCode's `zylcode-mcp` crate is named for MCP but, until this module, had no
//! protocol in it: tools were in-process executors registered from
//! `mcp.tools.yaml`, and `McpBridgeDescriptor` was a stored string triple that
//! nothing connected to. This module is the wire format half of the fix.
//!
//! Scope, stated plainly so it cannot be mistaken for more:
//!
//! * one JSON-RPC 2.0 message per line/body (the MCP spec removed batching in
//!   2025-06-18, so requests are never batched here);
//! * request ids are correlated, result and error are distinct types, and a
//!   malformed message is a typed error rather than a silent skip;
//! * server → client requests are recognised and surfaced. The client does not
//!   implement every server request (sampling, roots, elicitation) — it answers
//!   `-32601` method-not-found, which is the protocol's defined way to say "I do
//!   not implement this" rather than pretending.
//!
//! The protocol *version* is negotiated in [`super::session`], not here, because
//! it is a lifecycle concern.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

/// The only JSON-RPC version MCP uses.
pub const JSONRPC_VERSION: &str = "2.0";

// JSON-RPC 2.0 error codes that this client reacts to specifically. The rest
// are carried through verbatim: re-labelling a server's error would lose the
// only diagnostic the user has.
/// Method not found.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// Invalid params.
pub const INVALID_PARAMS: i64 = -32602;
/// Internal error.
pub const INTERNAL_ERROR: i64 = -32603;
/// The MCP "modern" revisions (2026-07-28 and later) reject a request whose
/// protocol version they do not implement with this code, listing the versions
/// they do support. A handshake-era client can only report it.
pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;

/// A JSON-RPC request id.
///
/// The spec permits numbers and strings. Both are accepted on the way in;
/// outbound ids are always numbers, because this client issues them from a
/// counter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric id, as this client issues.
    Number(u64),
    /// String id, which a server may use when it calls the client.
    Text(String),
}

impl RequestId {
    /// A numeric id.
    pub fn number(n: u64) -> Self {
        Self::Number(n)
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Text(s) => write!(f, "{s}"),
        }
    }
}

/// An outbound JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Always [`JSONRPC_VERSION`].
    pub jsonrpc: String,
    /// Correlates the response.
    pub id: RequestId,
    /// The MCP method, e.g. `tools/call`.
    pub method: String,
    /// Method parameters. Absent rather than `null` when there are none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Build a request.
    pub fn new(id: RequestId, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC notification: no id, no response expected.
pub fn notification(method: impl Into<String>, params: Option<Value>) -> Value {
    let mut body = json!({ "jsonrpc": JSONRPC_VERSION, "method": method.into() });
    if let Some(p) = params {
        body["params"] = p;
    }
    body
}

/// A JSON-RPC error object as returned by a server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i64,
    /// Human-readable message.
    pub message: String,
    /// Optional structured detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// True for `-32601`, which is how a server says it does not implement a
    /// method — including one this client sends during discovery against an
    /// older server.
    pub fn is_method_not_found(&self) -> bool {
        self.code == METHOD_NOT_FOUND
    }

    /// True when a server rejected the *protocol version* rather than the request.
    pub fn is_unsupported_protocol_version(&self) -> bool {
        self.code == UNSUPPORTED_PROTOCOL_VERSION
    }

    /// The versions a "modern" server says it supports, when it says so.
    pub fn supported_versions(&self) -> Option<Vec<String>> {
        self.data
            .as_ref()
            .and_then(|d| d.get("supported"))
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// A message received from a server.
#[derive(Debug, Clone)]
pub enum Inbound {
    /// The answer to a request this client sent.
    Response {
        /// Which request it answers.
        id: RequestId,
        /// `Ok(result)` or `Err(error)`; JSON-RPC makes these exclusive.
        outcome: Result<Value, JsonRpcError>,
    },
    /// A request the server sent to this client. Answered with `-32601` unless
    /// a handler exists.
    ServerRequest {
        /// Server-chosen id, echoed in the reply.
        id: RequestId,
        /// Requested method.
        method: String,
        /// Requested parameters.
        params: Option<Value>,
    },
    /// A notification: no reply.
    Notification {
        /// Notification method.
        method: String,
        /// Notification parameters.
        params: Option<Value>,
    },
}

/// Why a line could not be understood as JSON-RPC.
#[derive(Debug)]
pub enum ParseError {
    /// Not JSON.
    InvalidJson(serde_json::Error),
    /// JSON, but not an object.
    NotAnObject,
    /// An object that carries none of the shapes JSON-RPC defines.
    UnrecognisedShape(Value),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(e) => write!(f, "not valid JSON: {e}"),
            Self::NotAnObject => write!(f, "JSON-RPC message must be an object"),
            Self::UnrecognisedShape(v) => {
                write!(f, "not a JSON-RPC request, response or notification: {v}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

fn parse_id(value: &Value) -> Option<RequestId> {
    match value {
        Value::Number(n) => n.as_u64().map(RequestId::Number),
        Value::String(s) => Some(RequestId::Text(s.clone())),
        _ => None,
    }
}

/// Parse one inbound line into a message.
pub fn parse_inbound(raw: &str) -> Result<Inbound, ParseError> {
    let value: Value = serde_json::from_str(raw).map_err(ParseError::InvalidJson)?;
    classify(value)
}

/// Classify an already-parsed JSON value.
///
/// A response is identified by an `id` **and** exactly one of `result`/`error`.
/// A request has `id` and `method`; a notification has `method` and no `id`.
/// Anything else is an error: skipping it would make a protocol bug look like a
/// quiet network hiccup.
pub fn classify(value: Value) -> Result<Inbound, ParseError> {
    let obj = value.as_object().ok_or(ParseError::NotAnObject)?;

    let id = obj.get("id").and_then(parse_id);
    let method = obj.get("method").and_then(Value::as_str);

    match (method, id) {
        (Some(method), Some(id)) => Ok(Inbound::ServerRequest {
            id,
            method: method.to_string(),
            params: obj.get("params").cloned(),
        }),
        (Some(method), None) => Ok(Inbound::Notification {
            method: method.to_string(),
            params: obj.get("params").cloned(),
        }),
        (None, Some(id)) => {
            let has_result = obj.contains_key("result");
            let has_error = obj.contains_key("error");
            match (has_result, has_error) {
                (true, false) => Ok(Inbound::Response {
                    id,
                    outcome: Ok(obj.get("result").cloned().unwrap_or(Value::Null)),
                }),
                (false, true) => {
                    let err: JsonRpcError =
                        serde_json::from_value(obj.get("error").cloned().unwrap_or(Value::Null))
                            .map_err(|_| ParseError::UnrecognisedShape(value.clone()))?;
                    Ok(Inbound::Response {
                        id,
                        outcome: Err(err),
                    })
                }
                // Both or neither: JSON-RPC forbids this.
                _ => Err(ParseError::UnrecognisedShape(value)),
            }
        }
        (None, None) => Err(ParseError::UnrecognisedShape(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serialises_with_jsonrpc_and_id() {
        let r = JsonRpcRequest::new(RequestId::number(1), "tools/list", None);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["method"], "tools/list");
        assert!(
            v.get("params").is_none(),
            "params must be omitted, not null"
        );
    }

    #[test]
    fn notification_has_no_id() {
        let n = notification("notifications/initialized", None);
        assert_eq!(n["jsonrpc"], "2.0");
        assert_eq!(n["method"], "notifications/initialized");
        assert!(n.get("id").is_none());
    }

    #[test]
    fn parses_a_result_response() {
        let msg = parse_inbound(r#"{"jsonrpc":"2.0","id":7,"result":{"tools":[]}}"#).unwrap();
        match msg {
            Inbound::Response { id, outcome } => {
                assert_eq!(id, RequestId::number(7));
                assert_eq!(outcome.unwrap()["tools"], serde_json::json!([]));
            }
            other => panic!("expected a response, got {other:?}"),
        }
    }

    #[test]
    fn parses_an_error_response_and_classifies_method_not_found() {
        let msg =
            parse_inbound(r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"nope"}}"#)
                .unwrap();
        match msg {
            Inbound::Response {
                outcome: Err(e), ..
            } => {
                assert!(e.is_method_not_found());
                assert_eq!(e.to_string(), "JSON-RPC error -32601: nope");
            }
            other => panic!("expected an error response, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_string_id_response() {
        let msg = parse_inbound(r#"{"jsonrpc":"2.0","id":"abc","result":true}"#).unwrap();
        match msg {
            Inbound::Response { id, .. } => assert_eq!(id, RequestId::Text("abc".into())),
            other => panic!("expected a response, got {other:?}"),
        }
    }

    #[test]
    fn distinguishes_server_requests_from_notifications() {
        let req = parse_inbound(
            r#"{"jsonrpc":"2.0","id":9,"method":"sampling/createMessage","params":{}}"#,
        )
        .unwrap();
        assert!(
            matches!(req, Inbound::ServerRequest { ref method, .. } if method == "sampling/createMessage")
        );

        let note =
            parse_inbound(r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#)
                .unwrap();
        assert!(
            matches!(note, Inbound::Notification { ref method, .. } if method == "notifications/tools/list_changed")
        );
    }

    #[test]
    fn rejects_a_response_with_both_result_and_error() {
        let err = parse_inbound(
            r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1,"message":"x"}}"#,
        )
        .unwrap_err();
        assert!(matches!(err, ParseError::UnrecognisedShape(_)), "{err}");
    }

    #[test]
    fn rejects_non_json_and_non_objects() {
        assert!(matches!(
            parse_inbound("not json").unwrap_err(),
            ParseError::InvalidJson(_)
        ));
        assert!(matches!(
            parse_inbound("[]").unwrap_err(),
            ParseError::NotAnObject
        ));
        assert!(matches!(
            parse_inbound(r#"{"jsonrpc":"2.0"}"#).unwrap_err(),
            ParseError::UnrecognisedShape(_)
        ));
    }

    #[test]
    fn reads_a_modern_servers_supported_version_list() {
        let e = JsonRpcError {
            code: UNSUPPORTED_PROTOCOL_VERSION,
            message: "Unsupported protocol version".into(),
            data: Some(
                json!({"supported": ["2026-07-28", "2025-11-25"], "requested": "2025-06-18"}),
            ),
        };
        assert!(e.is_unsupported_protocol_version());
        assert_eq!(
            e.supported_versions().unwrap(),
            vec!["2026-07-28".to_string(), "2025-11-25".to_string()]
        );
    }
}
