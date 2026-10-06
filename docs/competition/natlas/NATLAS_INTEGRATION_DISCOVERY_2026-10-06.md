# N-ATLAS INTEGRATION DISCOVERY — 2026-10-06

**Phase:** C0 (branch isolation, discovery, safe scaffold)
**Branch:** `competition/natlas-2026`
**Branch parent:** `b4d13ab5e279524867d91e8e8c07a30b42a8f884` (`main`, six unpushed commits)
**Method:** read the repository. No claim below rests on memory, a file name or an assumption where
source could be read instead. Every "does not exist" is a statement about what was searched.

---

## 0. What this document decides

One question: **where does N-ATLAS plug in, without damaging ZylCode?**

The answer is a new, isolated boundary — not a modification of the existing provider router. The
reason is not aesthetic. The existing router contains a **silent synthetic-degradation path** that
would fabricate a response if it were reused for N-ATLAS (§1.3). Choosing the wrong seam here would
have produced the exact failure the directive forbids.

---

## 1. The existing provider architecture (read, not assumed)

### 1.1 What exists

| Concern | Where | Shape |
|---|---|---|
| Provider identity | `crates/zylcode-core/src/router.rs` | `ModelProvider` (closed enum: `OpenRouter`, `DeepSeek`, `Anthropic`, `LocalOllama`, `SyntheticOffline`) and a second `ProviderKind` (`Anthropic`, `OpenRouter`, `Ollama`, `SyntheticOffline`) |
| Per-provider config | `router.rs::ProviderConfig` | `kind`, `model`, `endpoint`, `timeout_ms`, `enabled`, `fallback_order`, `requires_api_key` |
| Router config | `router.rs::RouterConfig` | `primary_provider`, `fallback_provider`, `api_keys: HashMap`, `base_url_overrides: HashMap`, `provider_configs: Vec<ProviderConfig>` |
| Credential resolution | `RouterConfig::api_key` / `api_key_for_kind` | config map, then env (`OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, …) |
| HTTP dispatch | `TokenRouter::call_provider` | builds a per-provider JSON body, sets auth headers, `self.http.post(url)` |
| Response extraction | `TokenRouter::extract_text_and_usage` | per-provider `match` (Anthropic / Ollama / OpenAI-compatible) |
| Public entry | `TokenRouter::dispatch_prompt` | trimming → speculative cache → provider call → fallback |
| Scoring | `provider_scorecard.rs` | measured provider health, feeds routing |
| Capability model | `model_capabilities.rs` | declarative capability records |

There is **no provider trait**. Provider selection is a `match` on a closed enum. Adding a provider
means editing `router.rs`.

### 1.2 How a call actually flows

```
dispatch_prompt
  → context trim / compression
  → in-memory speculative cache probe            (may return a cached string)
  → if primary == SyntheticOffline: return synthetic_response(...)   ← fabrication path #1
  → on-disk vector cache probe                   (may return a cached string)
  → if no API keys present: return synthetic_response(...)           ← fabrication path #2
  → call_provider(primary) … on retryable error → call_provider(fallback)
  → if fallback also fails: synthetic_response(...)                  ← fabrication path #3
```

### 1.3 The hazard that decided the seam

`dispatch_prompt` returns a **synthetic response** in three places rather than an error
(`router.rs`, the `offline_fast` branch and the fallback tail). That behaviour is deliberate for the
product: it keeps offline and CI builds functional, and the provider is explicitly labelled
`synthetic-offline` in the scorecard.

It is **fatal for a competition integration.** If N-ATLAS were routed through this path and its
credentials were absent, the router would return a plausible-looking payload, and a careless caller
could present it as an N-ATLAS response. The directive forbids exactly that. Any seam that inherits
this path is disqualified.

---

## 2. Seam options considered

| Option | Description | Verdict |
|---|---|---|
| **A** | Add a `Natlas` variant to `ModelProvider` / `ProviderKind` and teach `call_provider` about it | **REJECTED.** Modifies a 1 806-line, heavily-tested core module; inherits the three fabrication paths; conflates N-ATLAS with the product's fallback semantics; and requires inventing the wire format *inside* a file the directive says to leave alone. |
| **B** | Point an existing provider's `endpoint` override at N-ATLAS and treat it as that provider | **REJECTED.** This is relabelling another provider as N-ATLAS — explicitly forbidden. It would also mis-report model identity in evidence. |
| **C** | A new, isolated `competition::natlas` boundary over a `NatlasTransport` trait, reusing the evidence and factory subsystems | **SELECTED.** Additive. No existing module is edited except one `pub mod` line. Failure is a first-class, labelled state. The wire format is confined to one trait implementation that does not exist yet. |

### 2.1 The selected seam, precisely

```text
NatlasClient::invoke(&NatlasRequest)
    → tokio timeout bound
    → NatlasTransport::send(&NatlasConfig, &NatlasRequest) -> Result<NatlasRawResponse, NatlasError>
         └─ the ONLY place bytes cross to the external service
    → interpret_raw: non-2xx → NatlasError::HttpStatus
    → parse_canonical_response: 2xx body → NatlasResponse | NatlasError::MalformedResponse
    → NatlasEvidence (redacted) → JSONL + evidence-graph node
    → (optional) NatlasEngineeringIntent::parse(text) → to_task_graph() → factory::TaskGraph
```

`NatlasTransport` is the seam. In Phase C0 its only shipped implementation is
`BlockedNatlasTransport`, which returns `NatlasError::BlockedNatlasAccess` with a stated reason.

---

## 3. Files and modules involved

### 3.1 Existing — read for this discovery, **not modified**

| Path | Used for |
|---|---|
| `crates/zylcode-core/src/router.rs` | the provider architecture being deliberately *not* extended |
| `crates/zylcode-core/src/model_capabilities.rs` | capability model shape |
| `crates/zylcode-core/src/provider_scorecard.rs` | measured provider health (not used in C0) |
| `crates/zylcode-core/src/claim.rs` | `EvidenceKind` for evidence refs |
| `crates/zylcode-core/src/evidence_graph.rs` | `EvidenceNode`, `NodeKind`, `EvidenceGraph` |
| `crates/zylcode-core/src/factory/graph.rs` | `TaskGraph`, `TaskNode`, `TaskKind`, `TaskAction` |
| `crates/zylcode-core/src/failure.rs` | failure taxonomy (design reference) |
| `crates/zylcode-core/src/intelligence/*` | repository context source for a real request |
| `crates/zylcode-mcp/src/*` | the gated tool dispatch the factory uses |

### 3.2 New — created in Phase C0

| Path | Purpose |
|---|---|
| `crates/zylcode-core/src/competition/mod.rs` | competition namespace |
| `crates/zylcode-core/src/competition/natlas/mod.rs` | boundary module + runtime status |
| `…/natlas/config.rs` | `NatlasConfig`, `RedactedNatlasConfig`, `NatlasConfigError` |
| `…/natlas/types.rs` | `NatlasRequest`, `NatlasResponse`, `NatlasError`, `NatlasStatus`, `NatlasUsage` |
| `…/natlas/transport.rs` | `NatlasTransport` trait, `BlockedNatlasTransport` |
| `…/natlas/client.rs` | `NatlasClient`, `interpret_raw`, `parse_canonical_response` |
| `…/natlas/evidence.rs` | `NatlasEvidence`, `redact_secrets` |
| `…/natlas/intent.rs` | `NatlasEngineeringIntent` → `TaskGraph` handoff |
| `crates/zylcode-core/tests/natlas_boundary.rs` | deterministic boundary tests (test double) |
| `crates/zylcode-core/src/lib.rs` | **one added line:** `pub mod competition;` |

---

## 4. What can be reused unchanged

| Existing capability | Reused as |
|---|---|
| `evidence_graph::{EvidenceNode, NodeKind, EvidenceGraph}` | an N-ATLAS call is recorded as a `Decision` node carrying provider/model identity; the graph stays hash-chained and traversable |
| `claim::{EvidenceKind}` | the N-ATLAS evidence reference kind on graph nodes |
| `factory::graph::{TaskGraph, TaskNode, TaskKind, TaskAction}` | the **handoff target**: a parsed intent becomes a validated, dependency-chained graph the existing `FactoryRunner` consumes |
| `factory::runner::FactoryRunner` | executes the graph, records evidence, honours approval gates — **unmodified** |
| `intelligence/*` | supplies the repository context that goes into `NatlasRequest.context` |
| `zylcode_mcp::dispatch` | the gated tool path the factory uses for the bounded action |
| `serde`, `chrono`, `tokio`, `reqwest` | already workspace dependencies; **no new dependency was added** |

The design principle: **one fact, one home.** Evidence lives in the evidence graph and the claim
store; the N-ATLAS module produces records and references, and does not duplicate those stores.

---

## 5. What must NOT be modified

* `router.rs`, `router/*`, `model_capabilities.rs`, `provider_scorecard.rs` — the product's provider
  path stays exactly as it is. The competition integration is additive.
* `factory/*`, `claim.rs`, `evidence_graph.rs`, `failure.rs` — reused, not edited.
* The pre-existing **foreign working-tree changes** (`ci.yml`, `release.yml`, `agent.rs`, `cli.rs`,
  `router.rs`, `terminal.rs`, and the untracked `RECONCILIATION_FORENSIC_REPORT.md`, `tasks/`,
  `vc_inspect.py`, `run_tests.bat`, `COMPLETE_AUDIT_REPORT.html`, `.git-msg.txt`,
  `filesystem.rs.backup`). They are not this phase's work and must not be staged or committed.
* `main`. All competition work stays on `competition/natlas-2026`.

---

## 6. Unknown N-ATLAS technical requirements

None of the following is known to this project. Each is recorded as an unknown rather than filled
with a plausible guess:

| # | Unknown | Why it blocks |
|---|---|---|
| U1 | Base URL / hostname | required before any call |
| U2 | Request path | deliberately **required** in config so it is never defaulted to a guess |
| U3 | Authentication scheme (header name, prefix, token format) | cannot be invented |
| U4 | Request body schema | cannot be invented |
| U5 | Response schema (where the text, usage and request id live) | the parser expects **our** contract; the N-ATLAS→ours translation is unimplemented |
| U6 | Model identifiers | must not be invented; `NATLAS_MODEL` is required |
| U7 | Rate limits, quotas, max context | affects batching and trimming |
| U8 | Streaming support | affects whether `dispatch_stream`-style handling is needed |
| U9 | Whether an official SDK exists | would change the transport implementation |
| U10 | Terms of use, data residency, permitted use | an institutional/legal prerequisite, not an engineering one |
| U11 | Whether access is gated behind approval or a competition credential | the immediate blocker |

---

## 7. Blockers

| Blocker | State | Consequence |
|---|---|---|
| No N-ATLAS documentation, endpoint or credentials (U1–U6, U11) | **BLOCKED_NATLAS_ACCESS** | no real transport; runtime integration cannot be proven |
| Unknown auth scheme (U3) | blocked | a transport written today would be a guess |
| Unknown request/response schema (U4, U5) | blocked | wire translation is unimplemented by design |

**Nothing was invented to route around these.** The boundary exists; the runtime is blocked and
labelled. This is the directive's stated preference over fabrication.

---

## 8. Proposed end-to-end competition path

The chain the competition entry is meant to demonstrate, with the honest state of each link today:

```text
Developer
   ↓   (intent captured)
ZylCode                                   IMPLEMENTED  (CLI / desktop / factory intake)
   ↓
N-ATLAS integration/provider boundary     SCAFFOLDED   (trait + config + client + evidence)
   ↓   ← BLOCKED_NATLAS_ACCESS here
structured engineering intent             IMPLEMENTED · TESTED  (NatlasEngineeringIntent::parse)
   ↓
ZylCode repository intelligence/context   IMPLEMENTED · TESTED  (intelligence/*, C-01..C-06)
   ↓
task/factory workflow                     IMPLEMENTED · TESTED  (TaskGraph + FactoryRunner)
   ↓
controlled tools/execution                IMPLEMENTED · TESTED  (zylcode_mcp::dispatch, gated)
   ↓
tests/verification                        IMPLEMENTED · TESTED  (factory Verify action)
   ↓
evidence + result                         IMPLEMENTED · TESTED  (evidence graph + claim store)
```

**Read the chain honestly:** every link except the N-ATLAS boundary is already real and tested. The
competition proposition therefore depends on exactly one thing that does not exist yet — a verified
N-ATLAS transport. That is a good place to be: the risk is concentrated in one known place rather
than smeared across the architecture.

### 8.1 The first bounded vertical slice (proposed for C1)

One developer task, small, observable, reproducible:

1. Operator runs a single ZylCode command with a development request.
2. ZylCode retrieves repository context from `intelligence/*`.
3. `NatlasClient` sends `NatlasRequest` through the **real** transport.
4. The response text is parsed as `NatlasEngineeringIntent`.
5. `to_task_graph()` produces a validated graph.
6. `FactoryRunner` executes the side-effect-free steps and **parks at the approval gate**.
7. A human approves; the bounded action runs through `zylcode_mcp::dispatch`.
8. A verification command runs; the claim is promoted only on a real exit code.
9. `NatlasEvidence` is appended and linked into the evidence graph.
10. The operator inspects the result and the evidence chain.

Step 6 is deliberate: the first slice stops at a human gate rather than modifying a repository
unattended. Autonomous repository modification is explicitly out of scope.

### 8.2 What C1 must not do

Reuse `dispatch_prompt`'s synthetic path. Implement a wire format from assumption. Present a
mock-driven run as evidence.

---

## 9. Discovery conclusion

The safest genuine integration seam is a **new boundary module over a `NatlasTransport` trait**,
reusing the existing evidence graph and factory task graph unchanged, leaving the product's provider
router untouched. Runtime integration is **`BLOCKED_NATLAS_ACCESS`** pending N-ATLAS documentation,
endpoint and credentials.

No N-ATLAS behaviour was simulated, described as real, or assumed.
