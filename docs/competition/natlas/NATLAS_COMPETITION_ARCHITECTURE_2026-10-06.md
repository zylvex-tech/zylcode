# N-ATLAS COMPETITION ARCHITECTURE — 2026-10-06

**Phase:** C0
**Branch:** `competition/natlas-2026`
**Scope:** the architecture of `ZylCode × N-ATLAS Developer Bridge`.
**Rule obeyed throughout:** a component is described with the status it has, not the status it is
expected to have. `PROPOSED` means nothing was built.

---

## 1. The concept

**ZylCode × N-ATLAS Developer Bridge** is *not* an N-ATLAS chat interface. A chat interface would
put a model in front of a developer and stop. This puts a model **inside** an engineering workflow
that already knows how to plan, execute, verify and prove.

```text
Developer
    ↓
ZylCode
    ↓
N-ATLAS integration / provider boundary
    ↓
structured engineering intent
    ↓
ZylCode repository intelligence / context
    ↓
task / factory workflow
    ↓
controlled tools / execution
    ↓
tests / verification
    ↓
evidence + result
```

The proposition: **use Nigeria's N-ATLAS AI capability as a genuine intelligence component inside an
AI-native software-engineering workflow.**

---

## 2. Component map and honest status

| Layer | Component | Status |
|---|---|---|
| Intake | Developer intent capture (CLI / desktop / factory job) | `IMPLEMENTED` (product) |
| Boundary | `NatlasConfig` | `IMPLEMENTED` · `TESTED` |
| Boundary | `NatlasTransport` trait | `IMPLEMENTED` |
| Boundary | `BlockedNatlasTransport` (C0 runtime) | `IMPLEMENTED` · `TESTED` |
| Boundary | **Real N-ATLAS transport** | **`BLOCKED_NATLAS_ACCESS`** |
| Boundary | `NatlasClient` (timeout, interpret, evidence) | `IMPLEMENTED` · `TESTED` (test double) |
| Boundary | `parse_canonical_response` | `IMPLEMENTED` · `TESTED` |
| Intent | `NatlasEngineeringIntent::parse` | `IMPLEMENTED` · `TESTED` |
| Intent | `to_task_graph()` handoff | `IMPLEMENTED` · `TESTED` |
| Context | Repository intelligence (`intelligence/*`) | `IMPLEMENTED` · `TESTED` (product) |
| Workflow | `factory::TaskGraph` + `FactoryRunner` | `IMPLEMENTED` · `TESTED` (product) |
| Execution | Gated tool dispatch (`zylcode_mcp::dispatch`) | `IMPLEMENTED` · `TESTED` (product) |
| Verification | Factory `Verify` action → claim promotion | `IMPLEMENTED` · `TESTED` (product) |
| Evidence | `NatlasEvidence` + JSONL + evidence-graph node | `IMPLEMENTED` · `TESTED` |
| Evidence | Claim store linkage | `IMPLEMENTED` (product); N-ATLAS linkage `TESTED` via test double |
| Validation | External beta testing | `PROPOSED` — no testers yet |
| Submission | Competition video / final entry | **not started** (explicitly out of C0 scope) |

---

## 3. The boundary

### 3.1 One crossing point

```rust
#[async_trait]
pub trait NatlasTransport: Send + Sync {
    async fn send(&self, config: &NatlasConfig, request: &NatlasRequest)
        -> Result<NatlasRawResponse, NatlasError>;
}
```

Everything above the trait is deterministic ZylCode code. Everything below it is N-ATLAS's contract,
which is unknown. That is the entire reason the boundary exists in this shape: **the unknown is
isolated to one implementation.**

### 3.2 Types

| Type | Role |
|---|---|
| `NatlasConfig` | `base_url`, `request_path`, `model`, `api_key`, `timeout_ms` — all external, none defaulted |
| `RedactedNatlasConfig` | log-safe view; carries key *presence and length* only |
| `NatlasRequest` | `intent`, `system`, `context[]`, `max_tokens` |
| `NatlasContextChunk` | one repository excerpt with its path |
| `NatlasResponse` | `text`, `model`, `request_id`, `usage`, `finish_reason` |
| `NatlasError` | `NotConfigured`, `BlockedNatlasAccess`, `Transport`, `Timeout`, `HttpStatus`, `MalformedResponse` |
| `NatlasStatus` | `NotConfigured`, `BlockedNatlasAccess`, `Succeeded`, `Failed` |
| `NatlasEvidence` | the non-secret record of one invocation |

`NatlasError` has **no catch-all variant**. A failure that cannot be stated cannot be reported
honestly, so the type does not permit one.

### 3.3 Why the response contract is ours

The client parses a schema **ZylCode defines and asks the model to return**. It is not a claim about
N-ATLAS's wire format. This separation is what lets the parsing, evidence and handoff layers be
built and tested today, while the N-ATLAS-specific translation stays honestly unimplemented.

---

## 4. Data flow — the first bounded slice

```text
1  Developer request                       → captured as NatlasRequest.intent
2  Repository context                      → intelligence/* → NatlasRequest.context
3  NatlasClient::invoke                    → timeout bound → NatlasTransport::send
                                             └── BLOCKED_NATLAS_ACCESS (C0)
4  Response → NatlasResponse               → parsed, or MalformedResponse
5  Response text → NatlasEngineeringIntent → strict schema, rejected if unmet
6  Intent → TaskGraph                      → validated, dependency-chained
7  TaskGraph → FactoryRunner               → side-effect-free steps run; parks at the approval gate
8  Human approval                          → real TaskAction::Approval
9  Bounded action                          → zylcode_mcp::dispatch (gated, evidence-recorded)
10 Verification                            → real exit code → claim promoted only if it matches
11 Evidence                                → NatlasEvidence JSONL + evidence-graph Decision node
12 Inspection                              → operator reads the result and the provenance chain
```

Steps 1, 2, 4–12 are implemented and tested in the product or in this phase. **Step 3 is blocked.**

### 4.1 Why the slice stops at a human gate

Autonomous unrestricted repository modification is out of scope. The first slice deliberately parks
at step 8 so a human sees the plan before anything is written. That is also the more impressive
demonstration: a system that knows when to stop is a stronger claim than one that acts without
asking.

---

## 5. Evidence model

Every genuine invocation will be able to record, without any secret:

`timestamp` · `provider` · `model` · `request_id` · `request_classification` · `status` ·
`http_status` · `latency_ms` · `success` · `error_code` · redacted `error_detail` ·
`resulting_operation` · `verification` · `prompt_chars` · `response_chars` · `secret_redactions` ·
`unknowns`

Two properties make this trustworthy rather than decorative:

1. **Secrets are redacted before storage**, and the number of redactions is itself recorded, so a
   reader can see that redaction happened instead of trusting it silently.
2. **`unknowns` is a field.** A record that can only state what is known cannot be trusted about what
   is not.

The record projects into the existing evidence graph as a `Decision` node carrying provider and model
identity — not a `ToolCall`, because no ZylCode tool ran. It is the same graph the product already
uses, so the N-ATLAS trail is traversable back to the originating intent.

---

## 6. What is reused, unchanged

`evidence_graph` · `claim` · `factory::graph` · `factory::runner` · `intelligence/*` ·
`zylcode_mcp::dispatch`. No new dependency was added. No existing module was edited except one
`pub mod competition;` line in `lib.rs`.

## 7. What is deliberately not built

* A real N-ATLAS transport — blocked on the interface.
* Any wire-format assumption.
* Autonomous repository modification.
* A UI surface for the bridge (`PROPOSED`).
* Streaming responses (`PROPOSED`).
* Multi-provider routing for N-ATLAS (`PROPOSED`; and it must never inherit the synthetic path).

## 8. Non-goals

Redesigning ZylCode. Completing the wider roadmap. Changing the product's provider behaviour.
Claiming competition compliance. Producing the competition video.
