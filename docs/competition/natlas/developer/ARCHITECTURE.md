# ARCHITECTURE — the ZylCode × N-ATLAS Developer Bridge

---

## The shape

```
Developer
   |  (instruction, any supported language)
   v
ZylCode developer toolkit / playground        apps/natlas-sdk
   |
   v
Repository intelligence                       existing ZylCode subsystem
   |  bounded, path-labelled context
   v
N-ATLAS boundary                              NatlasTransport
   |
   v
REAL N-ATLAS runtime                          local model, OpenAI-compatible protocol
   |
   v
NatlasResponse                                canonical response (ours)
   |
   v
strict structured engineering intent          NatlasEngineeringIntent
   |
   v
schema validation                             strict; rejection on mismatch
   |
   v
TaskGraph                                     dependency-chained, validated
   |
   v
HUMAN APPROVAL GATE                           required; a real gate, not a label
   |
   v
allowlisted tools                             zylcode_mcp::dispatch + permission gate
   |
   v
repository / file / tool operation
   |
   v
real verification                             an actual command, an actual exit code
   |
   v
evidence graph + JSONL                        hash-chained, secret-redacted
   |
   v
developer-visible result
```

---

## The one decision that matters: where the boundary sits

Phase C0 evaluated three seams and chose the third:

| Option | Verdict |
|---|---|
| Extend the product's closed `ModelProvider` enum | **Rejected** — inherits the router's silent synthetic-degradation paths, which is exactly the fabrication hazard |
| Override the router's endpoint | **Rejected** — amounts to relabelling another provider as N-ATLAS |
| **A new, isolated `NatlasTransport` boundary** | **Selected** — additive, testable, and unable to fake a success |

---

## The boundary

```rust
#[async_trait]
pub trait NatlasTransport: Send + Sync {
    async fn send(
        &self,
        config: &NatlasConfig,
        request: &NatlasRequest,
    ) -> Result<NatlasRawResponse, NatlasError>;
}
```

Everything **above** the trait is deterministic and testable without N-ATLAS: parsing, evidence,
redaction, the intent handoff. Everything **below** it is the network.

### Implementations

| Type | Purpose | State |
|---|---|---|
| `BlockedNatlasTransport` | the truthful blocked state | `IMPLEMENTED` · `TESTED` |
| `LocalNatlasTransport` | real call to a local, OpenAI-compatible runtime | `IMPLEMENTED` · `TESTED` |
| `HttpNatlasTransport` | remote adapter; **contract UNVERIFIED** | `IMPLEMENTED` · `CONTRACT_UNVERIFIED` |

There is **no fallback between them**. Choosing a transport is an explicit act.

---

## Why the OpenAI-compatible protocol is not an invented interface

`LocalNatlasTransport` speaks the OpenAI-compatible chat-completions protocol. That protocol is a
public, widely implemented standard (Ollama, llama.cpp, vLLM, and others). It is **not** an
N-ATLAS-specific detail, and the request/response schema we parse remains **ZylCode's own**.

What we never invent: an N-ATLAS endpoint, an N-ATLAS auth scheme, an N-ATLAS model id, or an
N-ATLAS response schema.

---

## Layers and their honest status

| Layer | Where | Status |
|---|---|---|
| Configuration (env-only, no invented defaults) | `natlas/config.rs` | `TESTED` |
| Request / response / error / status types | `natlas/types.rs` | `TESTED` |
| Boundary trait | `natlas/transport.rs` | `IMPLEMENTED` |
| Local transport | `natlas/local.rs` | `TESTED` |
| Remote adapter | `natlas/remote.rs` | `IMPLEMENTED` · `CONTRACT_UNVERIFIED` |
| Client: invoke -> interpret -> evidence | `natlas/client.rs` | `TESTED` |
| Evidence + redaction | `natlas/evidence.rs` | `TESTED` |
| Intent -> TaskGraph handoff | `natlas/intent.rs` | `TESTED` |
| Runtime probe | `natlas/local.rs` | `TESTED` |
| Developer SDK | `apps/natlas-sdk/src/index.ts` | `TESTED` |
| Playground | `apps/natlas-sdk/playground/` | `IMPLEMENTED` |
| **Genuine N-ATLAS invocation** | — | **`BLOCKED_NATLAS_ACCESS`** |

---

## Separability

The competition work is additive. The only edit to existing code is **one line** in
`crates/zylcode-core/src/lib.rs` (`pub mod competition;`). The whole boundary can be removed
without touching the product.

---

## Data flow of one invocation

1. `NatlasConfig` is read from the environment (no defaults).
2. `NatlasRequest` carries the instruction, the system preamble and path-labelled context.
3. `NatlasClient::invoke` enforces the timeout, classifies the request, and calls the transport.
4. The transport performs the HTTP call and returns `NatlasRawResponse`.
5. `interpret_raw` rejects a non-2xx status; `parse_canonical_response` rejects a malformed body.
6. `NatlasEvidence` records the outcome — with the key redacted and the redaction counted.
7. The intent parser turns the text into `NatlasEngineeringIntent`, strictly.
8. `to_task_graph` produces a validated, dependency-chained `TaskGraph`.
9. The graph parks at the human approval gate.
