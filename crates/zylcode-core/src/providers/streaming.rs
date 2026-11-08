//! Provider-neutral streaming contract.
//!
//! The old `StreamEvent` lived in `router.rs` and was only ever used by the fake
//! line-chunked fallback. A real provider streaming layer needs a contract that can
//! describe text deltas, provider-native tool-call deltas, usage, finish reason,
//! provider metadata, structured provider errors, and explicit synthetic/offline
//! provenance — without encoding any one provider's wire format back into the
//! neutral types.
//!
//! Provider modules translate their native stream shapes into [`StreamChunk`] and
//! hand them to the dispatcher callback. The dispatcher callback consumes neutral
//! chunks; it does not parse SSE, NDJSON, or any provider's tool block.
//!
//! # Cancellation
//!
//! [`CancelToken`] is the only cancellation mechanism. It is cooperative on the
//! consuming side *and* terminates the underlying HTTP operation: the transport
//! driver polls it alongside the response body, and on cancellation it drops the
//! `reqwest::Response` (closing the connection) before returning. Stopping
//! consumption is therefore never confused with aborting the request.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

/// One incremental piece of a provider response as seen by the dispatcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StreamChunk {
    /// Monotonic chunk index within one stream.
    pub index: u64,
    /// The kind of delta this chunk carries.
    pub kind: ChunkKind,
    /// Whether the stream has finished from the provider's perspective.
    pub done: bool,
    /// Why the provider ended the stream, when the provider said so.
    ///
    /// This is a first-class field rather than a `metadata` entry: a consumer
    /// deciding whether an answer was cut off by a length limit, filtered, or
    /// completed normally must not have to parse provider-specific JSON to find out.
    #[serde(default)]
    pub finish_reason: Option<FinishReason>,
    /// Whether this response was produced by the synthetic offline path.
    ///
    /// Real provider streams set this to `false`. Synthetic/offline paths set it
    /// to `true` so consumers and tests can tell the two apart without guessing
    /// from timing or payload shape.
    pub synthetic: bool,
    /// Whether provider-native tool-call state is present in this chunk.
    ///
    /// A stream that does not currently support provider tooling still sets this
    /// to `false` everywhere; it does not invent tool state to look more complete.
    #[serde(default)]
    pub tool_state: Option<ToolCallState>,
    /// Provider-reported usage for the request, if the provider supplied it.
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Provider-native metadata attached to this chunk or stream, if any.
    ///
    /// This is deliberately opaque at the neutral boundary. Provider modules may
    /// place things like provider event ids, repeat flags, or other provider-
    /// specific signals here; consumers that do not understand a provider must not
    /// treat unknown metadata as evidence that something happened externally.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Stable provider-side identifier for this stream, if the provider documents one.
    #[serde(default)]
    pub provider_ref: Option<String>,
    /// Provider error signal, if the stream ended with an error after partial output
    /// or was interrupted by one.
    #[serde(default)]
    pub error: Option<ProviderError>,
}

impl StreamChunk {
    /// A text delta chunk with no usage or tool state attached.
    pub fn text(index: u64, delta: impl Into<String>, synthetic: bool) -> Self {
        Self {
            index,
            kind: ChunkKind {
                text: Some(delta.into()),
                tool_call: None,
            },
            done: false,
            finish_reason: None,
            synthetic,
            tool_state: None,
            usage: None,
            metadata: None,
            provider_ref: None,
            error: None,
        }
    }

    /// The terminal chunk of a stream.
    pub fn terminal(index: u64, synthetic: bool) -> Self {
        Self {
            index,
            kind: ChunkKind {
                text: None,
                tool_call: None,
            },
            done: true,
            finish_reason: None,
            synthetic,
            tool_state: None,
            usage: None,
            metadata: None,
            provider_ref: None,
            error: None,
        }
    }
}

/// The neutral shape of an incremental delta.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChunkKind {
    /// Incremental text delta from the provider.
    pub text: Option<String>,
    /// Provider-native tool-call delta, if the stream supports tooling.
    #[serde(default)]
    pub tool_call: Option<ToolCallDelta>,
}

/// Provider-neutral view of a tool-call delta.
///
/// Provider modules translate their own tool-use/tool_calls shapes into this and
/// the dispatcher treats it as neutral state. Tool execution still flows through
/// the existing tool registry, permission/risk gate, executor, and evidence path;
/// this type is only the translation boundary, not a second execution framework.
///
/// The protocol translation for any provider is behind the native-tool-calling
/// acceptance gate, so no provider module currently emits these. The type exists
/// so the contract does not have to change when that gate passes.
/// One tool a request offers to the model.
///
/// This is the *declaration* half of tool calling: what the model is allowed to
/// ask for. Execution remains ZylCode's own gated path — the provider only
/// chooses a name and supplies arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolSpec {
    /// Name the model must use.
    pub name: String,
    /// What the tool does, shown to the model.
    pub description: String,
    /// JSON Schema for the arguments.
    pub parameters: serde_json::Value,
}

impl ToolSpec {
    /// The name this tool is declared under on the wire.
    ///
    /// ZylCode's ids contain dots (`fs.read`, `git.status`), and OpenAI's
    /// function-name grammar allows only `[A-Za-z0-9_-]` — a live request
    /// with `"name": "fs.read"` is rejected with HTTP 400, so the dotted id
    /// cannot be sent as-is. Characters outside the grammar become `_`, which
    /// keeps the mapping deterministic and reversible: the dispatcher resolves
    /// a returned name by comparing it to each declared tool's `wire_name()`.
    ///
    /// Duplicate wire names inside one declaration are invalid — the caller must
    /// not declare two ids that collapse to the same wire name, or a returned
    /// call would be ambiguous.
    pub fn wire_name(&self) -> String {
        self.name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }
}

/// Whether the model may, must, or must not call a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// The model decides.
    #[default]
    Auto,
    /// Tools are declared but must not be called.
    None,
    /// The model must call one of the declared tools.
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolCallState {
    /// In-progress tool calls assembled across one stream.
    #[serde(default)]
    pub calls: Vec<ToolCall>,
}

/// One provider-native tool call seen through the neutral contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolCall {
    /// Provider-assigned id for the call, if any.
    pub call_id: Option<String>,
    /// Provider-assigned id for the underlying tool use block, if any.
    pub block_id: Option<String>,
    /// Name of the tool the provider asked the assistant to call.
    pub name: String,
    /// Accumulated arguments so far.
    pub arguments: String,
    /// Whether the provider signaled this call is complete.
    pub done: bool,
    /// Provider-native metadata for this call, if any.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// One incremental piece of a provider tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ToolCallDelta {
    /// How this delta relates to a particular tool call.
    pub kind: ToolCallDeltaKind,
    /// Provider-assigned id for the call being edited, if any.
    pub call_id: Option<String>,
    /// Provider-assigned id for the underlying tool use block, if any.
    pub block_id: Option<String>,
    /// Delta text for the arguments, if this is an argument patch.
    pub argument_delta: Option<String>,
    /// Whether the provider signaled this call is now complete.
    pub done: bool,
    /// Provider-native metadata for this delta, if any.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// How a tool-call delta relates to a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallDeltaKind {
    /// A new tool call is starting.
    Start,
    /// Incremental argument text for an in-flight call.
    ArgumentDelta,
    /// A call reached a stop and should be considered complete.
    Done,
}

/// Provider-reported usage.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    /// Any provider-specific usage shape the neutral layer could not map falls here
    /// so the information is preserved rather than silently dropped.
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

impl Usage {
    pub fn new(input: Option<u64>, output: Option<u64>) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            extra: None,
        }
    }

    /// Merge provider-reported numbers, keeping whichever side is known.
    pub fn merge(&mut self, other: Usage) {
        self.input_tokens = other.input_tokens.or(self.input_tokens);
        self.output_tokens = other.output_tokens.or(self.output_tokens);
        if other.extra.is_some() {
            self.extra = other.extra;
        }
    }
}

/// A completed (non-streamed) provider answer.
///
/// The non-streaming path needs the same three facts the streaming path carries in
/// its terminal chunk — text, input/output usage — and nothing provider-specific.
/// Provider modules build it from their own response shape, so the router never
/// reads `choices`, `content`, or `eval_count` itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Completion {
    /// The assistant text, with no provider framing left in it.
    pub text: String,
    /// Tool calls the provider asked for, already translated. Empty when the
    /// provider asked for none — never a placeholder entry.
    pub tool_calls: Vec<ToolCall>,
    /// Provider-reported usage. Fields stay `None` when the provider did not report
    /// them, so the caller can decide whether to estimate instead of receiving a
    /// fabricated zero.
    pub usage: Usage,
}

impl Completion {
    /// An answer with no text and no reported usage.
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Why a provider stream stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The provider produced a normal text/tool-call response end.
    Stop,
    /// The provider hit a configured length limit.
    Length,
    /// Content was filtered or refused by the provider.
    ContentFiltered,
    /// The assistant side ended without producing a final answer.
    EndTurn,
    /// The provider gave a reason this neutral layer does not concretely model.
    Other,
    /// The stream ended because the caller cancelled it.
    Cancelled,
    /// The stream ended due to a transport/provider error.
    Error,
}

/// A provider error seen through the neutral boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderError {
    /// Provider error classifier, if one was derivable.
    pub kind: ProviderErrorKind,
    /// Provider's own error message, truncated as needed.
    pub message: String,
    /// Provider error code or type, if any.
    #[serde(default)]
    pub code: Option<String>,
    /// Provider-native error payload, preserved when the neutral layer could not map it.
    #[serde(default)]
    pub raw: Option<serde_json::Value>,
}

/// How the neutral layer classifies a provider error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorKind {
    /// Credentials the provider required were missing/invalid.
    Authentication,
    /// The endpoint is wrong or unreachable as an API endpoint.
    InvalidEndpoint,
    /// The requested model is not available from this provider.
    ModelUnavailable,
    /// The provider refused the request (rate limit, bad request, overloaded, etc.).
    ProviderRejected,
    /// Transport-level failure: DNS, refused connection, timeout, TLS.
    Transport,
    /// The stream ended before the provider signalled completion.
    Truncated,
    /// The provider sent a frame the neutral layer could not parse.
    Malformed,
    /// The stream ended for a reason we could not cleanly classify.
    Unknown,
}

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct CancelInner {
    cancelled: AtomicBool,
    notify: Notify,
}

/// A cloneable cancellation signal for one streaming operation.
///
/// Cancelling it terminates the underlying HTTP request: the transport driver
/// polls [`CancelToken::cancelled`] next to the response body and drops the
/// response on cancellation, which closes the connection and aborts the request
/// rather than merely stopping chunk forwarding.
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    inner: Arc<CancelInner>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation. Idempotent and safe to call from any thread.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Resolves once cancellation has been requested.
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        // Re-check after registering: `notify_waiters` only wakes waiters that
        // are already registered, so a cancel that lands between the two checks
        // must be observed here rather than lost.
        let notified = self.inner.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

/// Handle for an in-flight streaming operation.
///
/// Provider modules return this so the caller can abort the underlying operation
/// after the fact instead of only stopping the consumption loop.
#[derive(Debug, Clone, Default)]
pub struct StreamHandle {
    cancel: CancelToken,
}

impl StreamHandle {
    pub fn new(cancel: CancelToken) -> Self {
        Self { cancel }
    }

    /// A handle whose (absent) operation can never be cancelled.
    pub fn none() -> Self {
        Self::default()
    }

    /// Abort the underlying streaming request.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub fn token(&self) -> CancelToken {
        self.cancel.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

// ---------------------------------------------------------------------------
// Request shape shared by the provider modules
// ---------------------------------------------------------------------------

/// The neutral description of one streaming chat call.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub system: String,
    pub prompt: String,
    pub max_tokens: u64,
    pub temperature: f32,
    /// Tools the model may call. Empty means no `tools` field is sent at all,
    /// which is what a provider without native tool support must see.
    pub tools: Vec<ToolSpec>,
    /// Whether, and how firmly, the model should call one.
    pub tool_choice: ToolChoice,
}

impl ChatRequest {
    pub fn new(
        model: impl Into<String>,
        system: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            system: system.into(),
            prompt: prompt.into(),
            max_tokens: 4096,
            temperature: 0.2,
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
        }
    }

    /// Declare tools for this request.
    pub fn with_tools(mut self, tools: Vec<ToolSpec>) -> Self {
        self.tools = tools;
        self
    }
}

/// Where and how to reach one provider's streaming endpoint.
#[derive(Debug, Clone)]
pub struct StreamEndpoint {
    /// Provider name used in diagnostics only.
    pub provider: String,
    /// Fully-qualified streaming URL.
    pub url: String,
    /// Extra headers (auth and version negotiation) this provider requires.
    pub headers: Vec<(String, String)>,
    /// Whole-request deadline, including body read.
    pub timeout: Duration,
}

impl StreamEndpoint {
    pub fn new(provider: impl Into<String>, url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            provider: provider.into(),
            url: url.into(),
            headers: Vec::new(),
            timeout,
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

// ---------------------------------------------------------------------------
// Decoder boundary
// ---------------------------------------------------------------------------

/// One provider frame translated into neutral event material.
#[derive(Debug, Clone, Default)]
pub struct DecodedEvent {
    pub text: Option<String>,
    /// A provider-native tool-call delta, translated by the provider module.
    pub tool_call: Option<ToolCallDelta>,
    pub usage: Option<Usage>,
    pub finish: Option<FinishReason>,
    pub error: Option<ProviderError>,
    pub provider_ref: Option<String>,
    pub metadata: Option<serde_json::Value>,
    /// The provider explicitly signalled the end of the stream.
    pub done: bool,
}

impl DecodedEvent {
    pub fn text(delta: impl Into<String>) -> Self {
        Self {
            text: Some(delta.into()),
            ..Self::default()
        }
    }

    pub fn finish(reason: FinishReason) -> Self {
        Self {
            finish: Some(reason),
            ..Self::default()
        }
    }

    pub fn error(err: ProviderError) -> Self {
        Self {
            error: Some(err),
            finish: Some(FinishReason::Error),
            done: true,
            ..Self::default()
        }
    }

    pub fn done() -> Self {
        Self {
            done: true,
            ..Self::default()
        }
    }
}

/// A provider's line-oriented stream decoder.
///
/// `feed` receives one complete line (without its terminator, with a trailing
/// `\r` already stripped) and returns the neutral events that line produced.
/// Framing differences — SSE event blocks, NDJSON, `[DONE]` sentinels — belong to
/// the implementation, not to the transport driver.
pub trait LineDecoder {
    fn feed(&mut self, line: &str) -> Result<Vec<DecodedEvent>, ProviderError>;

    /// Flush any buffered partial frame at end of stream.
    fn finish(&mut self) -> Vec<DecodedEvent> {
        Vec::new()
    }
}

/// The result of one completed stream.
#[derive(Debug, Clone)]
pub struct StreamOutcome {
    pub text: String,
    pub finish: FinishReason,
    pub usage: Option<Usage>,
    pub chunks: u64,
    pub provider_ref: Option<String>,
    /// Tool calls assembled from the deltas, in the order the provider started
    /// them. Arguments are the accumulated text, not a parsed object: a provider
    /// may stream invalid JSON, and pretending otherwise would hide it.
    pub tool_calls: Vec<ToolCall>,
}

impl StreamOutcome {
    pub fn empty(finish: FinishReason) -> Self {
        Self {
            text: String::new(),
            finish,
            usage: None,
            chunks: 0,
            provider_ref: None,
            tool_calls: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Incremental line reader
// ---------------------------------------------------------------------------

/// Largest single line the reader will buffer before declaring the stream
/// malformed. A provider that violates this is not sending line-delimited data.
pub const MAX_LINE_BYTES: usize = 256 * 1024;

/// Splits an arbitrarily chunked byte stream into complete lines.
///
/// `reqwest` delivers body bytes in transport-sized pieces that have nothing to do
/// with provider frame boundaries, so every provider parser must go through
/// something like this rather than assuming one read equals one event.
#[derive(Debug, Default)]
pub struct LineReader {
    buf: Vec<u8>,
}

impl LineReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append bytes and return every complete line they finished.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, ProviderError> {
        self.buf.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let mut line: Vec<u8> = self.buf.drain(..=pos).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let text = String::from_utf8(line).map_err(|_| {
                ProviderError::new(
                    ProviderErrorKind::Malformed,
                    "provider stream contained a line that was not valid UTF-8".to_string(),
                )
            })?;
            lines.push(text);
        }
        if self.buf.len() > MAX_LINE_BYTES {
            return Err(ProviderError::new(
                ProviderErrorKind::Malformed,
                format!("provider stream line exceeded {MAX_LINE_BYTES} bytes without a newline"),
            ));
        }
        Ok(lines)
    }

    /// Return a trailing line that arrived without a terminator.
    pub fn finish(&mut self) -> Option<String> {
        if self.buf.is_empty() {
            return None;
        }
        let mut line = std::mem::take(&mut self.buf);
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        String::from_utf8(line).ok()
    }
}

// ---------------------------------------------------------------------------
// Transport driver
// ---------------------------------------------------------------------------

/// Send one streaming request and drive `decoder` over its body.
///
/// Responsibilities kept here so provider modules do not each re-implement them:
/// HTTP status classification, incremental line splitting, cancellation (which
/// drops the response and therefore aborts the request), truncation detection,
/// and monotonic chunk construction.
pub async fn run_stream<D: LineDecoder>(
    client: &reqwest::Client,
    endpoint: &StreamEndpoint,
    body: &serde_json::Value,
    cancel: &CancelToken,
    decoder: &mut D,
    synthetic: bool,
    on_chunk: &mut (dyn FnMut(StreamChunk) + Send),
) -> Result<StreamOutcome, ProviderError> {
    let mut outcome = StreamOutcome::empty(FinishReason::Other);
    let mut index: u64 = 0;

    // A cancelled request must not egress at all.
    if cancel.is_cancelled() {
        on_chunk(cancelled_terminal(index, synthetic));
        outcome.finish = FinishReason::Cancelled;
        return Ok(outcome);
    }

    let mut req = client
        .post(&endpoint.url)
        .timeout(endpoint.timeout)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .json(body);
    for (name, value) in &endpoint.headers {
        req = req.header(name.as_str(), value.as_str());
    }

    let resp = tokio::select! {
        biased;
        () = cancel.cancelled() => {
            on_chunk(cancelled_terminal(index, synthetic));
            outcome.finish = FinishReason::Cancelled;
            return Ok(outcome);
        }
        sent = req.send() => sent.map_err(|e| {
            ProviderError::new(ProviderErrorKind::Transport, format!("provider {} request failed: {e}", endpoint.provider))
        })?,
    };

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err(classify_http_failure(
            &endpoint.provider,
            status.as_u16(),
            &body_text,
        ));
    }

    let mut stream = resp.bytes_stream();
    let mut reader = LineReader::new();
    let mut finished = false;
    let mut failure: Option<ProviderError> = None;

    loop {
        let next = tokio::select! {
            biased;
            () = cancel.cancelled() => {
                // Dropping the byte stream drops the response body, which closes
                // the connection and aborts the in-flight request.
                drop(stream);
                on_chunk(cancelled_terminal(index, synthetic));
                outcome.finish = FinishReason::Cancelled;
                outcome.chunks = index;
                return Ok(outcome);
            }
            item = stream.next() => item,
        };

        let Some(item) = next else {
            break;
        };

        let bytes = item.map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Transport,
                format!("provider {} stream interrupted: {e}", endpoint.provider),
            )
        })?;

        let lines = reader.push(&bytes)?;
        let mut events: Vec<DecodedEvent> = Vec::new();
        for line in lines {
            events.extend(decoder.feed(&line)?);
        }
        for event in events {
            if emit(
                event,
                &mut outcome,
                &mut index,
                synthetic,
                &mut failure,
                on_chunk,
            ) {
                finished = true;
            }
        }
        if finished {
            break;
        }
    }

    if !finished {
        if let Some(tail) = reader.finish() {
            for event in decoder.feed(&tail)? {
                if emit(
                    event,
                    &mut outcome,
                    &mut index,
                    synthetic,
                    &mut failure,
                    on_chunk,
                ) {
                    finished = true;
                }
            }
        }
        for event in decoder.finish() {
            if emit(
                event,
                &mut outcome,
                &mut index,
                synthetic,
                &mut failure,
                on_chunk,
            ) {
                finished = true;
            }
        }
    }

    // A provider that failed after partial output is still a failure. The partial
    // text and the error-bearing terminal chunk have already been forwarded, so a
    // caller can render what arrived, but the call must not look successful.
    if let Some(err) = failure {
        outcome.chunks = index;
        return Err(err);
    }

    if !finished {
        // The provider closed the connection without a completion signal. That is
        // a truncated stream, not a successful short answer.
        return Err(ProviderError::new(
            ProviderErrorKind::Truncated,
            format!(
                "provider {} stream ended after {} chunk(s) without a completion signal",
                endpoint.provider, index
            ),
        ));
    }

    outcome.chunks = index;
    Ok(outcome)
}

/// The one terminal chunk a cancelled stream emits.
///
/// It says *why* it ended, so a consumer does not have to treat "no more chunks"
/// as either success or failure after a cancellation.
fn cancelled_terminal(index: u64, synthetic: bool) -> StreamChunk {
    let mut chunk = StreamChunk::terminal(index, synthetic);
    chunk.finish_reason = Some(FinishReason::Cancelled);
    chunk
}

/// Fold one provider tool delta into the running list of calls.
///
/// Calls are correlated by `call_id` (OpenAI-style) or `block_id`
/// (Anthropic-style), whichever the provider supplies. A `Start` carries the
/// tool name in `metadata.name`, because [`ToolCallDelta`] deliberately has no
/// name field: only the first delta names the tool, and the neutral contract
/// keeps per-provider framing out of its own shape.
///
/// An argument delta for a call that never started is dropped rather than
/// invented: a call with no name cannot be dispatched, and fabricating one would
/// produce a tool invocation nobody asked for.
fn apply_tool_delta(calls: &mut Vec<ToolCall>, delta: ToolCallDelta) {
    let key = delta.call_id.clone().or_else(|| delta.block_id.clone());
    let position = key.as_ref().and_then(|k| {
        calls
            .iter()
            .position(|c| c.call_id.as_ref() == Some(k) || c.block_id.as_ref() == Some(k))
    });

    match delta.kind {
        ToolCallDeltaKind::Start => {
            let name = delta
                .metadata
                .as_ref()
                .and_then(|m| m.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            if name.is_empty() {
                return;
            }
            calls.push(ToolCall {
                call_id: delta.call_id,
                block_id: delta.block_id,
                name,
                arguments: String::new(),
                done: false,
                metadata: delta.metadata,
            });
        }
        ToolCallDeltaKind::ArgumentDelta => {
            if let (Some(pos), Some(text)) = (position, delta.argument_delta) {
                calls[pos].arguments.push_str(&text);
            }
        }
        ToolCallDeltaKind::Done => {
            if let Some(pos) = position {
                calls[pos].done = true;
            }
        }
    }
}

/// Fold one decoded event into the running outcome, returning `true` when the
/// stream is complete.
fn emit(
    event: DecodedEvent,
    outcome: &mut StreamOutcome,
    index: &mut u64,
    synthetic: bool,
    failure: &mut Option<ProviderError>,
    on_chunk: &mut (dyn FnMut(StreamChunk) + Send),
) -> bool {
    if let Some(err) = event.error.clone() {
        let mut chunk = StreamChunk::terminal(*index, synthetic);
        chunk.finish_reason = Some(FinishReason::Error);
        chunk.error = Some(err.clone());
        on_chunk(chunk);
        *index += 1;
        outcome.finish = FinishReason::Error;
        outcome.chunks = *index;
        *failure = Some(err);
        return true;
    }

    if let Some(text) = &event.text {
        outcome.text.push_str(text);
    }
    if let Some(provider_ref) = &event.provider_ref {
        outcome.provider_ref = Some(provider_ref.clone());
    }
    if let Some(finish) = event.finish {
        outcome.finish = finish;
    }

    if let Some(delta) = event.tool_call.clone() {
        apply_tool_delta(&mut outcome.tool_calls, delta);
    }

    let has_payload = event.text.is_some()
        || event.usage.is_some()
        || event.metadata.is_some()
        || event.provider_ref.is_some()
        || event.finish.is_some()
        || event.tool_call.is_some();

    if has_payload {
        if let Some(usage) = event.usage.clone() {
            match &mut outcome.usage {
                Some(existing) => existing.merge(usage.clone()),
                None => outcome.usage = Some(usage.clone()),
            }
        }
        let mut chunk =
            StreamChunk::text(*index, event.text.clone().unwrap_or_default(), synthetic);
        if event.text.is_none() {
            chunk.kind.text = None;
        }
        chunk.usage = event.usage;
        chunk.finish_reason = event.finish;
        chunk.metadata = event.metadata;
        chunk.provider_ref = event.provider_ref;
        if event.tool_call.is_some() {
            chunk.kind.tool_call = event.tool_call;
            chunk.tool_state = Some(ToolCallState {
                calls: outcome.tool_calls.clone(),
            });
        }
        // Completion is signalled by exactly one terminal chunk below, never by a
        // payload chunk, so consumers cannot see two "done" chunks per stream.
        chunk.done = false;
        on_chunk(chunk);
        *index += 1;
    }

    if event.done {
        let mut terminal = StreamChunk::terminal(*index, synthetic);
        terminal.finish_reason = Some(outcome.finish);
        terminal.usage = outcome.usage.clone();
        terminal.provider_ref = outcome.provider_ref.clone();
        on_chunk(terminal);
        *index += 1;
        outcome.chunks = *index;
        return true;
    }
    false
}

/// Map a non-success provider HTTP response into a neutral error.
pub fn classify_http_failure(provider: &str, status: u16, body: &str) -> ProviderError {
    let lower = body.to_lowercase();
    let kind = match status {
        401 | 403 => ProviderErrorKind::Authentication,
        404 => {
            if lower.contains("model") {
                ProviderErrorKind::ModelUnavailable
            } else {
                ProviderErrorKind::InvalidEndpoint
            }
        }
        400 | 422 => {
            if lower.contains("model") {
                ProviderErrorKind::ModelUnavailable
            } else {
                ProviderErrorKind::ProviderRejected
            }
        }
        429 => ProviderErrorKind::ProviderRejected,
        500..=599 => ProviderErrorKind::ProviderRejected,
        _ => ProviderErrorKind::Unknown,
    };
    let mut err = ProviderError::new(
        kind,
        format!("provider {provider} returned HTTP {status}: {body}"),
    );
    err.code = Some(status.to_string());
    err
}

/// Shared streaming constants used by provider modules and the dispatcher.
pub mod constants {
    /// Maximum characters to preserve from a provider error message in the neutral
    /// error shape.
    pub const ERROR_MESSAGE_MAX_CHARS: usize = 2000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_reader_splits_across_arbitrary_chunk_boundaries() {
        let mut reader = LineReader::new();
        assert!(!reader.push(b"one\ntw").unwrap().is_empty());
        let mut got = Vec::new();
        got.extend(reader.push(b"o\nthree").unwrap());
        assert_eq!(got, vec!["two".to_string()]);
        got.extend(reader.push(b"\n").unwrap());
        assert_eq!(got, vec!["two".to_string(), "three".to_string()]);
        assert_eq!(reader.finish(), None);
    }

    #[test]
    fn line_reader_strips_carriage_returns() {
        let mut reader = LineReader::new();
        assert_eq!(reader.push(b"a\r\n").unwrap(), vec!["a".to_string()]);
        assert_eq!(reader.push(b"b").unwrap(), Vec::<String>::new());
        assert_eq!(reader.finish().as_deref(), Some("b"));
    }

    #[test]
    fn line_reader_reports_an_unterminated_giant_line() {
        let mut reader = LineReader::new();
        let err = reader.push(&vec![b'x'; MAX_LINE_BYTES + 1]).unwrap_err();
        assert_eq!(err.kind, ProviderErrorKind::Malformed);
    }

    #[tokio::test]
    async fn cancel_token_is_observable_before_and_after_waiting() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
        // Resolves immediately, i.e. a cancel that lands before anyone awaits is
        // not lost.
        token.cancelled().await;
    }

    #[tokio::test]
    async fn cancel_token_wakes_a_registered_waiter() {
        let token = CancelToken::new();
        let waiter = token.clone();
        let handle = tokio::spawn(async move { waiter.cancelled().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("waiter must wake")
            .expect("waiter task must not panic");
    }

    #[test]
    fn http_failure_classification_distinguishes_actionable_cases() {
        assert_eq!(
            classify_http_failure("anthropic", 401, "bad key").kind,
            ProviderErrorKind::Authentication
        );
        assert_eq!(
            classify_http_failure("ollama", 404, "no route").kind,
            ProviderErrorKind::InvalidEndpoint
        );
        assert_eq!(
            classify_http_failure("openai", 404, "model 'x' not found").kind,
            ProviderErrorKind::ModelUnavailable
        );
        assert_eq!(
            classify_http_failure("openai", 429, "slow down").kind,
            ProviderErrorKind::ProviderRejected
        );
        let err = classify_http_failure("openai", 503, "down");
        assert_eq!(err.code.as_deref(), Some("503"));
        assert!(err.message.contains("503"));
    }

    #[test]
    fn finish_reason_is_a_first_class_field_on_both_payload_and_terminal_chunks() {
        let mut outcome = StreamOutcome::empty(FinishReason::Other);
        let mut index = 0u64;
        let mut failure = None;
        let mut seen: Vec<StreamChunk> = Vec::new();

        let event = DecodedEvent {
            text: Some("partial".into()),
            usage: None,
            finish: Some(FinishReason::Length),
            error: None,
            provider_ref: None,
            metadata: None,
            tool_call: None,
            done: true,
        };
        assert!(emit(
            event,
            &mut outcome,
            &mut index,
            false,
            &mut failure,
            &mut |c| seen.push(c)
        ));

        assert_eq!(seen.len(), 2, "one payload chunk and one terminal chunk");
        assert_eq!(
            seen[0].finish_reason,
            Some(FinishReason::Length),
            "a payload chunk must not hide the finish reason in metadata"
        );
        assert!(seen[0].metadata.is_none());
        assert_eq!(seen[1].finish_reason, Some(FinishReason::Length));
        assert!(seen[1].done);
    }

    #[test]
    fn a_cancelled_stream_says_so_instead_of_looking_complete() {
        let chunk = cancelled_terminal(4, false);
        assert!(chunk.done);
        assert_eq!(chunk.finish_reason, Some(FinishReason::Cancelled));
        assert!(chunk.error.is_none());
        assert!(!chunk.synthetic);
    }

    #[test]
    fn terminal_and_text_chunks_carry_provenance() {
        let real = StreamChunk::text(0, "hi", false);
        assert!(!real.synthetic);
        assert!(!real.done);
        let synth = StreamChunk::terminal(1, true);
        assert!(synth.synthetic);
        assert!(synth.done);
    }
}
