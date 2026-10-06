# N-ATLAS EVIDENCE CHECKLIST — 2026-10-06

**Phase:** C0
**Purpose:** the evidence a competition submission must carry, itemised, with its honest state today.

A box is ticked only when the artefact **exists and was produced by a real run**. Nothing here is
ticked because it is planned, expected, or almost done.

Legend: ✅ present · ⬜ not yet · ⛔ blocked · 🚫 must never be faked

---

## A. Boundary evidence (achievable without N-ATLAS access)

| # | Item | State | Where |
|---|---|---|---|
| A1 | Configuration reads from the environment with no invented defaults | ✅ | `competition/natlas/config.rs` + tests |
| A2 | Missing configuration produces an explicit not-configured state | ✅ | `config.rs`, `natlas_boundary.rs` |
| A3 | Secret never appears in Debug output or evidence | ✅ | redaction tests |
| A4 | Blocked state is labelled `BLOCKED_NATLAS_ACCESS`, not "failed" | ✅ | `transport.rs`, `client.rs` tests |
| A5 | Response parsing accepts the contract and rejects malformed input | ✅ | `client.rs` tests |
| A6 | Non-2xx status is not coerced into a response | ✅ | `natlas_boundary.rs` |
| A7 | Timeout is enforced by the client | ✅ | `natlas_boundary.rs` |
| A8 | Evidence is written as JSONL and contains no secret | ✅ | `natlas_boundary.rs` |
| A9 | Intent parsing is strict and rejects unknown steps | ✅ | `intent.rs` tests |
| A10 | Handoff produces a valid, dependency-chained factory graph | ✅ | `intent.rs`, `natlas_boundary.rs` |

**All of A is rung L2 at best.** It proves the boundary, not the integration.

---

## B. Integration evidence (requires real access)

| # | Item | State | Notes |
|---|---|---|---|
| B1 | Real base URL, path and model recorded (names, not secrets) | ⛔ | requires N-ATLAS documentation |
| B2 | A captured real response with provider/model identity | ⛔ | requires access |
| B3 | Provider-supplied request id, if the service returns one | ⛔ | requires access |
| B4 | Measured latency of a real call | ⛔ | requires access |
| B5 | `NatlasEvidence` record for a real call, in the evidence log | ⛔ | requires access |
| B6 | Evidence-graph node linking the real call to the originating intent | ⛔ | requires access |
| B7 | Proof that no secret was captured | ⬜ | redaction exists; must be re-verified on a real record |

---

## C. End-to-end evidence (rung L4)

| # | Item | State | Notes |
|---|---|---|---|
| C1 | Real N-ATLAS response parsed into `NatlasEngineeringIntent` | ⛔ | depends on B |
| C2 | Intent converted into a validated `TaskGraph` | ✅ (with a test double) | must be redone with a real response |
| C3 | Bounded action executed through `zylcode_mcp::dispatch` | ⛔ | depends on B |
| C4 | Verification command run; claim promoted only on a matching exit code | ⛔ | depends on B |
| C5 | Traversable evidence chain: intent → decision → action → result → verification | ⛔ | depends on B |
| C6 | No regression in the workspace test battery | ⬜ | must be run and captured on the final slice |

---

## D. External validation (rung L5)

| # | Item | State | Notes |
|---|---|---|---|
| D1 | ≥1 real tester recorded in the beta log | 🚫 **not fabricated** | real people only |
| D2 | One bounded task per tester, on a named repository | ⬜ | |
| D3 | Task outcome recorded as observed, including failures | ⬜ | |
| D4 | Measured elapsed time | ⬜ | |
| D5 | Tester rating, observations, written feedback | ⬜ | |
| D6 | Issues discovered, with resolution or explicit non-resolution | ⬜ | |

---

## E. Governance and honesty evidence

| # | Item | State |
|---|---|---|
| E1 | Discovery document naming the seam and the unknowns | ✅ |
| E2 | Validation protocol written **before** any real call | ✅ |
| E3 | Test doubles explicitly named and excluded from evidence | ✅ |
| E4 | No endpoint / auth / model id / response shape invented | ✅ (none present in the code) |
| E5 | Product router untouched; integration additive | ✅ |
| E6 | Branch isolation verified; `main` unchanged; nothing pushed | ✅ |
| E7 | Statement of what is still unknown, maintained | ✅ |

---

## F. Repository hygiene

| # | Item | State |
|---|---|---|
| F1 | All competition work on `competition/natlas-2026` | ✅ |
| F2 | Foreign/pre-existing working-tree changes untouched | ✅ |
| F3 | Competition code separable by deleting `competition/` + one `lib.rs` line | ✅ |
| F4 | No new dependency added | ✅ |

---

## G. Submission artefacts (out of scope for C0)

| # | Item | State |
|---|---|---|
| G1 | Competition video | 🚫 not started (explicitly out of scope) |
| G2 | Final written submission | 🚫 not started |
| G3 | Screenshots of a real run | 🚫 must be real when produced |
| G4 | Benchmark results | 🚫 must be measured when produced |

---

## The single sentence this checklist protects

> Everything in **A** is real today. Nothing in **B**, **C** or **D** exists yet, and none of it may
> be simulated to appear otherwise.
