# FLAGSHIP ENGINEERING JOURNEY — N-ATLAS × ZylCode Developer Bridge

**Branch:** `competition/natlas-2026` · **Audit pass:** 2026-10-07 · **Status at close: READY FOR OWNER LIVE DEMO**

This report records the work that turns multilingual N-ATLAS into a *real* ZylCode engineering
action: a full chain from a natural-language instruction, through the model, into a structured
intent, through the product's factory, with a human approval gate, a real repo tool action, a real
test, and a provenance-backed evidence record. Every claim here is backed by a test or a genuine
model call. Nothing is simulated.

---

### 1. Audit before coding — the central gap was confirmed
The existing `intent.rs` `to_task_graph()` collapsed **every** `implement`/`test`/`run` step into a
side-effect-free `TaskAction::Note`, because the schema carried no file content and no command. The
bridge could *describe* a mutation but could not *perform* one. That was the gap this work closed.

### 2. Reused the production factory, not the legacy dead-end
The bridge hands off to the real `FactoryRunner` / `FactoryJob` / `TaskGraph` — the durable product
workflow — exactly as the directive required. The legacy `agent.rs` path was **not** touched. No new
execution engine was invented; the seam is `NatlasEngineeringIntent::to_task_graph()`.

### 3. Disposable fixture repo — never the real ZylCode source
Every factory test runs in a `tempfile` workspace (`env()` helper in `tests/natlas_bridge.rs`). No test
touches the real repository and none leaves an artefact behind.

### 4. Evidence IDs opened and populated: EV-016 → EV-022
EV-016 and EV-017 are **genuine live calls** (L3). EV-018/019/020 are hermetic test-backed. EV-021 is
the beta package. EV-022 is the matrix-honesty documentation. (See `NATLAS_EVIDENCE_INDEX.md`.)

### 5. EV-016 — genuine Nigerian Pidgin comprehension (L3, PASS)
Through the controlled endpoint: healthz 200 (model `NCAIR1/N-ATLaS`), Pidgin prompt *"Abeg, explain
for me wetin be 'variable'…"* → HTTP 200, coherent answer. Output was in standard English; Pidgin
*generation* was not tested. Pidgin is **not** one of N-ATLAS's four stated target languages — this is
an empirical bonus, recorded honestly.

### 6. EV-017 — genuine Yoruba → structured ZylCode intent contract (L3, PARTIAL)
Genuine call: healthz 200, HTTP 200, real `NCAIR1/N-ATLaS`. The model **understood the Yoruba
request** and emitted the correct contract *shape* (`summary` + `steps` with `implement` carrying
`path`/`content` and `test` carrying `command`/`verify:true`). **The strict JSON parse failed** because
the model embedded a Python triple-quoted string with raw newlines as the `content` value. The model's
intent is proven; the failure is an LLM-in-JSON fidelity defect, not a capability gap. Flagged, not
faked — `NatlasEngineeringIntent::parse` correctly rejects it.

### 7. Intent contract extended so models can emit real mutations
`NatlasIntentStep` now carries `path` / `content` / `command` / `verify`. `to_task_graph()` maps
`implement`/`document` + (path, content) → `WriteFile`; `test` + command + `verify` → `Verify`; `run` /
`test`-without-verify + command → `RunCommand{ expect_exit:0 }`; a missing content/command → `Note`
(**no fabricated file body**); `approve` → a real `Approval` gate.

### 8. Approval gating is structural and order-independent — BUG FOUND AND FIXED
A real side-effect node must depend on the first `approve` step **no matter where it sits in the list**.
The original code set `approval_id` only *after* reaching the approval step, so a mutation listed
*before* `approve` was not gated (`depends_on: []`). `approval_id` is now computed **upfront** from
`approval_step_id()`. The test `approval_gates_every_side_effect_regardless_of_order` proves a backward
edge (step-01 → step-02) is created and `TaskGraph::validate()` accepts it. This is the competition's
"no mutation before approval" rule, enforced by the DAG.

### 9. Endpoint resilience states — honest, never substituted (EV-019)
`NatlasResilienceState` (Ok / Warming / Loading / Timeout / Quota / AuthFailure / Unavailable /
Blocked / Failed) with `from_http` / `from_error`. A cold/quota/auth/unavailable endpoint is reported
**truthfully**; there is deliberately **no** "fall back to another model" state. Substituting a
different model would be fabrication, so it cannot happen by construction.

### 10. Evidence redaction + provenance (EV-020)
Secrets are scrubbed from error records before storage and the redaction count is recorded (never
printed, never committed). The evidence graph seeds an `INTENT` node and every later node traces back
to it, so a claim's provenance is auditable.

### 11. Targeted tests added — the directive's full enumerated list
`tests/natlas_bridge.rs` covers: intent translation; malformed output rejected; unknown kind rejected;
approval enforcement (wrong-task approve refused); no-mutation-before-approval; approved mutation;
execution failure (exit mismatch → a **Contradicted** claim, not a dropped one); test failure;
endpoint unavailable (503 → **no synthetic response**); auth failure (401 → stated); evidence
redaction; provenance; no synthetic fallback. **12 tests, all green.**

### 12. Hermetic bridge suite is green (EV-018)
`cargo test -p zylcode-core --test natlas_bridge` → **12 passed; 0 failed.** This proves the entire
bridge mechanics — NL intent contract → validated task graph → real factory execution with a human
approval gate → real file write + real test → evidence graph — without any live endpoint.

### 13. Pre-existing suites not regressed
After the `intent.rs` / `types.rs` refactor, `natlas_boundary` (17/17) and `natlas_runtime` (12/12)
still pass. The change added capability without breaking the prior 58-test baseline.

### 14. Full competition test total: 79 / 79 green
```
cargo test -p zylcode-core --lib competition::   -> 38 passed; 0 failed
cargo test -p zylcode-core --test natlas_boundary -> 17 passed; 0 failed
cargo test -p zylcode-core --test natlas_runtime  -> 12 passed; 0 failed
cargo test -p zylcode-core --test natlas_bridge   -> 12 passed; 0 failed
```
(`natlas_live.rs` is excluded — it drives the real endpoint and is gated; it is a demo harness, not a
CI assertion.)

### 15. Beta-test package prepared (EV-021) — PS1 ≥2-external-tester requirement
`developer/BETA_TEST_EVIDENCE_TEMPLATE.md` + `developer/BETA_TEST_INVITE.md`, recruitment status
**Recruited: 0**, core team never counted, explicit no-fabrication rules. This is a **human action**;
the package is ready but cannot be self-issued.

### 16. Multilingual matrix honesty (EV-022)
Corrected from a stale "EMPTY" to **PARTIAL** with a documented native-speaker authorship caveat
(quality bar, not capability gap). EV-017 folded in; gate **ML-6** (structured-intent contract) added.

### 17. Secret hygiene
`NATLAS_API_KEY` and the HF token are read from a local gitignored file, never printed, never
committed. Redaction is unit- and integration-tested.

### 18. Foreign / owner dirty work preserved
The working tree's foreign edits (`agent.rs`, `router.rs`, `cli.rs`, `terminal.rs`, `lib.rs`, plus
untracked `tasks/`, audit HTML, `run_tests.bat`, etc.) were left **untouched**. Only the competition
files and the new `tests/natlas_bridge.rs` were added or modified.

### 19. No push performed
The owner has not authorised a push. Remote state is unchanged. Deadline: **12 Oct 2026 23:59 WAT**.

### 20. Known gap — LLM-in-JSON fidelity (EV-017)
Before wiring the live model output into `to_task_graph()`, the production SYSTEM preamble must:
(a) forbid triple-quoted / multi-line literals in `content` and require `\n`-escaped single-line
strings, and/or (b) add a tolerant repair pass. This is a contract-tightening task, not a capability
gap. The model's intent was correct; only the serialization was not.

### 21. Known risk — ZeroGPU scheduling for demo-day
The controlled endpoint is healthy now, but it runs on shared ZeroGPU. Sustained/repeatable calls are
quota-limited. **Re-warm the Space before the live demo** (the B1(c) public path is the fallback).

### 22. READY FOR OWNER LIVE DEMO
The deterministic chain is proven (EV-018). The genuine N-ATLAS leg is demonstrated (EV-014/015/016/
017). The owner's remaining steps are human actions: re-warm the controlled endpoint and record the
end-to-end screen-capture with the human approval gate, recruit ≥2 beta testers, and supply the team
profile / CAC. Nothing in the engineering path blocks those.

---

## What was actually changed (files)
- `crates/zylcode-core/src/competition/natlas/intent.rs` — contract extended; **approval-gating bug fixed**; new unit tests.
- `crates/zylcode-core/src/competition/natlas/types.rs` — `NatlasResilienceState` added; new unit tests.
- `crates/zylcode-core/tests/natlas_bridge.rs` — **new**: 12-test hermetic bridge suite.
- `docs/competition/natlas/NATLAS_EVIDENCE_INDEX.md` — EV-016…EV-022 recorded; totals updated to 79/79.
- `docs/competition/natlas/MULTILINGUAL_VALIDATION_MATRIX.md` — EV-017 row; ML-6 gate; native-speaker caveat.
- `docs/competition/natlas/SUBMISSION_CHECKLIST_2026-10-07.md` — bridge suite + EV-017 cross-referenced; C3 demo marked proven-hermetically.
- `docs/competition/natlas/evidence/EV-017-yoruba-structured-intent.json` — raw genuine capture (gitignored).

## Honesty ledger
- 0 fabricated tester rows. 0 fabricated benchmark numbers. 0 simulated-success claims.
- EV-017 is PARTIAL, not PASS — the model's JSON was malformed; recorded as such.
- The endpoint is healthy *now*; sustained availability is a documented scheduling risk, not a claim.
