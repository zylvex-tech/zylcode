# Provider Architecture Proposal — ZylCode Core

Scope: replace fake/dispatch_prompt-splitting streaming and absent native tool calling with a real, provider-neutral streaming contract and per-provider protocol modules, while keeping `crates/zylcode-core/src/router.rs` as orchestration only.

Status: proposal, pre-implementation. Contracts below are the basis for the first implementation pass; no provider client code is written yet.

## 1. Provider directory layout

New tree under `crates/zylcode-core/src/providers/`:

```
crates/zylcode-core/src/providers/
    mod.rs                 # public re-exports, capability table, top-level stream dispatch
    streaming.rs           # provider-neutral StreamChunk contract + StreamHandle + cancellation
    capabilities.rs        # ProviderCapabilitySet + provider_capabilities() + capability evidence notes
    http.rs                # shared HTTP helpers: method/body construction, timeout, cancellation-aware fetch abstraction
    error.rs               # provider-neutral ProviderError + provider-specific error classification

    anthropic/
        mod.rs
        client.rs          # request construction, auth headers, endpoint selection
        streaming.rs       # SSE parsing -> StreamChunk, incremental, termination, malformed handling
        tools.rs           # Anthropic tool-use request/partial + tool-result serialization

    openai_compatible/
        mod.rs
        client.rs          # request construction, Bearer/auth, endpoint + header variation per provider kind
        streaming.rs       # OpenAI-compatible SSE parsing -> StreamChunk, incremental, termination, malformed
        tools.rs           # OpenAI function/tool_calls request + result serialization

    ollama/
        mod.rs
        client.rs          # /api/chat request construction, model routing
        streaming.rs       # NDJSON chunk parsing -> StreamChunk, incremental, termination, malformed
        tools.rs           # Ollama tool-call request + result serialization

    synthetic/
        mod.rs
        mod.rs
        client.rs          # offline/synthetic path: deterministic incremental chunks, explicitly labeled offline
```

Ownership rules:

- `capabilities.rs` owns the static capability model and the evidence notes that justify each flag.
- `streaming.rs` owns the provider-neutral stream contract, `StreamHandle`, and cancellation/timeout semantics shared by all providers.
- `http.rs` owns shared HTTP construction and the cancellation-aware transport boundary; it does not own provider protocol parsing.
- Each provider folder owns that provider's wire format, request construction, SSE/NDJSON parsing, and tool-call serialization. No provider-specific parsing lives in `router.rs`.
- `synthetic/` owns the offline path and is the only place where "offline/synthetic" labeling is produced.
- `router.rs` orchestrates: pick provider config, pick capabilities, delegate to the provider module's streaming entry point, forward `StreamChunk` to the consumer, and map provider errors to the neutral `ProviderError`.

What does NOT move into these modules from `router.rs`:

- `ModelProvider`, `ProviderKind`, `ProviderConfig`, `RouterConfig`, `TokenRouter`, `dispatch_prompt`, `call_provider`, `probe_provider_with_config` stay in `router.rs` as orchestration/config surface.

## 2. Provider-neutral `StreamChunk` contract

This is the streaming contract at the dispatch boundary: `dispatch_stream` and
`dispatch_stream_cancellable` hand consumers `StreamChunk` values directly, and
`router.rs` no longer defines a reduced stream type of its own.

Implemented shape (`providers/streaming.rs`). Owned strings rather than borrowed
slices: chunks cross an `await` boundary and are serialised at the IPC edge, so
borrowed deltas would force a lifetime parameter on every consumer for no gain.

```rust
pub struct StreamChunk {
    pub index: u64,
    pub kind: ChunkKind,
    pub done: bool,
    pub finish_reason: Option<FinishReason>,
    pub synthetic: bool,
    pub tool_state: Option<ToolCallState>,
    pub usage: Option<Usage>,
    pub metadata: Option<serde_json::Value>,
    pub provider_ref: Option<String>,
    pub error: Option<ProviderError>,
}

pub struct ChunkKind {
    pub text: Option<String>,
    pub tool_call: Option<ToolCallDelta>,
}
```

Design notes:

- `finish_reason` is a first-class field, not a `metadata` entry. A consumer deciding whether an answer was truncated (`Length`), filtered (`ContentFiltered`), cancelled (`Cancelled`), or completed (`EndTurn`/`Stop`) must not have to parse provider-specific JSON.
- `synthetic: bool` is set once per stream by the producing module. Synthetic streams are never presented as real provider completions; they are explicitly offline/simulation-shaped.
- Exactly one chunk per stream carries `done = true`, and that terminal chunk repeats the stream-level `finish_reason`, `usage`, and `provider_ref` so a consumer that only inspects the terminal chunk still learns how the stream ended.
- `tool_call` is `Option<ToolCallDelta>` rather than a required enum: a plain text delta and a tool-call delta share one chunk type, and no provider currently advertises tool calling (see `capabilities.rs` — `tools` is `false` everywhere until that gate passes).
- `metadata` stays opaque and is never the only carrier of information the contract claims to represent.
- `ProviderError` is neutral and classifies provider failures without forcing the consumer to parse SSE/NDJSON error shapes directly.
- `index` is the stream-sequence counter so consumers can detect gaps, reorder if needed, and correlate cancellation/timeout.

### 2a. Compatibility projection at the application boundary

The desktop IPC layer ships `{ index, delta, done }` on `intent:chunk`, and the
frontend renders exactly that shape (`StreamDelta` in `src/lib/events.ts`).
`providers/compat.rs` holds one explicitly lossy projection, `StreamEvent`, whose
only constructor is `From<&StreamChunk>`.

Rules that keep this a projection and not a second streaming path:

- nothing is produced from a provider into `StreamEvent`; the chunk is always the source,
- the dispatcher never builds one, so it cannot drift from the contract,
- fields the projection drops (usage, finish reason, tool deltas, structured errors, synthetic provenance) remain on the chunk, which is what new consumers must read.

A consumer that can read `StreamChunk` should do so instead of using `StreamEvent`.

## 3. Tool-call contract

This contract is the neutral representation used at the ZylCode execution boundary. Provider modules translate their native tool formats into/from this; the rest of the system never sees Anthropic `tool_use` blocks, OpenAI `tool_calls`, or Ollama tool-shaped NDJSON except inside the relevant provider module.

Neutral tool-request representation:

```rust
pub struct ToolCallRequest<'a> {
    pub call_id: String,
    pub name: &'a str,
    pub arguments: &'a str,
    pub partial: bool,
}

pub struct ToolCallOutcome<'a> {
    pub call_id: &'a str,
    pub name: &'a str,
    pub status: ToolCallStatus,
    pub result_json: Option<&'a str>,
    pub error: Option<ToolExecutionError>,
    pub evidence_id: Option<String>,
}

pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    RejectedByPolicy,
}

pub struct ToolExecutionError {
    pub class: String,
    pub message: String,
    pub provider_note: Option<String>,
}
```

Flow (authoritative, not a second execution framework):

1. Provider produces a neutral `ToolCallRequest` from its native tool-use block.
2. ZylCode routes the request through the existing `ToolRegistry` / permission/risk gate / `real_tools::dispatch` / approved executor path.
3. Execution produces evidence and a tool result.
4. Provider module translates the neutral tool result back into the provider-native tool-result shape for the continuation request.
5. The provider stream continues with the tool result embedded in the next provider request.

Constraints:

- Provider modules may only translate; they do not re-implement file system, shell, git, or search execution.
- `ToolCallOutcome` carries `evidence_id` so evidence ledgers can be correlated with provider tool turns.
- `RejectedByPolicy` is a first-class status so the provider continuation can represent a denied tool call without fabricating success.

## 4. Capability model

Capabilities are static and evidence-bound. A capability is advertised only when the corresponding runtime path and tests exist.

```rust
pub struct ProviderCapabilitySet {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub structured_output: bool,
    pub logprobs: bool,
    pub usage: bool,
    pub embeddings: bool,
    pub reasoning: bool,
    pub max_client_tokens: Option<u64>,
    pub notes: Option<&'static str>,
}
```

Current proposed evidence posture:

- `streaming`: claimed only for providers with a real incremental parsing implementation + tests. Synthetic/offline is deliberately not "real streaming"; it is labeled `synthetic`.
- `tools`: claimed only for providers with a real native tool-call parse/serialize path integrated with the existing execution boundary.
- `vision`: NOT claimed until image content blocks are actually sent and handled end to end.
- `structured_output`: NOT claimed until the provider's structured-output modality is implemented and tested.
- `logprobs`: claimed only where the provider endpoint actually returns them and we parse them.
- `usage`: claimed only when usage is parsed from real provider responses.
- `embeddings`: separate from chat; claimed only when an embeddings path exists.
- `reasoning`: claimed only when the provider exposes reasoning in a way we actually surface.

The model picker must respect these flags: it must not promise streaming, tools, vision, or structured output for a provider unless the flag is true and the flag is true for a reason that is currently true.

## 5. Acceptance gates before native tool calling

Streaming work is not "done" when a chunk type exists. Each provider must pass the following before we claim native tool calling for any provider.

### Anthropic

- Request construction uses the real Anthropic messages endpoint and auth headers.
- SSE response is parsed incrementally, not as one-shot then split.
- Incremental `text_delta` arrives as separate `StreamChunk` values.
- Stream terminates cleanly with a finish reason.
- Malformed SSE events are handled without panicking the stream.
- Provider error responses are classified and surfaced through `ProviderError`.
- Cancellation reaches the underlying HTTP request and terminates it, not just the forwarder.

### OpenAI-compatible

- Same criteria using the provider's SSE/delta semantics.
- Must work for at least one OpenAI-compatible variant end to end in the deterministic harness.
- Tool-call parsing must be separable from text streaming in tests.

### Ollama

- Same criteria using Ollama's NDJSON chunk format.
- Must demonstrate incremental arrival of deltas, not buffered completion.

### Synthetic/offline

- Produces deterministic incremental chunks.
- Every synthetic stream is explicitly marked `synthetic: true`.
- Never represented as a real provider response.
- Never reports successful external execution, deployment, or side effects.

### Shared

- Cancellation must terminate the underlying request, with stream termination and provider connection cleanup where applicable.
- Timeout must be a real timeout on the HTTP/transport boundary, not a client-side sleep.
- Tests must be deterministic and local; ordinary CI must not require live provider credentials or public internet access.

## 6. Hard architecture constraints

- `router.rs` remains orchestration; it does not become the home for SSE parsing, NDJSON parsing, Anthropic tool-use blocks, OpenAI function calling, or Ollama transport details.
- No fake streaming implemented by splitting a completed response into lines.
- No fake native tool calling.
- No public internet or real credentials required by ordinary CI tests; use deterministic local HTTP fixtures/harnesses.
- Cancellation and timeout are real transport-level concerns.
- `ToolRegistry`, permission/risk controls, and evidence infrastructure remain the execution path; provider modules translate only.
- Existing MCP infrastructure in `zylcode-mcp` is retained and extended; do not build a parallel MCP architecture.
- No capability is advertised until the corresponding runtime path and tests exist.
- Provider module additions are scoped so `router.rs` does not grow substantially as a monolith.

## 7. MCP preparation

Before implementing the MCP client, verify the current authoritative MCP protocol and transport requirements. Do not hard-code handshake details from an older MCP implementation.

Verification checklist:

- `initialize` and capability negotiation in the current spec.
- `tools/list` and `tools/call` request/response shapes.
- Notifications versus requests.
- Cancellation semantics at the protocol and transport level.
- Streamable HTTP transport requirements, including how polling/URL-based progress and cancellation work.
- Stdio transport requirements.
- Current error/response semantics and how servers signal tool errors.

The MCP client must then implement the protocol as specified, with the transport layer isolated from the tool-execution layer, so the existing `ToolRegistry` / permission / evidence path remains the execution boundary.

## 8. Reporting cadence at milestones

At each major milestone, report:

1. files changed
2. what is now genuinely executable
3. tests added
4. tests passed
5. remaining limitations
6. whether the capability can now truthfully be advertised

No fabricated completion. No UI-only claims. Capability claims track implementation evidence.

## 9. Immediate next step

Before writing provider clients:

1. Write this proposal to the workspace (done by this document).
2. Read the current `router.rs` streaming/tool/paths to confirm extraction boundaries.
3. Verify the current MCP spec before any MCP code.
4. Then implement `providers/` + `StreamChunk` + capability model + deterministic harness tests.
5. Gate native tool calling behind the acceptance tests above.
