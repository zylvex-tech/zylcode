# EVIDENCE AND VALIDATION

How to prove what actually happened — and how to tell a real result from a test double.

---

## 1. The proof ladder

| Rung | Means |
|---|---|
| **L0** | Nothing exists. |
| **L1** | Code exists. |
| **L2** | A deterministic test ran, using a **test double**. |
| **L3** | A **genuine N-ATLAS invocation** occurred, with the outcome captured. |
| **L4** | A genuine invocation drove a bounded engineering operation, plus real verification. |
| **L5** | External developers independently reproduced it. |

**A mock/test-double result can never exceed L2.** This is the rule that keeps the project honest.

Current level: **L2**, rising to L3 the moment a real model answers.

---

## 2. What an evidence record contains

Every invocation writes one JSONL line:

| Field | Meaning |
|---|---|
| `timestamp` | when it happened |
| `provider` | `natlas` |
| `model` | **as reported by the server** |
| `request_id` | the provider's id, when supplied |
| `request_classification` | a coarse label (ours, a keyword heuristic) |
| `status` | `succeeded` / `failed` |
| `http_status` | the raw status, when one was received |
| `latency_ms` | measured, not estimated |
| `success` | boolean |
| `error_code` | a stable code when it failed |
| `prompt_chars` / `response_chars` | sizes, not contents |
| `secret_redactions` | how many secrets were scrubbed |

It never contains the API key. It never contains the full prompt.

---

## 3. Evidence chain

For a full workflow the chain is:

```
request -> N-ATLAS invocation -> model response -> intent
        -> approval -> action -> result -> verification
```

Each link is a node; `NatlasEvidence::as_graph_node` produces a graph node carrying the provider
and model identity, and the existing `EvidenceGraph` is hash-chained and verifiable
(`verify_integrity`).

---

## 4. How to tell a real result from a test double

| Signal | Real | Test double |
|---|---|---|
| Model identity | whatever the server reports | contains `TEST DOUBLE` / `stub` |
| Where it came from | a live HTTP call | `MockNatlasTransport`, `StubServer`, or the playground's offline demo |
| Label | — | **always labelled** in the UI and in the docs |
| Counts as competition evidence | yes, once captured | **never** |

The playground shows a `TEST DOUBLE` badge and a warning banner whenever the offline demo is used.
The Rust integration tests name their double `StubServer` / `MockNatlasTransport`.

---

## 5. Reproducing the current test evidence

```bash
# unit tests inside the boundary (no network, no model)
cargo test -p zylcode-core --lib competition::

# boundary tests (test double)
cargo test -p zylcode-core --test natlas_boundary

# runtime transport tests (local stub HTTP server, real HTTP round trip)
cargo test -p zylcode-core --test natlas_runtime

# SDK smoke test
cd apps/natlas-sdk && node --experimental-strip-types test/smoke.mjs
```

Run the canonical suites **sequentially** where a global counter is involved (see
`NATLAS_VALIDATION_PROTOCOL_2026-10-06.md` §6.1 for the known router-counter flake).

---

## 6. What counts as proof of integration

| Claim | Minimum proof |
|---|---|
| "The boundary is implemented" | L1 + tests |
| "The transport works" | L2 (stub) is enough for the *mechanism*, not for N-ATLAS |
| "N-ATLAS integration works" | **L3** — a genuine invocation with captured evidence |
| "The workflow works" | **L4** — a genuine invocation driving a bounded operation plus real verification |
| "It works for other developers" | **L5** — external reproduction |

---

## 7. Falsification

If you find a path where the toolkit reports success without a genuine invocation, that is a
**defect of the highest severity**. Report it with the exact steps. The correct behaviour is
always to report the failure truthfully.
