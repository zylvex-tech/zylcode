# ZYLCODE TRANSFORMATION PLAN — Evidence-First Autonomous Engineering

**Date:** 2026-09-26
**Derived from:** Master Transformation Prompt (§1–§38).
**Input:** `ENGINEERING_TRUTH.md` (same session) — every phase below maps to a measured gap,
not a wish.
**Priority order (§34, never reversed):** correctness → evidence integrity → security →
reliability → reproducibility → core autonomous capability → UX → performance → extensibility
→ ecosystem → monetization.

---

## Non-negotiable constraints (§2)

Rust stays. MCP stays. Plugins, model abstraction, persistent state, desktop, CLI, and the
ecosystem vision stay. Each phase **consolidates** existing machinery
(`ledger`, `proof_engine`, `best_of_n`, `missions`, `mcp/evidence`) rather than replacing it.

## The ladder decision (§7 + governance)

Two ladders are live and must never be conflated:

| Ladder                                                     | Meaning                          | Owner                                |
| ---------------------------------------------------------- | -------------------------------- | ------------------------------------ |
| **Capability rung R0–R5** (claimed → commissioned)         | maturity of a _capability_       | `ZYLCODE_PROOF_GRAPH.md` (GOVERNING) |
| **Claim verification level R0–R7** (model output → formal) | evidence strength of _one claim_ | Master Prompt §7                     |

Rule: every artifact names its ladder explicitly. `R7 FORMALLY VERIFIED` may not be emitted
before mechanical checking exists (§8). Until then the product language is
**Evidence-First Software Synthesis**.

---

## Phase T1 — Claim model + Evidence Graph (foundation; §4, §5)

The smallest coherent change everything else depends on.

1. `core/claim.rs` — `Claim{statement,kind,status,source,evidence_refs,confidence,verification_level,created_at,verified_at}`
   with the seven epistemic states, **fail-closed rules**:
   - `status = VERIFIED` requires ≥ 1 evidence ref _and_ level ≥ R2; the API makes the
     promotion refuse otherwise (§3: no evidence → cannot be PROVEN).
   - `CONTRADICTED` is reachable only through recorded contradicting evidence.
   - hash-chained persistence (`.zylcode/claims.json`, same conventions as `ProofEngine`).
2. `core/evidence_graph.rs` — typed nodes (`INTENT SPECIFICATION PLAN DECISION ACTION
TOOL_CALL RESULT ARTIFACT VERIFICATION CLAIM`) + typed edges, each node carrying the §5
   provenance fields (initiator, model, tool, inputs, outputs, files, environment, commit,
   timestamps, hashes). Nodes reference _existing_ evidence objects (LedgerEntry ids,
   ProofRecord ids, tool JSONL ids) — no duplication.
3. Persistence survives restart/crash (file-backed, hash-chained like `ProofEngine`).
4. Tests: fail-closed promotion, contradiction path, chain-tamper detection, serde round-trip,
   cross-reference integrity (dangling evidence_refs rejected).

**Exit criterion:** a claim produced by a real command run can be loaded after process
restart and its evidence chain verified bit-for-bit.

## Phase T2 — Failure as a first-class object (§9)

Unified `Failure{operation, cause, evidence, affected_artifacts, recovery_strategy,
retry_count, resolution}` and the taxonomy `FAILED BLOCKED TIMED_OUT CANCELLED UNKNOWN
PARTIALLY_COMPLETED RECOVERED`, mapped from (not replacing) `ExecutionState` and
`MissionState`. No path may turn a failure into success without evidence (§9 last line).

**Exit criterion:** a timed-out tool run yields a `TIMED_OUT` failure object citing its
ledger entry; `mark_verified` on that mission is refused.

## Phase T3 — THE FIRST MISSION (§36) — prove the philosophy end-to-end

One complete autonomous engineering mission against a **real fixture project** (not this
repo's own source, to keep risk bounded), executing all 15 mandated steps with no simulated
steps:

spec → structured plan → real tool calls → real file modification → real tests →
**injected recoverable failure** (e.g. a deliberately broken assertion) → diagnose → repair →
re-run verification → persist execution ledger → emit evidence graph → emit final
engineering record → classify each outcome VERIFIED vs HYPOTHESIS/UNKNOWN →
**kill the process mid-mission and resume from checkpoint** → export reproducibility package
(spec, plan, commands, tool versions, environment, hashes, evidence graph, limitations).

Deliverable: `zylcode mission run --spec …` (or equivalent named entry point — R3 requires
a reachable surface) + committed transcript as evidence.

**Exit criterion:** an independent re-run of the exported package reproduces the verification
results; interruption leaves a resumable, auditable ledger — never a silent restart.

## Phase T4 — Truth surface + Engineering Record (§11, §12)

Group claims in the UI as PROVEN / DERIVED / UNKNOWN / HYPOTHESIS / BLOCKED (data comes
from T1 claims; grouping is presentation only). Engineering Record export = reproducibility
package UI wrapper. Claims without evidence refs render as their true status — the surface
cannot be talked into "green" (§18).

## Phase T5 — Adversarial hardening + self-audit (§21, §24)

Corrupted evidence files, partial writes, mid-write kills, tampered chains, duplicate
operations, permission escalation attempts. Then turn the system on itself: run the §21
claim demos (crash recovery, offline, unsupported-completion refusal) as scripted,
reproducible proof-lab entries (§26).

---

## Working discipline (§33, per phase)

INSPECT → MAP → DEFINE → IMPLEMENT (smallest coherent change) → targeted tests → integration
tests → adversarial test → record evidence → update docs → commit → report with
`Implemented / Tested / Verified / Known limitations / Remaining work / Evidence`.

Forbidden: "implemented" without code, "works" without a run, "verified" without evidence,
mock-for-completion, hidden failures, test-shaped green (§32).

## What explicitly does NOT start yet

- Commercial/marketplace work (priority last).
- Computer-use replacement (governance: replace, do not extend — separate work order).
- Formal-verification machinery (R7) beyond keeping the architecture open for it (§8).
- Any rung promotion in the capability registry — acceptance is the auditor's verdict, not
  the builder's (Proof Graph §8).

## First increment (this session)

Phase T1 step 1: `core/claim.rs` with fail-closed promotion rules + hash chain + tests,
wired into `zylcode-core`, battery green. Started 2026-09-26.
