# Provider Extraction Plan — Concrete Router.rs Edits

Status: router.rs currently compiles after collapsing the duplicated `ProviderCapabilitySet` block. This document defines the exact edits that move provider protocol out of `router.rs` before any provider client code is written. It is the implementation spec for the first provider pass, not a new architecture discussion.

## Target state

After this pass:

- `crates/zylcode-core/src/providers/` exists and owns provider-neutral streaming types, the capability model, shared HTTP utilities, and provider error classification.
- `router.rs` keeps `ModelProvider`, `ProviderKind`, `ProviderConfig`, `RouterConfig`, `TokenRouter`, `dispatch_prompt`, `call_provider`, `probe_provider_with_config`, telemetry, cost estimation, and the orchestration wiring.
- `router.rs` no longer owns SSE parsing, NDJSON parsing, Anthropic `tool_use` block handling, OpenAI `tool_calls` handling, Ollama tool serialization, or the static per-provider capability table.
- `StreamEvent` is retired at the dispatch boundary and replaced by the new provider-neutral `StreamChunk` contract at the consumer side.

## Non-negotiable boundaries

- Provider-specific request construction and response parsing live in `providers/<provider>/...`.
- `router.rs` calls into `providers::streaming` / `providers::capabilities` / `providers::error` only through the neutral contract.
- The new modules must not pull provider-specific parsing back into `router.rs` via generics or callbacks that encode wire formats there.
- The existing `reqwest::Client` owned by `TokenRouter` is the HTTP surface; provider modules should not introduce a second HTTP client.

## Extraction sequence

### 1. Create the provider-neutral surface first

Files:

- `crates/zylcode-core/src/providers/mod.rs`
- `crates/zylcode-core/src/providers/streaming.rs`
- `crates/zylcode-core/src/providers/capabilities.rs`
- `crates/zylcode-core/src/providers/error.rs`
- `crates/zylcode-core/src/providers/http.rs`

Responsibilities:

- `streaming.rs` defines `StreamChunk`, `ChunkKind`, `ToolCallDeltaKind`, `FinishReason`, `Usage`, `ProviderMetadata`, `ProviderError`, and a cancellation-aware stream handle abstraction used by the router and provider modules.
- `capabilities.rs` defines `ProviderCapabilitySet` and `provider_capabilities()` with the evidence posture in the proposal. This is the single source of truth for advertised capabilities; `router.rs` stops defining it.
- `error.rs` defines neutral provider error classification so provider modules can map their wire errors into one shape.
- `http.rs` defines shared helpers for constructing POST bodies, headers, timeouts, and cancellation-aware send boundaries on top of the existing `reqwest::Client`. It does not parse provider responses.
- `mod.rs` re-exports the public neutral types and the capability lookup, and declares the provider submodules.

### 2. Move the capability model out of router.rs

Exact edit:

- Remove the current `StreamEvent` + `ProviderCapabilitySet` + `Default` + `provider_capabilities()` block from `router.rs`.
- Keep `StreamEvent` only briefly if anything still references it; otherwise remove it entirely once `dispatch_stream` is migrated to the neutral contract.
- Add `pub use crate::providers::capabilities::ProviderCapabilitySet;` and `pub use crate::providers::capabilities::provider_capabilities;` if downstream consumers need them from `router`.
- Add `pub use crate::providers::streaming::...;` for the types the dispatcher callback will consume.

### 3. Replace the fake `dispatch_stream` with a neutral streaming entry point

Exact edit:

- Rewrite `TokenRouter::dispatch_stream` so it:
  - resolves the effective provider + model from the snapshot config,
  - looks up `provider_capabilities(provider)`,
  - delegates to the appropriate provider module’s streaming entrypoint,
  - forwards neutral `StreamChunk` values to `on_chunk`,
  - maps provider errors to neutral `ProviderError`,
  - marks synthetic/offline streams explicitly `synthetic: true`.
- The callback type becomes `FnMut(StreamChunk) + Send` (or an equivalent provider-neutral chunk type), not `FnMut(StreamEvent) + Send`.

### 4. Keep `call_provider` as orchestration, but thin it toward provider modules

The current `call_provider` does three things:

- builds the per-provider JSON body,
- sets per-provider auth headers + OpenRouter extras,
- sends the request and parses text/usage out of the JSON response.

Extraction goal:

- Move per-provider body construction and per-provider response extraction into provider modules.
- Keep `call_provider` as the place that:
  - chooses the provider module,
  - hands it the config/snapshot and the HTTP client,
  - receives a neutral outcome.

This is the part most likely to grow, so the constraint is: provider modules own the wire shapes; `call_provider` owns selection, config snapshot, and neutral outcome mapping.

### 5. Provider module skeleton

Initial modules to create:

- `providers/anthropic/` — request construction for `/v1/messages`, auth headers, SSE parsing, and Anthropic tool-use block translation.
- `providers/openai_compatible/` — request construction for `/chat/completions`, Bearer/auth + OpenRouter extras, SSE parsing, and OpenAI tool_calls translation.
- `providers/ollama/` — `/api/chat` request construction and NDJSON parsing.
- `providers/synthetic/` — deterministic offline stream, explicitly labeled synthetic.

Each provider module exposes:

- a request-building function,
- a streaming entrypoint returning the neutral contract,
- a tool translation boundary if/when tool calling is implemented for that provider.

### 6. What stays in router.rs

Stay in `router.rs`:

- `ModelProvider`, `ProviderKind`, `ProviderConfig`, `RouterConfig`.
- `TokenRouter` construction and snapshot semantics.
- `dispatch_prompt`.
- `probe_provider_with_config` and `ProbeStatus` / `ProviderProbe`.
- token/cost/scorecard telemetry.
- orchestration that chooses provider module and wires neutral stream chunks to the callback.

Move out of `router.rs`:

- `StreamEvent` at the dispatch boundary.
- `ProviderCapabilitySet` and `provider_capabilities()`.
- SSE/NDJSON parsing.
- Anthropic tool_use / OpenAI tool_calls / Ollama tool serialization.
- Per-provider response JSON extraction.

### 7. Tests to add with this pass

These are local and deterministic; no live credentials.

- neutral chunk construction and serialization,
- synthetic stream emits `synthetic: true` and never looks like a real provider response,
- provider error classification maps sample wire errors into neutral `ProviderError`,
- capability lookup returns the intended static set per provider,
- `dispatch_stream` signature and forwarding behavior with a no-op or recording callback,
- that `router.rs` no longer contains provider-specific SSE/NDJSON parsing or Anthropic/OpenAI/Ollama tool serialization.

### 8. Acceptance gate before native tool calling

Do not implement Anthropic tool_use, OpenAI tool_calls, or Ollama tool parsing as a second execution system. When tool calling is implemented, it must:

- translate provider-native tool blocks into the neutral tool-call contract,
- route through the existing ToolRegistry / permission / evidence path,
- translate the neutral result back to the provider-native result shape only at the provider boundary,
- be covered by deterministic local tests for parse/serialize/forward/result translation.

Until those exist, tool capabilities remain not claimed.

## Reporting after this pass

After the extraction commit, report:

1. files changed,
2. what is now genuinely executable,
3. tests added,
4. tests passed,
5. remaining limitations,
6. whether streaming and capabilities can now truthfully be advertised for each provider.

No claim of real streaming or real tool calling until the corresponding implementation and tests exist.

---

## Implementation status — Phase 1 (landed)

This section records what the first pass actually shipped, as opposed to what it planned.

### Files

| Path | Role |
| --- | --- |
| `providers/mod.rs` | module root, declared provider submodules, capability tests |
| `providers/streaming.rs` | neutral `StreamChunk` contract, `CancelToken`, `StreamHandle`, `LineReader`, `LineDecoder`, `run_stream` transport driver, `classify_http_failure` |
| `providers/sse.rs` | SSE framing (`event:`/`data:` blocks, comments, multi-line data, unterminated tail) |
| `providers/capabilities.rs` | static per-provider capability table (single source of truth) |
| `providers/error.rs` | neutral error → probe-status and finish-reason mapping |
| `providers/http.rs` | base-URL joining helpers only; no response parsing |
| `providers/respan.rs` | Respan gateway constants |
| `providers/anthropic.rs` | Messages API body/headers, SSE decoder, `stream()` |
| `providers/openai_compatible.rs` | chat-completions body/headers, SSE decoder, `stream()` |
| `providers/ollama.rs` | `/api/chat` body, NDJSON decoder, `stream()` |
| `providers/synthetic.rs` | labelled offline payload and labelled synthetic chunk stream |
| `providers/test_server.rs` | deterministic loopback HTTP server for transport tests |
| `providers/transport_tests.rs` | acceptance tests over real sockets |

`router.rs` lost nothing it is responsible for: it still owns provider selection, the
chain, telemetry, cost estimation, and the probe. It gained
`dispatch_stream_cancellable` and now delegates the synthetic payload and the
streaming protocol to the provider modules.

### What is genuinely executable now

- Real incremental streaming from Anthropic (SSE), OpenAI-compatible (SSE), and Ollama
  (NDJSON), driven by `router::TokenRouter::dispatch_stream`.
- Cancellation that aborts the request: `CancelToken` is polled next to the response
  body and the response is dropped, closing the connection. Verified by a loopback
  server that reports the disconnect.
- Structured failure classification on the wire: 401/403, 404 (endpoint vs model),
  400/422, 429/5xx, refused connection, timeout, malformed frame, truncated stream,
  and provider error after partial output.
- A deterministic local harness. No credentials and no public internet are needed.

### What is deliberately not implemented

- **Native tool calling.** No provider module sends `tools`, and
  `provider_capabilities().tools` is `false` for every provider. Tool deltas that a
  provider sends anyway are surfaced as opaque `metadata`, never executed.
- **`StreamChunk` at the public dispatch boundary.** `dispatch_stream` still speaks
  the pre-existing `StreamEvent { index, delta, done }` so downstream consumers keep
  compiling. The neutral contract is what provider modules produce; promoting it to
  the public boundary is a separate, breaking change.
- **Respan-specific request shaping.** Respan routes through the OpenAI-compatible
  module; only its constants are extracted.

### Acceptance gate (Phase 1E)

| Requirement | Evidence |
| --- | --- |
| provider protocol lives under `providers/` | no SSE/NDJSON/body construction left in `router.rs` |
| single neutral chunk contract | `StreamChunk`; all three decoders emit `DecodedEvent` |
| genuine streaming | `anthropic_delivers_genuine_incremental_chunks` asserts two distinct text chunks |
| synthetic is labelled, never claims success | `synthetic::tests::payload_never_claims_a_real_side_effect`, `every_chunk_is_marked_synthetic_and_the_stream_terminates_once` |
| cancellation aborts the request | `cancelling_mid_stream_aborts_the_underlying_request` (server observes disconnect) |
| deterministic local tests | `providers::test_server` loopback; 67 provider tests, no network |
| provider error / malformed / truncated / timeout / refused | `transport_tests` |
| no native tool calling | `native_tool_calling_is_not_claimed_anywhere`; decoders keep tool deltas as metadata |
| no second execution framework | provider layer returns neutral chunks; `ToolRegistry` untouched |

### Streaming truth table

| Provider | Streams | Transport | Cancellation honours request abort | Tools |
| --- | --- | --- | --- | --- |
| Anthropic | yes | SSE | yes | not claimed |
| OpenAI / OpenRouter / DeepSeek / Respan / custom | yes | SSE | yes | not claimed |
| Ollama | yes | NDJSON | yes | not claimed |
| Synthetic offline | labelled synthetic chunks only | none | n/a | not claimed |
