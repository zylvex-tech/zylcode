# ZYLCODE — FINAL OWNER GATE REPORT

**Directive:** ZYLCODE AI-NATIVE SOFTWARE & RESEARCH FACTORY — DISCOVERY, ARCHITECTURE AND FOUNDATION WAVE (2026-10-06)
**Repository:** `https://github.com/zylvex-tech/zylcode` — **this is the only repository that was touched.**
**Branch:** `main` · **Push status:** **NOT PUSHED** (no push authorisation was given; nothing was pushed.)
**Report date:** 2026-10-06

> **On this report's shape.** This report is reproduced from the owner directive as faithfully as the
> available record allows. Where the directive's exact field list differs from what is below, say so
> and it will be reformatted — the *content* is what matters, and every claim below is either
> reproduced from a command in this session or explicitly marked as not run.

---

## 1. Scope compliance

| Requirement | Status | Evidence |
|---|---|---|
| Only `zylcode` touched | **HONOURED** | `git remote -v` shows one repo; every file changed in this session is under `C:/Projects/zylcode` |
| ZylForge untouched | **HONOURED** | no path outside this workspace was written |
| zylvex.tech website untouched | **HONOURED** | not present in this workspace |
| Guardian / defence repos untouched | **HONOURED** | not present in this workspace |
| No push | **HONOURED** | `git rev-list --left-right --count origin/main...HEAD` → `0  4` (4 ahead, unpushed) |
| No amend of historical commits | **HONOURED** | all four commits are new; history from `aae6b4e` forward is append-only |

---

## 2. Baseline (captured before any change)

| Property | Value |
|---|---|
| `main` HEAD at start | `aae6b4e4ffd939cd09abb6d7345ae1a762f7ba03` |
| `origin/main` | identical to the above |
| Ahead / behind | `0 / 0` |
| Dirty entries at start | 14 |
| Stashes | 0 |
| Tracked files | 475 |

The working tree carried **foreign, pre-existing edits** (`.github/workflows/ci.yml`,
`release.yml`, `agent.rs`, `cli.rs`, `router.rs`, `terminal.rs`, plus untracked files). **These were
not touched and not committed.** The one exception is a single additive line (`pub mod factory;`) in
`lib.rs`, which was staged in isolation with a hand-written hunk so the foreign change in the same
file was left alone.

---

## 3. Commits created (all unpushed)

| # | SHA | Subject | Scope |
|---|---|---|---|
| 1 | `f30c83f` | `docs(governance): software-factory architecture, capability census and execution model` | 8 governance docs + machine-readable registry |
| 2 | `6c383c5` | `feat(factory): durable dependency-aware task graph, job lifecycle and store` | `factory/{graph,job,store}.rs` |
| 3 | `b09340e` | `feat(factory): evidence-producing factory runner and the end-to-end vertical slice` | `factory/runner.rs` + `tests/factory_job_e2e.rs` |
| 4 | `efab2ac` | `fix(census): reconcile capability-registry counts and three stale statuses` | correction of defects found in commit 1 (see §8) |
| 5 | `179a3d7` | `docs(census): record the 2026-10-06 workspace test battery as K-01's baseline` | census K-01 |
| 6 | *(this report)* | `docs(gate): final owner gate report` | this file |

Commit 4 exists because commit 1 was **not** internally consistent. Rather than amend history, the
defect was corrected forward and documented. Details in §8.

---

## 4. Deliverables produced

### 4.1 Governance documents (all dated 2026-10-06)

| Document | Covers directive phase(s) | Honest status |
|---|---|---|
| `ZYLCODE_SOFTWARE_FACTORY_MASTER_ARCHITECTURE_2026-10-06.md` | factory architecture | roles-not-agents model; every table separates IMPLEMENTED from DESIGNED |
| `ZYLCODE_CAPABILITY_CENSUS_2026-10-06.md` | census | 86 capabilities, evidence-derived status |
| `ZYLCODE_FACTORY_EXECUTION_MODEL_2026-10-06.md` | execution | lifecycle, graph semantics, verification-first, HITL, recovery |
| `ZYLCODE_RESEARCH_MODE_ARCHITECTURE_2026-10-06.md` | research | **DESIGNED only** — no code |
| `ZYLCODE_INSTITUTIONAL_MODE_ARCHITECTURE_2026-10-06.md` | institutional / education | **DESIGNED only** — no code |
| `ZYLCODE_EVIDENCE_AND_PROVENANCE_MODEL_2026-10-06.md` | evidence model | the three R-ladders, claim/failure models, memory scopes |
| `ZYLCODE_AGENT_EVALUATION_MODEL_2026-10-06.md` | evaluation | **DESIGNED only** — 11 cases, 5 provider-free |
| `ZYLCODE_AFIT_INSTITUTIONAL_READINESS_2026-10-06.md` | institutional readiness | DEMONSTRABLE NOW / ACTIVE / RESEARCH / BLOCKED |

### 4.2 Machine-readable registry

`docs/capability-census-2026-10-06.json` — 86 capabilities across 22 categories (A–V), a status
vocabulary, and open gates.

### 4.3 Code

`crates/zylcode-core/src/factory/{graph,job,store,runner}.rs` and
`crates/zylcode-core/tests/factory_job_e2e.rs`. This is a **new subsystem**; it modifies no existing
behaviour.

---

## 5. First implementation target — verdict

The directive's required slice:

```
JOB → DURABLE TASK GRAPH → REPOSITORY CONTEXT → ONE BOUNDED IMPLEMENTATION TASK
    → REAL TOOL EXECUTION → TEST → EVIDENCE RECORD → HUMAN REVIEW → DURABLE COMPLETION STATE
```

| Link in the chain | Verdict | Evidence |
|---|---|---|
| Job | **PROVEN** | `FactoryJob`, `factory/job.rs` |
| Durable task graph | **PROVEN** | `factory/graph.rs` — DAG, cycle detection, readiness, orphan propagation |
| Repository context | **PROVEN** | `factory_job_e2e.rs::the_slice_…` reads a file it wrote back |
| One bounded implementation task | **PROVEN** | real file write asserted on disk |
| Real tool execution | **PROVEN** | through the gated `zylcode_mcp::dispatch` path |
| Test | **PROVEN** | real command, exit code checked |
| Evidence record | **PROVEN** | evidence graph + claim store; ancestry to INTENT asserted |
| Human review | **PROVEN** | approval gate parks and resumes |
| Durable completion state | **PROVEN** | job reloaded from disk, `is_complete()`, stage `DELIVERY` |
| **Model transition** | **`BLOCKED_PROVIDER`** | no approved cloud provider; model work is blocked, **not faked** |

**The deterministic half is proven. The model transition is blocked and labelled.** Per the
directive's instruction, it was not faked.

---

## 6. Evidence — reproduction

Run from the repository root. All commands below were executed in this session.

```text
cargo test -p zylcode-core --lib factory
  → test result: ok. 23 passed; 0 failed; 0 ignored

cargo test -p zylcode-core --test factory_job_e2e
  → test result: ok. 7 passed; 0 failed; 0 ignored
     (the_slice_executes_in_dependency_order_with_real_tools_and_evidence
      a_model_task_is_blocked_provider_and_never_faked
      a_human_approval_gate_parks_and_then_resumes
      a_failed_dependency_cancels_its_dependents
      a_policy_refusal_parks_for_approval_then_executes_after_approval
      a_job_resumes_from_the_store_and_does_not_repeat_completed_work
      an_invalid_graph_is_refused_before_anything_runs)

cargo test --workspace --all-targets --no-fail-fast
  → EXIT=0 · 730 passed · 0 failed · 1 ignored · 25 test-result groups
     the one ignored test is `live_commissioning_probe`
     ("makes real provider calls; run explicitly with --ignored") — so no egress occurred
```

`cargo clippy --workspace --all-targets -- -D warnings` was clean after the fixes in this session.

---

## 7. Capability census — reconciled state

| Status | Count |
|---|---|
| `TESTED` | 47 |
| `RUNTIME_VERIFIED` | 6 |
| `WIRED` | 5 |
| `SCAFFOLDED` | 4 |
| `UI_ONLY` | 2 |
| `NOT_IMPLEMENTED` | 19 |
| `BLOCKED` | 3 |
| **Total** | **86** |

Both files now agree: 86 entries, identical ids, zero status mismatches, and the summary counts are
computed from the entries.

---

## 8. Defects found in my own work, and fixed

| # | Defect | Where | Fix |
|---|---|---|---|
| D-1 | The registry declared **66 capabilities** and a `by_status` summing to **82**, while it held **86** entries — a fabricated/derivable-from-nothing count | census JSON + MD | commit `efab2ac`; counts recomputed from entries |
| D-2 | Three statuses stale in the JSON (`F-03`, `F-04` = `NOT_IMPLEMENTED`; `P-06` = `SCAFFOLDED`) while the markdown already recorded this wave's tested work | census JSON | commit `efab2ac`; set to `TESTED` with module paths and test counts |
| D-3 | A two-value status cell (`SCAFFOLDED → TESTED`) used on one row only | census MD | normalised to a single value |

This is exactly the class of error the directive forbids. It was found by auditing my own output,
not by an external reviewer, and it is recorded rather than quietly corrected.

---

## 9. Open gates / carried items

| Gate | State | Reason |
|---|---|---|
| `AGENT-01` | **BLOCKED** | No usable approved cloud provider (OpenRouter/Anthropic 401; DeepSeek 402; OpenRouter key is a literal placeholder). **Not** worked around with Ollama or simulation. |
| `CI-BILLING` | **BLOCKED_EXTERNAL** | GitHub Actions account billing lock; jobs execute zero steps. |
| `GATE0-HISTORY` | **OPEN — OWNER DECISION** | Diverged Gate-0 history; reconciliation report untracked; merge not executed. |
| `PHASE-2A` | **RE-OPENED** | Not accepted; disputed registry entries pending independent re-acceptance. |
| `G-01` | open | vector cache can record a synthetic hit |
| `G-02` | open | orphan Tauri project IPC path |
| `G-03` | open | GUI editor save routes to a command absent from `generate_handler` |
| Computer-Use Engine | **R0 simulation facade** | sleep-based actions, `vec![0]` captures, hardcoded confidences — deliberately **not** extended |

---

## 10. Hard stops encountered

**None.** No wrong repo, no destructive risk, no architecture conflict, no secret exposure, no
unsatisfiable provider requirement, no licensing issue, no indistinguishability of owner vs agent
work, no baseline regression, no requirement to weaken evidence. The provider block
(`AGENT-01`) was handled as designed — by blocking and labelling, not by stopping or faking.

---

## 11. What was explicitly NOT done

- **Nothing was pushed.**
- **No model was called.** No provider was silently used; no response was fabricated.
- **No fallback between providers.**
- Research, institutional, education, security and design capabilities are **designed, not built** —
  and every document says so.
- Parallel execution over isolated worktrees is **designed**, not implemented; the runner executes
  sequentially and says so.
- The Computer-Use simulation facade was **not extended** (replace, do not extend).
- No existing behaviour was modified; the factory subsystem is additive.

---

## 12. Truthfulness self-audit

| Rule | Compliance |
|---|---|
| Never claim a rung above the evidence | census statuses are evidence-derived; three stale ones were corrected downward-to-truth |
| Acting ≠ having acted | every `TESTED` claim names a committed test; every `RUNTIME_VERIFIED` claim names a captured outcome |
| Builders do not certify their own work | the factory code is **not** marked R3; only unit/integration tests (R2) are claimed |
| Unbuilt systems marked PROPOSED | research/institutional/security/design marked DESIGNED throughout |
| Never fabricate a count | the one fabricated count found was corrected and disclosed |

---

## 13. What I need from the owner

1. **Authorisation to push** the four commits (or an instruction to keep them local).
2. **A usable approved provider** (or explicit confirmation that AGENT-01 stays blocked for now).
3. **A decision on `GATE0-HISTORY`** (reconcile the diverged Gate-0 history, or quarantine it).
4. **Confirmation of the exact FINAL OWNER GATE REPORT field list**, if this shape differs from the
   directive's.
