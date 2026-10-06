# ZYLCODE FACTORY EXECUTION MODEL

**Date:** 2026-10-06
**Status:** GOVERNING FOR THIS WAVE. Subordinate to the ratified governance corpus.
**Covers directive phases:** 3 (orchestrator lifecycle), 4 (task graph), 9 (human-in-the-loop),
12 (verification-first development), 20 (failure recovery).
**Implemented substrate:** `crates/zylcode-core/src/factory/{graph,job,store,runner}.rs`.

---

## 1. The execution loop, end to end

```text
FactoryRunner::open(cfg)
  ├─ validate the graph (acyclic, dependencies exist)      ← refuses an unsound graph
  ├─ open per-job evidence graph + claim store
  ├─ resume the evidence spine from disk
  └─ seed the INTENT node (once)

FactoryRunner::run()
  loop
    ├─ if graph.is_complete()        → COMPLETED
    ├─ cancel every orphaned node    → dependency-failure propagation
    ├─ pick the first ready node     → dependency order, stable
    ├─ execute it through a real tool (or block, or park for a human)
    ├─ record evidence + claim + failure
    ├─ recompute stage (forward only)
    └─ persist the job atomically    → a crash here loses at most one task
```

**Nothing in this loop is simulated.** A task either dispatches through
`zylcode_mcp::dispatch` (gated, evidenced), parks at a human gate, or is explicitly blocked with a
reason.

---

## 2. The job lifecycle (Phase 3)

### 2.1 States

`JobStage`: `INTAKE · DISCOVERY · REQUIREMENTS · RESEARCH · PLAN · ARCHITECTURE · TASK_GRAPH ·
IMPLEMENTATION · TEST · REVIEW · SECURITY · DOCUMENTATION · RELEASE_READINESS · OWNER_APPROVAL ·
DELIVERY`

### 2.2 The projection rule

The stage is **derived**, never asserted:

```text
stage = DELIVERY                                    if the graph is complete
      = furthest stage of any SUCCEEDED task kind   otherwise
      = TASK_GRAPH                                  if work has begun but nothing succeeded
      = INTAKE                                      if nothing has been attempted
```

and it is applied through `recompute_stage()`, which is **forward-only**.

**Why derived rather than asserted.** An independently-mutated stage field can drift from reality —
a job could display `TEST` while no test ran. Deriving it from the graph makes that class of lie
structurally impossible. The cost is that the stage can lag (a stage is only reached when a task
proves it), which is the correct direction to err.

### 2.3 Transition record

Every transition writes, where applicable: node id, task kind/role, prior state, new state,
timestamp, attempt count, evidence-node id, claim ids, failure id, and the persisted job document.
A reader can reconstruct the entire run from `.zylcode/factory/jobs/<id>.json` plus the per-job
evidence graph.

---

## 3. The task graph (Phase 4)

### 3.1 Node semantics

| Field | Purpose |
|---|---|
| `id` | unique within the job |
| `kind` | the logical role (see master architecture §3) |
| `action` | **what actually runs** — tool, model, human, or annotation |
| `depends_on` | ids that must reach a *success* state first |
| `state` | one of nine states (below) |
| `attempts` / `max_attempts` | bounded retry accounting |
| `blocked_reason` | present **only** in `Blocked` |
| `approved_by` | set when a human cleared an approval gate |
| `evidence_node_id` / `claim_ids` / `failure_id` | links into the evidence stores |
| `outcome` | short honest summary |

### 3.2 The nine states and why they are not collapsed

| State | Means | Must never be reported as |
|---|---|---|
| `Pending` | dependencies unmet | "in progress" |
| `Ready` | dependencies met, not started | "running" |
| `Running` | executing now | "done" |
| `Succeeded` | ran and met its contract | — |
| `Failed` | ran and did not meet its contract | "blocked" |
| `Blocked` | **cannot run**; reason recorded | "failed" or "pending" |
| `AwaitingApproval` | parked at a human gate | "blocked" |
| `Cancelled` | explicitly cancelled (incl. by propagation) | "skipped" |
| `Skipped` | deliberately not executed | "cancelled" |

### 3.3 Graph invariants enforced before execution

1. **Unique ids.** A duplicate id makes the graph ambiguous.
2. **Every dependency resolves.** A dangling edge is refused at validation, and again at persistence
   (`JobStore::save` refuses to write an invalid graph, so a resume can never read nonsense).
3. **Acyclic.** Verified with Kahn's algorithm; a cycle is an error, not a partial run.
4. **No self-dependency.**

### 3.4 Failure propagation

`TaskGraph::orphaned()` returns pending nodes whose dependency reached `Failed`, `Blocked` or
`Cancelled`. The runner **cancels them explicitly** with a reason. They are never left `Pending`
forever, and never silently promoted. This is what makes "resume after restart" safe: the graph's
terminal state is unambiguous.

---

## 4. Verification-first development (Phase 12)

### 4.1 The required arc

```text
PLAN → PATCH → FORMAT → TYPECHECK → UNIT TEST → INTEGRATION TEST → BUILD
     → SECURITY CHECK → REVIEW → EVIDENCE
```

### 4.2 What the implemented slice actually enforces

| Stage | Enforced by | Status |
|---|---|---|
| PLAN | `TaskGraph` + job document | **IMPLEMENTED** |
| PATCH | `fs.write` through the gated dispatch | **IMPLEMENTED** |
| FORMAT | — | **DESIGNED** |
| TYPECHECK | `shell.execute` (e.g. `cargo check`) as a `Verify` task | **IMPLEMENTED as a pattern** |
| UNIT / INTEGRATION TEST | `shell.execute` with an **exit-code expectation** | **IMPLEMENTED** |
| BUILD | `delivery::build_workspace` | **IMPLEMENTED** (outside the runner) |
| SECURITY CHECK | — | **DESIGNED** (no scanner exists) |
| REVIEW | `TaskAction::Approval` | **IMPLEMENTED** |
| EVIDENCE | evidence graph + claim store | **IMPLEMENTED** |

### 4.3 The two rules that make "verified" mean something

> **A green-looking UI is not evidence. An agent saying "fixed" is not evidence.**

The runner encodes both:

1. **Exit code decides.** A `Verify` or `RunCommand` task compares the observed exit code against an
   explicit expectation. A match produces a claim at `R3TestVerified` promoted to `VERIFIED`. A
   mismatch produces a **`CONTRADICTED`** claim plus a `Failure` record — the failure is *recorded*,
   not dropped.
2. **Promotion is fail-closed.** `Claim::promote_verified()` refuses without at least one evidence
   reference and a level of `R2` or higher. The runner calls it; it cannot force it. If promotion is
   refused the claim stays `OBSERVED`, which is the honest outcome.

---

## 5. Human-in-the-loop governance (Phase 9)

### 5.1 The autonomy gradient

| Operation class | Intended treatment | Implemented? |
|---|---|---|
| routine safe read | agent may proceed | **yes** — `RiskLevel::Read` within the default threshold |
| routine safe code edit | agent may proceed within the job's risk ceiling | **yes** — `RiskLevel::Write` if the caller sets that ceiling |
| new dependency | policy check | **DESIGNED** |
| schema migration | approval by risk | **DESIGNED** |
| secret access | explicit permission | **DESIGNED** |
| production deployment | human approval | **DESIGNED** |
| destructive operation | human approval | **yes** — `RiskLevel::Destructive` exceeds any default ceiling → `RequireApproval` |
| security-critical architecture | review gate | **DESIGNED** |
| release | evidence gate | **DESIGNED** |

### 5.2 How the gate actually works

The gate is **not** a factory invention. It is the existing `zylcode_mcp` permission policy at the
single dispatch chokepoint. The factory adds only the *durable parking*:

```text
dispatch() decides RequireApproval
   → the task becomes AwaitingApproval (durable, survives restart)
   → run() returns AWAITING_APPROVAL and stops
   → FactoryRunner::approve(task_id, approver) records who approved
   → the next run() re-dispatches with ToolContext::approval_required = true
   → the policy sees the approval and allows
```

**Why this is not theatre:** the approval is (a) attributed to a named human, (b) persisted, (c)
required by the *same* policy object that would otherwise deny, and (d) only effective for that
task. Approving the wrong task id is **refused**.

### 5.3 The design rule the directive states

> *Agents should not stop every few minutes for trivial decisions. But autonomy must not mean
> uncontrolled mutation.*

The mechanism that satisfies both halves is the **risk ceiling**, chosen deliberately by whoever
constructs the runner's `ToolRuntime`. A factory job that only reads sets `Read`; a job that edits
sets `Write`; a job that needs to run tests sets `Execute`. Nothing above the ceiling happens
without a human, and the ceiling is a single, auditable value.

---

## 6. Failure recovery (Phase 20)

### 6.1 What must survive, and what does

| Failure | Behaviour | Status |
|---|---|---|
| application restart | job document + evidence stores are on disk; `FactoryRunner::open` resumes | **IMPLEMENTED** |
| machine restart | same — no in-memory state is required to resume | **IMPLEMENTED** |
| provider failure | `TaskAction::ModelTask` → `Blocked` / `BLOCKED_PROVIDER`; deterministic tasks continue | **IMPLEMENTED** |
| tool failure | the task fails; its dependents are cancelled; a `Failure` record is written | **IMPLEMENTED** |
| rate limit | **DESIGNED** (backoff policy) | — |
| network loss | tool-level: `shell.execute` records the failure honestly | **IMPLEMENTED as a record** |
| agent crash | last persisted transition is the resume point | **IMPLEMENTED** |
| merge conflict | **DESIGNED** (parallel worktrees) | — |
| test failure | `CONTRADICTED` claim + `Failure`; dependents cancelled | **IMPLEMENTED** |
| partial output | a task is atomic in the document: it is `Running` until it records an outcome | **IMPLEMENTED** |

### 6.2 The recovery rule

> **A durable job resumes from evidence, not from a hallucinated narrative.**

The runner re-reads the graph from the job document. A task already `Succeeded` is not re-executed
(its `attempts` counter is the proof). The evidence spine is re-read from the evidence graph, so the
provenance chain across a restart is unbroken rather than restarted.

### 6.3 What is *not* recovered

**Rollback of a landed side effect.** If a `WriteFile` task succeeded and a later task failed, the
runner does **not** revert the write. Compensation/rollback is **DESIGNED**. Claiming otherwise
would be false: the implemented slice propagates failure forward and never pretends to undo the
past.

---

## 7. Execution ordering and the parallel question

The runner executes ready tasks **sequentially in declaration order**. The graph records genuine
dependencies, so parallel-safe tasks are *identifiable*; they are not *executed* in parallel.

**Why sequential, deliberately.** Parallel execution requires isolation (worktrees exist) plus file
ownership (does not exist) plus a merge strategy (does not exist). Running tasks concurrently
without those would race over a single working tree — and this repository carries a deliberately
preserved dirty tree that must not be raced over. Sequential execution is the honest maximum of what
the current primitives support.

The path to parallelism is: worktree per task → file lease → dependency-aware scheduler → merge
queue. Only the first exists.

---

## 8. Test coverage of this model

| Behaviour | Test |
|---|---|
| dependency-ordered execution + real tools + evidence | `tests/factory_job_e2e.rs::the_slice_executes_in_dependency_order_with_real_tools_and_evidence` |
| model task blocked, no claim invented | `::a_model_task_is_blocked_provider_and_never_faked` |
| approval parks and resumes; wrong id refused | `::a_human_approval_gate_parks_and_then_resumes` |
| failed dependency cancels dependents; contradicted claim | `::a_failed_dependency_cancels_its_dependents` |
| policy refusal → approval → execution | `::a_policy_refusal_parks_for_approval_then_executes_after_approval` |
| resume from store without repeating work | `::a_job_resumes_from_the_store_and_does_not_repeat_completed_work` |
| invalid graph refused | `::an_invalid_graph_is_refused_before_anything_runs` |
| DAG semantics (diamond, cycle, orphan, readiness) | `factory/graph.rs` unit tests (9) |
| forward-only stage projection | `factory/job.rs` unit tests (7) |
| atomic persistence, invalid-graph refusal, corrupt-file surfacing | `factory/store.rs` unit tests (7) |

The exact counts and command output are in the gate report. This table names the tests; it does not
substitute for running them.
