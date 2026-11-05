# ENGINEERING TRUTH — actual state of ZylCode

**Date:** 2026-09-26
**Authority:** Master Transformation Prompt §22 (repository truth model) / §23 (inspect, do not conceal).
**Method:** every claim below was produced by a command executed this session on the primary
working tree (`main`, clean tracked state). Commands and outputs are quoted. Nothing here is
hand-asserted; a claim without a command is marked `UNVERIFIED` by definition.

Status vocabulary (§22): `IMPLEMENTED` · `TESTED` · `VERIFIED` · `PARTIAL` · `EXPERIMENTAL` ·
`PLANNED` · `SPECULATIVE` · `DEPRECATED`.

---

## 1. Verification battery (this session, primary tree)

| Check        | Command                                                 | Result                                |
| ------------ | ------------------------------------------------------- | ------------------------------------- |
| Compile      | `cargo check --workspace --all-targets`                 | **PASS** (1m46s, 0 errors)            |
| Lint         | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (0 warnings)                 |
| Tests        | `cargo test --workspace --all-targets --no-fail-fast`   | **547 passed / 0 failed** (14 suites) |
| Claims guard | `python scripts/check_retracted_claims.py`              | **OK** — 11 baselined, 0 new          |

These are _builder results_ (Proof Graph §8: they do not by themselves upgrade any rung).

## 2. What exists and what it actually is

Measured by source inspection of the public API surface (`pub struct/enum/fn`) this session.

### 2.1 Evidence primitives — `PARTIAL`, strongest area

| Capability                                                   | Where                                                      | Measured state                                                                                                                                                                                                                                            | Honest status   |
| ------------------------------------------------------------ | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| Execution ledger (hash-chained, session-scoped, checkpoints) | `core/ledger.rs` + `memory_ledger.rs` + `sqlite_ledger.rs` | `LedgerEntry{prev_hash,…}`, `SessionCheckpoint`, `LedgerStore` trait; 2 implementations; integration test `tests/crash_recovery.rs`                                                                                                                       | **TESTED**      |
| Crash recovery / reconciliation                              | `core/agent.rs`                                            | `recover_from_checkpoint()`, `RecoveryResult`, `ReconciliationResult/Status` exist and are exercised by tests                                                                                                                                             | **TESTED**      |
| Proof records (deterministic verification)                   | `core/proof_engine.rs`                                     | `ProofRecord` hash-chained with `command/exit_status/environment/source_commit`, honest `VerificationState` (incl. `Blocked`, `RuntimeNotReached`, `EvidenceMissing`), `ProofSource` distinguishes runtime vs simulated; persisted `.zylcode/proofs.json` | **TESTED**      |
| Builder/verifier separation                                  | `core/best_of_n.rs`, `core/patch_best_of_n.rs`             | `TestSuiteVerifier` + `SelfVerificationGate` — candidates are accepted only through a deterministic gate, not model say-so                                                                                                                                | **TESTED**      |
| Tool execution evidence                                      | `mcp/evidence.rs`, `mcp/permission.rs`, `mcp/actor.rs`     | `JsonlEvidenceSink`, `ToolRuntime` with permission gate; every gated tool call writes evidence                                                                                                                                                            | **TESTED**      |
| Mission queue                                                | `core/missions.rs`                                         | `Mission{mode,state,ledger_session}`, plan builder, `finish/approve/block/mark_verified/attach_artifacts`                                                                                                                                                 | **TESTED**      |
| Build/release pipeline                                       | `core/delivery.rs`                                         | `build_workspace()` steps, `package_release()`                                                                                                                                                                                                            | **TESTED**      |
| Crash-recovery product surface                               | `.zylcode/ledger.db`, `missions.json`                      | Live runtime state present from prior sessions                                                                                                                                                                                                            | **IMPLEMENTED** |

### 2.2 Governance truth machinery — `PARTIAL`

| Item                                     | Measured state                                                                        | Status                                  |
| ---------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------- |
| Proof Graph R0–R5 ladder                 | `docs/governance/ZYLCODE_PROOF_GRAPH.md` GOVERNING; rung assignments table maintained | **IMPLEMENTED** (governance)            |
| Claims guard                             | baselines 11 known retracted-claim occurrences; 0 new today                           | **VERIFIED** (as a guard, this session) |
| Capability registry with dispute history | `docs/capability-registry.json`                                                       | **IMPLEMENTED**                         |
| Tool catalogue metrics pinned            | `mcp/tool_catalogue.rs` — `r3_verified_count == 0` asserted by test                   | **TESTED**                              |

### 2.3 Missing or thinner than the thesis requires

These are the gaps the transformation must close. None is hidden.

| Prompt requirement                                                                                                                            | Current state                                                                                                                                                                                                                                         | Status                               |
| --------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| §4 Formal **Claim** model with epistemic states (`OBSERVED/DERIVED/VERIFIED/HYPOTHESIS/UNVERIFIED/CONTRADICTED/UNKNOWN`)                      | No runtime `Claim` type exists. Three partial analogues: `VerificationState` (command outcomes), `AcceptanceState` (human), capability-registry rows (governance, static JSON). None unifies statement + status + evidence_refs + confidence + level. | **PLANNED** (this session starts it) |
| §5 First-class **Evidence Graph** (typed nodes `INTENT→…→CLAIM`, provenance, hashes, survives restart)                                        | Evidence is _stored_ in three unlinked places: ledger entries (sqlite/memory), `proofs.json` chain, tool JSONL sink. No node/edge graph, no cross-links, no single traversal from intent → claim.                                                     | **PLANNED**                          |
| §9 Structured **Failure** object (`FAILED/BLOCKED/TIMED_OUT/CANCELLED/UNKNOWN/PARTIALLY_COMPLETED/RECOVERED` + recovery_strategy/retry_count) | `ExecutionState` has `Failed/Cancelled`; `MissionState` has `failed/blocked`. No unified taxonomy, no `Failure{operation,cause,evidence,recovery…}` record.                                                                                           | **PLANNED**                          |
| §11 **Reproducibility package** / Engineering Record export per task                                                                          | `package_release()` packages software releases, not engineering records. No spec+plan+commands+hashes+evidence export for a mission.                                                                                                                  | **PLANNED**                          |
| §12 **Truth surface** (PROVEN/DERIVED/UNKNOWN/HYPOTHESIS/BLOCKED grouping in UI)                                                              | `surfaces.rs::evidence_payload()` feeds the Evidence Center, but has no claim grouping because claims don't exist yet.                                                                                                                                | **PLANNED** (blocked on §4)          |
| §36 **First Mission** end-to-end (15 steps, real failure, resume, record, package)                                                            | Mission engine can run build missions with verification, but no mission has produced graph+record+package with an interruption/resume cycle as one auditable unit.                                                                                    | **PLANNED**                          |
| §24 Adversarial evidence tests (corrupted evidence, partial writes, tamper)                                                                   | Chain-integrity tests exist for proofs; not a systematic adversarial suite (corrupted JSONL, truncated sqlite, tampered claims).                                                                                                                      | **PARTIAL**                          |

### 2.4 Known defects and external blockers (carried forward, still true)

1. **GitHub Actions billing lock** — `BLOCKED_EXTERNAL`. Runs trigger; jobs fail with zero
   executed steps ("account is locked due to a billing issue"). Local battery is the only
   verification authority until an org owner clears it. (SOURCE_OF_TRUTH §P0.5.3)
2. **Diverged Gate-0 history** — `RECONCILIATION_FORENSIC_REPORT.md` (untracked, dated
   2026-09-21) records `865c142` vs main divergence; merge recommended, **not executed**;
   owner decision pending.
3. **Benchmark non-determinism** — repository-intelligence precision threshold held at 0.20
   with documented rationale; must not be lowered further, must not be claimed deterministic.
4. **Untracked handoff files on the primary tree** — `tasks/`, `RECONCILIATION_FORENSIC_REPORT.md`,
   `COMPLETE_AUDIT_REPORT.html`, `crates/zylcode-core/src/project/filesystem.rs.backup`,
   `.git-msg.txt`. Ownership unresolved; not deleted, not staged.
5. **Ladder-name collision (new, this session)** — the Master Prompt §7 defines an
   R0–R7 _claim verification ladder_ (model output → formal) whose semantics differ from the
   governing R0–R5 _capability proof rung_ (claimed → commissioned). Both are live. Every
   artifact must name which ladder it means. Reconciliation is a planned governance task;
   until then, ambiguity itself is a defect.

## 3. What the word "verified" will mean from here

Per §3 of the Master Prompt: deterministic components establish facts; the model may not.
This repository already encodes that in `ProofRecord` + `SelfVerificationGate`. The
transformation extends it upward: **claims may only be `VERIFIED` when they cite evidence
objects**, and the evidence graph makes those citations traversable. Until a claim cites
evidence produced by this session's commands, its status is not `VERIFIED` — including the
claims in this document.

## 4. Session evidence index (this document's own citations)

- `cargo check --workspace --all-targets` → PASS
- `cargo clippy --workspace --all-targets -- -D warnings` → PASS
- `cargo test --workspace --all-targets --no-fail-fast` → 547/0
- `python scripts/check_retracted_claims.py` → OK (11 baselined, 0 new)
- API surface greps over `ledger.rs`, `proof_engine.rs`, `best_of_n.rs`, `delivery.rs`,
  `surfaces.rs`, `agent.rs`, `missions.rs`, `mcp/evidence.rs`
