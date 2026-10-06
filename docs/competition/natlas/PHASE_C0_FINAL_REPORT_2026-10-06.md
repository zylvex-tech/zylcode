# ZYLCODE — N-ATLAS 2026 COMPETITION
# PHASE C0 FINAL REPORT — BRANCH ISOLATION, DISCOVERY & SAFE SCAFFOLD

- **Date:** 2026-10-06
- **Branch:** `competition/natlas-2026`
- **Phase status:** **COMPLETE — STOPPED FOR OWNER REVIEW**
- **N-ATLAS integration status:** **`BLOCKED_NATLAS_ACCESS`**
- **Pushed:** **NO — nothing was pushed.**
- **Competition:** N-ATLAS / National AI Innovation Challenge — Track: Innovation & Enterprise — Problem area: Developer Infrastructure
- **Working concept:** "ZylCode × N-ATLAS Developer Bridge"

> Truthfulness rule observed throughout: N-ATLAS has **never been contacted**. No endpoint,
> auth scheme, model identifier, SDK or wire format has been assumed or invented. No mock is
> counted as integration. Where the interface is undocumented, the boundary is present and the
> runtime path is marked `BLOCKED_NATLAS_ACCESS`.

---

## 1. Branch created

`competition/natlas-2026`

Verified: `git branch --show-current` → `competition/natlas-2026`.
No competition implementation exists on `main`. `main` was not modified.

## 2. Exact branch parent SHA

`b4d13ab5e279524867d91e8e8c07a30b42a8f884` — the tip of `main` at branch time
(`docs(gate): final owner gate report for the factory foundation wave`).

The branch was created at that exact commit; `main` was left untouched.

## 3. Current HEAD

- Engineering tip: **`f4724b90206d55c77710c751c1e1b3a913003b9a`**
- This report is added as the Phase C0 **closing commit** and becomes the new tip of
  `competition/natlas-2026`. The exact resulting HEAD is stated in the owner-facing report
  delivered alongside this file.

## 4. Files created (15)

**Implementation (8) — `crates/zylcode-core/src/competition/`**
1. `mod.rs`
2. `natlas/mod.rs`
3. `natlas/config.rs`
4. `natlas/types.rs`
5. `natlas/transport.rs`
6. `natlas/client.rs`
7. `natlas/evidence.rs`
8. `natlas/intent.rs`

**Tests (1)**
9. `crates/zylcode-core/tests/natlas_boundary.rs`

**Documentation (6) — `docs/competition/natlas/`**
10. `README.md`
11. `NATLAS_INTEGRATION_DISCOVERY_2026-10-06.md`
12. `NATLAS_COMPETITION_ARCHITECTURE_2026-10-06.md`
13. `NATLAS_VALIDATION_PROTOCOL_2026-10-06.md`
14. `NATLAS_EVIDENCE_CHECKLIST_2026-10-06.md`
15. `BETA_TEST_EVIDENCE_TEMPLATE.md` (deliberately empty — 0 rows)

## 5. Files modified (1)

1. `crates/zylcode-core/src/lib.rs` — **one added line**: `pub mod competition;` (line 14).

> `lib.rs` also carries a **foreign, uncommitted hunk** in the working tree. That hunk was
> deliberately **excluded** from staging (staged via `git hash-object -w` +
> `git update-index --cacheinfo`), so it remains untouched and uncommitted. The committed
> `lib.rs` differs from `HEAD~4` by exactly the one line above.

## 6. Commits created

Four engineering commits (plus this closing report commit):

| # | SHA | Message |
|---|-----|---------|
| 1 | `dfa47a1` | `feat(competition): isolated N-ATLAS integration boundary` |
| 2 | `f79c142` | `test(competition): deterministic N-ATLAS boundary tests with a named test double` |
| 3 | `9d6a78b` | `docs(competition): N-ATLAS discovery, architecture, validation and evidence` |
| 4 | `f4724b9` | `docs(competition): record the pre-existing router test flake exposed by this branch` |
| 5 | *(this file)* | `docs(competition): Phase C0 final report` |

All five are narrow and coherent. No historical commit was amended.

## 7. Tests added

**35 tests** — all new, all competition-scoped:

- **18 unit tests** inside the module:
  - `client.rs` — 4 (parse well-formed, reject missing model identity, reject non-JSON/empty text, plus raw interpretation)
  - `evidence.rs` — 6 (redaction, short-secret guard, failure redaction, blocked≠failed labelling, JSONL round-trip, graph-node identity)
  - `intent.rs` — 6 (parse well-formed, fenced parse, reject empty steps, reject unknown kinds, reject missing description, valid handoff graph)
  - `transport.rs` — 2 (blocked transport always returns `BlockedNatlasAccess`; never returns a body)
- **17 integration tests** in `tests/natlas_boundary.rs`, driven by an explicitly named
  `MockNatlasTransport` **TEST DOUBLE** (behaviours: `Respond` / `Fail` / `Stall`).

## 8. Tests executed

Re-run at report time, on this branch, this machine (Rust 1.97.1, Windows):

```
cargo test -p zylcode-core --lib competition::
cargo test -p zylcode-core --test natlas_boundary
cargo clippy -p zylcode-core -- -D warnings
```

## 9. Exact pass / fail / ignored totals

| Suite | Passed | Failed | Ignored |
|-------|--------|--------|---------|
| `--lib competition::` | **18** | 0 | 0 |
| `--test natlas_boundary` | **17** | 0 | 0 |
| **Competition total** | **35** | **0** | **0** |
| Full `zylcode-core` lib suite | 459 | 0 | — |
| Clippy `-D warnings` | clean | — | — |

Full-phase battery earlier in the phase: **764 passed / 1 failed / 1 ignored**. The single
failure is the **pre-existing, intermittently-flaky**
`router::tests::d1_vector_cache_cross_prompt_contamination_guard` — see item 13.

## 10. N-ATLAS integration status

**`BLOCKED_NATLAS_ACCESS`**

Meaning precisely: the integration **boundary is SCAFFOLDED and TESTED** against a test double,
but **no runtime integration exists and none has been attempted**, because the N-ATLAS wire
contract (endpoint path, auth scheme, request/response schema, model identifiers) is
undocumented. `NatlasClient::invoke` against the real transport would return
`NatlasError::BlockedNatlasAccess` — it cannot succeed by accident.

## 11. Integration seam selected

```
NatlasTransport::send(&self, config: &NatlasConfig, request: &NatlasRequest)
    -> Result<NatlasRawResponse, NatlasError>
```

A **new, additive boundary** — *not* a modification of the existing provider router.

- **Rejected:** extending the closed `ModelProvider` enum (would inherit the router's three
  silent synthetic-degradation paths — i.e. the fabrication hazard).
- **Rejected:** endpoint-override on the existing router (would amount to relabelling another
  provider as N-ATLAS).
- **Selected:** an isolated transport trait whose wire translation is the *only* blocked part,
  while parsing, evidence, redaction and the factory handoff are ZylCode's own contract and are
  deterministically testable today.

## 12. N-ATLAS information / credentials still required

Before any runtime integration can be attempted (see `NATLAS_INTEGRATION_DISCOVERY` §6, U1–U11):

1. Base URL of the N-ATLAS API.
2. Exact request path (no default is invented — it is a required config field).
3. Auth scheme (header name, token format, any signing).
4. Request body schema (fields, required vs optional, context/attachment format).
5. Response body schema (text field, model identity field, request-id field, usage).
6. Model identifiers available through N-ATLAS.
7. Streaming vs non-streaming; timeout guidance.
8. Rate limits / quota behaviour and the error envelope for each.
9. Whether N-ATLAS accepts repository context, and in what shape.
10. A test credential or sandbox endpoint.
11. Terms governing what evidence may be published.

Credentials must be supplied **externally** via `NATLAS_BASE_URL`, `NATLAS_API_KEY`,
`NATLAS_MODEL` (+ optional `NATLAS_REQUEST_PATH`, `NATLAS_TIMEOUT_MS`). No secret is hardcoded,
committed, or logged; the config `Debug` impl and all evidence redact the key.

## 13. Regressions

**None attributable to this phase.**

One **pre-existing, intermittently-flaky** test was observed and is **documented, not fixed**:

- `router::tests::d1_vector_cache_cross_prompt_contamination_guard`
- Failure signature: `left: 6, right: 5`, message "the D1 guard test must not perform network egress".
- Attributed to a **process-global `AtomicU64`** egress counter incremented at `router.rs:1088`,
  racing across parallel tests. Evidence it is not ours: the competition module has **zero
  references** to the router or that counter; the test passes in isolation (1/0),
  single-threaded (459/0), on parallel re-run (459/0), and with competition tests skipped (441/0).
- Deliberately **not fixed** — the fix belongs in `router.rs`, which this phase must not modify.
- Recorded in `NATLAS_VALIDATION_PROTOCOL_2026-10-06.md` §6.1.

No existing test was weakened. No test was deleted.

## 14. Competition submission gaps remaining

| Gap | Status |
|-----|--------|
| Real N-ATLAS runtime integration | ⛔ blocked — awaiting credentials + contract |
| End-to-end slice against a live N-ATLAS | ⛔ not possible yet |
| External beta testers (real humans) | 🚫 **not fabricated** — template only, 0 rows |
| Demonstration video | 🚫 out of scope for C0 |
| Screenshots / benchmarks | 🚫 none fabricated |
| Competition compliance claim | 🚫 not made |
| Public deployment | 🚫 not done |

## 15. Exact recommended Phase C1

**Phase C1 — Contract Ingestion & First Live Invocation (gated on owner-supplied N-ATLAS access).**

Scope, in order:
1. Owner supplies the N-ATLAS contract details and a test credential (item 12).
2. Implement a single concrete `HttpNatlasTransport` (one new file) that maps the *documented*
   contract to `NatlasRawResponse`. No other module changes.
3. Add contract-conformance tests using recorded, non-secret fixture bytes supplied by N-ATLAS.
4. Perform **one** live invocation against the sandbox; capture full `NatlasEvidence` with
   request id, latency, status and the resulting operation.
5. Promote the end-to-end slice from `SCAFFOLDED` to `RUNTIME_VERIFIED` **only** if the live
   invocation succeeds and the evidence chain is complete.
6. Do **not** begin beta testing until at least one genuine live invocation is evidenced.

Phase C1 must not start until the owner reviews this report.

## 16. `git status`

```
 M .github/workflows/ci.yml
 M .github/workflows/release.yml
 M crates/zylcode-core/src/agent.rs
 M crates/zylcode-core/src/cli.rs
 M crates/zylcode-core/src/lib.rs          <- foreign hunk, deliberately uncommitted
 M crates/zylcode-core/src/router.rs
 M crates/zylcode-core/src/terminal.rs
?? .git-msg.txt
?? COMPLETE_AUDIT_REPORT.html
?? crates/zylcode-core/src/project/filesystem.rs.backup
?? docs/governance/RECONCILIATION_FORENSIC_REPORT.md
?? run_tests.bat
?? tasks/
?? vc_inspect.py
```

Exactly **14 foreign entries** (7 modified + 7 untracked). **All pre-existing and foreign** —
none created, discarded, overwritten, stashed, reset or committed by this phase.
Stashes: **0**.

## 17. Ahead / behind

`git rev-list --left-right --count origin/main...HEAD` → **`0  10`**

- behind `origin/main`: **0**
- ahead of `origin/main`: **10** (6 inherited + 4 created this phase; +1 closing report commit = 11 at final tip)

`origin/main` remains `1338d0b`. There is **no** remote `competition/natlas-2026` branch.

## 18. Confirmation — NOTHING WAS PUSHED

**Confirmed.** `git ls-remote --heads origin competition/natlas-2026` returns **empty**.
No push, no PR, no merge to `main`, no force operation, no history amendment, no tag.
All Phase C0 work exists **only** as local commits on `competition/natlas-2026`.

---

## Stop declaration

Phase C0 is complete. Per directive, this session **stops here**. It does **not** proceed to
Phase C1, does **not** merge to `main`, does **not** push, and does **not** create any
competition submission artefact. Awaiting owner review.
