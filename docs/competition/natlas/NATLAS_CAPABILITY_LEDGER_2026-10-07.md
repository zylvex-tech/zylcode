# N-ATLAS CAPABILITY LEDGER (Phase 1)

- **Date:** 2026-10-07
- **Branch:** `competition/natlas-2026` @ `2a6035e`
- **Method:** every row was established by reading the code and running the tests — **not** by
  the presence of a type, an interface or a button.

**Status vocabulary**

| Word | Means |
|---|---|
| `IMPLEMENTED` | Code exists on this branch. |
| `TESTED` | Exercised by a committed test that ran and passed. |
| `RUNTIME_VERIFIED` | Exercised against a **genuine** N-ATLAS instance, outcome captured. |
| `PARTIAL` | Present but incomplete in a stated way. |
| `BLOCKED` | Cannot be reached for a stated reason. Never a synonym for "not done". |
| `ABSENT` | Not built. |
| `PROPOSED` | Specified well enough to build; nothing built. |

> **`TESTED` ≠ `RUNTIME_VERIFIED`.** A passing unit test establishes nothing about the external
> service. Only `RUNTIME_VERIFIED` rows have been exercised against real N-ATLAS.

---

| ID | Capability | Existing? | Implemented? | Tested? | Runtime verified? | Required? | Gap | Evidence |
|---|---|---|---|---|---|---|---|---|
| NAT-001 | Provider registration | Yes | `PARTIAL` — isolated `NatlasTransport` seam; **not** registered in the product `ModelProvider` registry | `TESTED` | No | Yes | No first-class registration in the product provider list | `competition/natlas/transport.rs`; `NATLAS_INTEGRATION_DISCOVERY` §1.3 |
| NAT-002 | Configuration | Yes | `IMPLEMENTED` — env-only, no invented defaults | `TESTED` | n/a | Yes | none | `config.rs`; `REQUIRED_ENV` |
| NAT-003 | Authentication | Yes | `IMPLEMENTED` — local bearer; remote requires explicit header+prefix, no default | `TESTED` | No | Yes | No credential for a controlled endpoint | `local.rs:189`, `remote.rs:88` |
| NAT-004 | Connectivity | Yes | `IMPLEMENTED` — real HTTP, no fallback | `TESTED` (stub + dead-port) | `PARTIAL` | Yes | Genuine round trip captured via Gradio, **not** via this transport | `local.rs`; `EV-001…003` |
| NAT-005 | Model discovery / selection | Yes | `IMPLEMENTED` — `probe()` via `/api/tags` then `/v1/models` | `TESTED` | `PARTIAL` (`/healthz` observed live) | Yes | Model-selection UI | `local.rs:252` |
| NAT-006 | Request construction | Yes | `IMPLEMENTED` — OpenAI-compatible body, context labelled by path | `TESTED` | Yes (via Gradio) | Yes | none | `local.rs:85` |
| NAT-007 | Response parsing | Yes | `IMPLEMENTED` — pure `translate_openai_reply`, identity from server | `TESTED` | Yes (via Gradio) | Yes | none | `local.rs:117` |
| NAT-008 | Streaming | No | `ABSENT` — `stream:false`; remote engine supports SSE but we do not consume it | No | No | Optional | No incremental output | `local.rs:102` |
| NAT-009 | Error handling | Yes | `IMPLEMENTED` — typed `NatlasError`, never coerced to success | `TESTED` | Yes (502/401 observed) | Yes | none | `types.rs` |
| NAT-010 | Timeout handling | Yes | `IMPLEMENTED` — `NATLAS_TIMEOUT_MS` (our bound, not a claim) | `PARTIAL` | No | Yes | No dedicated timeout test | `config.rs:26` |
| NAT-011 | Retry behaviour | No | `ABSENT` | No | No | Optional | none | — |
| NAT-012 | Cancellation | No | `ABSENT` | No | No | Optional | none | — |
| NAT-013 | Usage / metadata | Yes | `IMPLEMENTED` — tokens, request id, finish reason captured | `TESTED` | No | Yes | — | `local.rs:159` |
| NAT-014 | Agent invocation | Yes | `IMPLEMENTED` — client → interpret → evidence | `TESTED` | No | Yes | Not driven by the real agent kernel | `client.rs`, `intent.rs` |
| NAT-015 | Repository context | Yes | `IMPLEMENTED` — `NatlasContextChunk`, path-labelled | `TESTED` | No | Yes | No automatic repo scan wired in | `types.rs`, `local.rs:88` |
| NAT-016 | Tool / workflow integration | Yes | `IMPLEMENTED` — intent → validated `TaskGraph` | `TESTED` | No | Yes | Execution stage `PROPOSED` | `intent.rs` |
| NAT-017 | Factory integration | Yes | `IMPLEMENTED` — handoff into the factory task graph | `TESTED` | No | Yes | Approved-write execution not wired | `intent.rs`; C1 §19 |
| NAT-018 | UI provider selection | `PARTIAL` | SDK + playground only; **not** in the main ZylCode UI | `TESTED` (SDK smoke) | No | Yes | No in-product provider picker | `apps/natlas-sdk/` |
| NAT-019 | Secret handling | Yes | `IMPLEMENTED` — `redact_secrets`, `RedactedNatlasConfig` | `TESTED` | n/a | Yes | none | `evidence.rs`, `config.rs` |
| NAT-020 | Logging / redaction | Yes | `IMPLEMENTED` — `Debug` redacted by construction | `TESTED` | n/a | Yes | none | `config.rs:43` |
| NAT-021 | Runtime evidence | **Yes** | `IMPLEMENTED` — EV-000…003 captured 2026-10-07 | n/a | **`RUNTIME_VERIFIED`** | Yes | Sustained invocation quota-limited | `evidence/EV-*`; `NATLAS_EVIDENCE_INDEX.md` |
| NAT-022 | External beta testing | Prepared | Package ready; **0 testers** | No | No | Yes | **No external tester** | `BETA_TEST_EVIDENCE_TEMPLATE.md` |
| NAT-023 | Demo workflow | `PARTIAL` | Steps defined; end-to-end not yet run against a controlled endpoint | No | No | Yes | Blocked on B1 | `VIDEO_DEMO_PLAN.md` |
| NAT-024 | Documentation | Yes | `IMPLEMENTED` — 23 documents | n/a | n/a | Yes | Sections awaiting real evidence | `docs/competition/natlas/` |
| NAT-025 | Submission evidence | `PARTIAL` | Package prepared; index now started | n/a | `PARTIAL` | Yes | Beta rows + video absent | `NATLAS_EVIDENCE_INDEX.md` |

---

## Summary

| Count | Value |
|---|---|
| Capabilities tracked | 25 |
| `RUNTIME_VERIFIED` | **1** (NAT-021) — up from 0 |
| `IMPLEMENTED` + `TESTED` | 16 |
| `ABSENT` | 4 (NAT-008, 011, 012, and part of 018) |
| Blocked on a controlled endpoint | NAT-001, 003, 004 (full), 014, 015, 016, 017, 023 |

**Honest headline:** the *architecture* around N-ATLAS is real and tested; the *genuine invocation*
was achieved once on 2026-10-07 and is now blocked on a controlled endpoint, not on access.
