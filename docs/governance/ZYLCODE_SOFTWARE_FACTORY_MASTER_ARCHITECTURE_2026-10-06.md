# ZYLCODE SOFTWARE FACTORY — MASTER ARCHITECTURE

**Date:** 2026-10-06
**Status:** GOVERNING FOR THIS WAVE. Subordinate to `ZYLCODE_PRODUCT_CONSTITUTION_V2.md`,
`ZYLCODE_ARCHITECTURE_V2.md`, `ZYLCODE_PROOF_GRAPH.md` and `ZYLCODE_AGENT_OPERATING_PROTOCOL.md`.
Where this document and a ratified governing document disagree, **the governing document wins and
this document is wrong.**
**Scope:** the architecture that turns ZylCode from an AI coding IDE into an AI-native software,
research and software-factory platform.
**Covers directive phases:** 2 (factory model), 3 (orchestrator), 4 (task graph), 18 (UI/UX factory),
19 (worktrees / parallel agents), 21 (factory dashboard).

---

## 0. The claim discipline this document obeys

Three words are used with exact meanings and no others.

| Word | Means |
|---|---|
| **IMPLEMENTED** | Code exists in this tree, on this branch, at the commit named in the gate report. |
| **DESIGNED** | Specified here well enough to build. **Nothing has been built.** |
| **BLOCKED** | Cannot be reached for a stated reason. Never a synonym for "not done". |

A capability that is designed but not built is labelled **DESIGNED** in every table. Describing a
system is not building it (Constitution §9.3). This document contains a great deal of **DESIGNED**.

---

## 1. What ZylCode is becoming

```text
IDEA → REQUIREMENTS → RESEARCH → ARCHITECTURE → DESIGN → IMPLEMENTATION → TESTING
     → VERIFICATION → SECURITY REVIEW → DOCUMENTATION → BUILD → RELEASE
     → DEPLOYMENT → MAINTENANCE → CONTINUOUS IMPROVEMENT
```

Today ZylCode executes a real, narrow band of that arc and documents the rest. The architecture
below is how the band widens **without** inventing capability.

### 1.1 The one rule that shapes every decision

> **Every capability must eventually execute through a real implementation, tool, model,
> repository action or evidence-producing workflow.**

A button that does nothing is worse than an absent button, because it is a claim. The architecture
therefore refuses to model a stage it cannot execute; it models the stage and marks it **DESIGNED**
or **BLOCKED** until an executor exists.

---

## 2. The eight systems, and where the factory sits

`ZYLCODE_ARCHITECTURE_V2.md` defines eight systems. The factory is not a ninth. It is the
**composition** of existing systems under a durable job:

| Existing system | Role in the factory |
|---|---|
| Project System | Owns the job's project identity (`project_id`), not a directory. |
| Agent Kernel | Executes model-driven steps. **Provider-BLOCKED today.** |
| Intelligence Graph | Supplies repository context before any change (directive Phase 5). |
| Vision Studio | Supplies design representation (directive Phase 18). **PROPOSED.** |
| Execution Engine | Runs real tools. This is where factory tasks actually happen. |
| Proof Engine | Records deterministic verification (directive Phase 12). |
| Delivery Engine | Builds and packages (directive Phase 21 / T). |
| Computer-Use Engine | **R0 simulation facade. Must be replaced, not extended.** |

The factory adds a **durable orchestration layer over these**, plus two new durable stores:
`crates/zylcode-core/src/factory/` (job + graph) and the existing evidence stores it writes into.

---

## 3. The factory model — roles, not agents

The directive names sixteen roles. **A role is a responsibility, not a process.** Creating sixteen
"agents" that share one code path would satisfy the letter of the directive and none of its intent;
the directive itself forbids it ("Do not create fake independent agents merely to satisfy this
list").

Each role is classified by **what actually discharges it**:

| # | Role | Discharged by | Status |
|---|---|---|---|
| 1 | PRODUCT / REQUIREMENTS | Model prompt → structured requirements artifact | **DESIGNED** (prompt blocked on provider) |
| 2 | RESEARCH | Research workspace + source classification | **DESIGNED** (see RESEARCH_MODE doc) |
| 3 | SYSTEM ARCHITECT | Model prompt + ADR record | **DESIGNED** |
| 4 | REPOSITORY INTELLIGENCE | Deterministic services (`intelligence/*`) | **IMPLEMENTED** |
| 5 | IMPLEMENTATION | Tool pipeline (`fs.write`) + model patch proposal | **IMPLEMENTED** (tool path); model path **BLOCKED** |
| 6 | TEST ENGINEER | Deterministic tool pipeline (`shell.execute`) | **IMPLEMENTED** |
| 7 | DEBUGGING | Signature parser + model hypothesis | **IMPLEMENTED** (parser); model **BLOCKED** |
| 8 | SECURITY REVIEWER | Deterministic scanners | **DESIGNED** — no scanner exists |
| 9 | PERFORMANCE ENGINEER | Benchmark harness | **DESIGNED** |
| 10 | DEPENDENCY / SUPPLY-CHAIN REVIEWER | Lockfile + advisory DB | **DESIGNED** |
| 11 | DOCUMENTATION | Deterministic generator + model prose | **DESIGNED** |
| 12 | UI/UX | Vision Studio | **PROPOSED** |
| 13 | ACCESSIBILITY REVIEWER | a11y + visual-regression tools | **DESIGNED** |
| 14 | RELEASE ENGINEER | `delivery::build_workspace` | **IMPLEMENTED** (local); remote release **BLOCKED** |
| 15 | DEVOPS / DEPLOYMENT | Deploy executor | **DESIGNED** — `deploy_targets()` enumerates only |
| 16 | CODE REVIEWER | Model review + deterministic checks | **DESIGNED** |
| 17 | EVIDENCE / GOVERNANCE | `claim`, `evidence_graph`, `proof_engine` | **IMPLEMENTED** |

**The classification is the architecture.** A role that can be discharged deterministically is
never given to a model, because a deterministic check is cheaper, reproducible, and cannot
hallucinate. A role that genuinely needs judgement is a model prompt, and it is **BLOCKED** while no
provider answers.

### 3.1 Role → task-kind mapping (implemented)

`crates/zylcode-core/src/factory/graph.rs` encodes the roles that can be executed today as
`TaskKind`. The mapping is deliberate: **only roles with a real executor became task kinds.**

```rust
pub enum TaskKind {
    Requirements, Research, Architecture, RepositoryContext, Implementation,
    Test, Debug, SecurityReview, Documentation, ReleaseReadiness, HumanApproval,
}
```

A `TaskKind` is not a promise that the role is smart. It is a promise that the role has a
**discharge mechanism**. `TaskAction::ModelTask` is the only mechanism that requires a model, and it
is the only one that returns `BLOCKED_PROVIDER`.

---

## 4. The orchestrator — a durable job over a task graph

### 4.1 The job lifecycle (directive Phase 3)

```text
INTAKE → DISCOVERY → REQUIREMENTS → RESEARCH → PLAN → ARCHITECTURE → TASK_GRAPH
       → IMPLEMENTATION → TEST → REVIEW → SECURITY → DOCUMENTATION
       → RELEASE_READINESS → OWNER_APPROVAL → DELIVERY
```

`JobStage` (**IMPLEMENTED**) is a **projection**, not an independent state machine. It is derived
from what the graph has actually achieved and moves **forward only**. A job at `TEST` does not fall
back to `PLAN` because a later task is pending. The stage is therefore a **conservative lower bound
on progress: it can lag reality and can never lead it.** A stage that led reality would be a
fabricated status, which is the one thing this product may not produce.

### 4.2 What every transition knows (directive Phase 3)

| Field | Where it lives | Status |
|---|---|---|
| job ID, project ID | `FactoryJob` | **IMPLEMENTED** |
| repository, branch/worktree | `FactoryJob` | **IMPLEMENTED** |
| current phase | `FactoryJob::stage` | **IMPLEMENTED** |
| responsible role | `TaskNode::kind` → `TaskKind::role()` | **IMPLEMENTED** |
| input artifacts | `TaskNode::action` + evidence-node `inputs` | **IMPLEMENTED** |
| output artifacts | evidence-node `outputs` + `TaskNode::outcome` | **IMPLEMENTED** |
| tool calls | `zylcode_mcp` JSONL evidence sink + evidence-graph node | **IMPLEMENTED** |
| model / provider | evidence-node `model` field | **IMPLEMENTED** (unset while BLOCKED) |
| evidence | evidence graph + claim store | **IMPLEMENTED** |
| errors | `Failure` records (`failure.rs`) | **IMPLEMENTED** |
| blockers | `TaskNode::blocked_reason` | **IMPLEMENTED** |
| human decisions | `TaskNode::approved_by` + `AwaitingApproval` state | **IMPLEMENTED** |
| timestamps | `created_at` / `finished_at` on node and job | **IMPLEMENTED** |
| retries | `TaskNode::attempts` / `max_attempts` | **IMPLEMENTED** |
| rollback / recovery | dependency-failure propagation + durable resume | **IMPLEMENTED** (propagation); rollback of a landed effect is **DESIGNED** |

**Failures are not hidden.** A blocked task records its reason; a failed task records a `Failure`
with a real cause parsed from output; a dependent of either is **cancelled explicitly** rather than
left `Pending` forever.

---

## 5. The task graph (directive Phase 4)

### 5.1 What the graph supports

| Requirement | Status | Mechanism |
|---|---|---|
| dependencies | **IMPLEMENTED** | `TaskNode::depends_on`; `TaskGraph::ready()` returns only satisfied nodes |
| parallel-safe vs serial | **IMPLEMENTED (as data)** | the DAG distinguishes them; the runner executes sequentially and says so |
| blocked tasks | **IMPLEMENTED** | `TaskState::Blocked` + reason |
| retry | **IMPLEMENTED (bounded)** | `attempts` / `max_attempts`; no automatic retry loop yet |
| cancellation | **IMPLEMENTED** | `FactoryRunner::cancel` + dependency propagation |
| human approval | **IMPLEMENTED** | `TaskAction::Approval` + `AwaitingApproval` + `approve()` |
| evidence attachment | **IMPLEMENTED** | `TaskNode::evidence_node_id`, `claim_ids`, `failure_id` |
| failure propagation | **IMPLEMENTED** | `TaskGraph::orphaned()` → `Cancelled` |
| resume after restart | **IMPLEMENTED** | job saved after every transition; `JobStore::load` → `FactoryRunner::open` |

### 5.2 The canonical example

```text
"Build authentication"
  requirements → threat model → architecture → schema → backend → frontend
              → tests → security review → documentation → integration test
```

Expressed as a graph, the backend/frontend branches are **parallel-safe** (no edge between them) and
the integration test **joins** them. The graph validates acyclicity before a job may run
(`TaskGraph::validate`, Kahn's algorithm) — an unsound graph is refused, not executed partially.

### 5.3 What the graph is not

It is **not** a workflow engine with conditions, loops or compensation. Those are **DESIGNED**. The
implemented core is deliberately the smallest structure that is genuinely dependency-aware, so that
the semantics can be trusted before features are added.

---

## 6. Parallel agents over isolated worktrees (directive Phase 19)

**Status: SCAFFOLDED.** The primitives exist; the orchestration does not.

| Concern | Status | Evidence |
|---|---|---|
| throwaway worktree per candidate | **IMPLEMENTED** | `patch_best_of_n::CandidateWorktree` |
| container isolation, only worktree writable | **IMPLEMENTED** | `sandbox::SandboxedWorktreeVerifier` + tests asserting the mount policy |
| task branches | **DESIGNED** | — |
| file ownership / leases | **DESIGNED** | — |
| dependency-aware scheduler across agents | **DESIGNED** | — |
| merge queue / conflict detection | **DESIGNED** | — |
| integration branch | **DESIGNED** | — |

**The hard constraint:** agents must not race over one dirty working tree. This repository already
carries a large, deliberately-preserved dirty tree (see §9); a naive parallel agent feature would
destroy it. Parallelism therefore **starts from the worktree primitive that already exists** and is
gated behind file-lease semantics that do not exist yet.

A parent orchestrator knowing "which agent owns which task and files" is **DESIGNED**; the task
graph's `depends_on` gives the *task* half of that answer today, and the *file* half requires the
lease model.

---

## 7. The UI/UX factory (directive Phase 18)

**Status: PROPOSED (Vision Studio) with one implemented piece.**

| Stage | Status |
|---|---|
| requirements → information architecture | **DESIGNED** |
| wireframe → component design | **DESIGNED** |
| design tokens | **NOT_IMPLEMENTED** |
| editable canvas | **SCAFFOLDED** (`intelligence/canvas.rs` — no product surface renders it) |
| code generation | **DESIGNED** |
| runtime preview | **IMPLEMENTED** (embedded preview, commit `96aa8a4`) |
| accessibility | **DESIGNED** |
| visual regression | **NOT_IMPLEMENTED** |
| implementation hand-off | **DESIGNED** |

**The directive's own constraint is honoured:** *"Do not rebuild the entire ZylCode interface merely
because this wave introduces factory concepts."* The desktop shell (four zones, commits `fc93a29`,
`96aa8a4`) is ratified work and is **not touched** by this wave. The factory's own surface is
additive and, in this wave, **not built at all** (§8).

---

## 8. The factory dashboard (directive Phase 21)

**Status: DESIGNED. Not built in this wave.**

The dashboard's governing rule is that **every status shown must derive from actual state**. The
implemented substrate makes that possible; the surface does not exist yet.

| Proposed section | Derives from | Available now? |
|---|---|---|
| MISSION | `FactoryJob::intent` | yes |
| PLAN | `TaskGraph` node titles/actions | yes |
| TASK GRAPH | `TaskGraph` + states | yes |
| AGENTS | `TaskKind::role()` + node state | yes (roles, not processes) |
| REPOSITORY | `intelligence/*` | yes |
| RESEARCH | research workspace | **no — not implemented** |
| RUNS | `RunReport` | yes |
| TESTS | `shell.execute` evidence + claims | yes |
| SECURITY | security findings | **no — not implemented** |
| EVIDENCE | `EvidenceGraph` | yes |
| DECISIONS | decision memory | **no — not implemented** |
| ARTIFACTS | `artifact_bus` | yes |
| RELEASES | `delivery` | yes |

**Explicit prohibitions, from the directive, restated because they are easy to violate:**
no decorative fake percentages; no fake "agents working"; no fake test counts; no fabricated
deployment status. A section whose data does not exist is shown as **not implemented**, not as zero.

---

## 9. Architecture constraints imposed by this repository's real state

An architecture that ignores the repository it lands in is fiction. Four real constraints:

1. **A large dirty tree is deliberately preserved.** 14 uncommitted entries at the baseline,
   including `docs/governance/RECONCILIATION_FORENSIC_REPORT.md`, `tasks/`, and a
   `filesystem.rs.backup`. Ownership is unresolved. **No factory feature may commit, reset, clean or
   stash these.** The factory reads the tree; it does not tidy it.
2. **Provider access is blocked.** Every model-dependent design must degrade to
   `BLOCKED_PROVIDER` without pretending. This is why the runner's only provider-dependent variant
   is `TaskAction::ModelTask` and why it returns `Blocked`.
3. **Remote CI cannot run** (external billing lock). Verification authority is the **local battery**.
   Any factory feature that depends on CI to be proven is unprovable today.
4. **The Computer-Use Engine is a simulation facade.** It must be **replaced, not extended**. No
   factory role may be built on it.

---

## 10. What this wave actually built

Honest accounting. Everything below is **IMPLEMENTED** and tested in this session.

| Artifact | Path |
|---|---|
| Task graph (DAG, validation, readiness, propagation) | `crates/zylcode-core/src/factory/graph.rs` |
| Durable job + forward-only lifecycle projection | `crates/zylcode-core/src/factory/job.rs` |
| Atomic job store | `crates/zylcode-core/src/factory/store.rs` |
| Evidence-producing runner over the real tool runtime | `crates/zylcode-core/src/factory/runner.rs` |
| End-to-end proof of the slice | `crates/zylcode-core/tests/factory_job_e2e.rs` |
| Capability census (machine + human) | `docs/capability-census-2026-10-06.json`, `ZYLCODE_CAPABILITY_CENSUS_2026-10-06.md` |

**Explicitly NOT built in this wave:** the dashboard, the research workspace, institutional mode,
the security subsystem, decision memory, parallel scheduling, the evaluation harness. All are
**DESIGNED** in the companion documents.

---

## 11. Traceability — directive phase → where it is answered

| Directive phase | Document | Section |
|---|---|---|
| 2 Roles | this doc | §3 |
| 3 Orchestrator / lifecycle | this doc §4; `FACTORY_EXECUTION_MODEL` §2 | |
| 4 Task graph | this doc §5; `FACTORY_EXECUTION_MODEL` §3 | |
| 5 Repository intelligence | `CAPABILITY_CENSUS` C; existing `intelligence/*` | |
| 6 Research workspace | `RESEARCH_MODE_ARCHITECTURE` | |
| 7 Reproducible research | `RESEARCH_MODE_ARCHITECTURE` §4 | |
| 8 Evidence engine | `EVIDENCE_AND_PROVENANCE_MODEL` | |
| 9 Human-in-the-loop | `FACTORY_EXECUTION_MODEL` §5 | |
| 10 Agent memory | `EVIDENCE_AND_PROVENANCE_MODEL` §5 | |
| 11 Evaluation | `AGENT_EVALUATION_MODEL` | |
| 12 Verification-first | `FACTORY_EXECUTION_MODEL` §4 | |
| 13 Security | `AGENT_EVALUATION_MODEL` §4 | |
| 14 Private/local/sovereign | `INSTITUTIONAL_MODE_ARCHITECTURE` §5 | |
| 15 Institutional mode | `INSTITUTIONAL_MODE_ARCHITECTURE` §2 | |
| 16 Education mode | `INSTITUTIONAL_MODE_ARCHITECTURE` §3 | |
| 17 Research-to-product | `RESEARCH_MODE_ARCHITECTURE` §5 | |
| 18 UI/UX factory | this doc §7 | |
| 19 Worktrees / parallel | this doc §6 | |
| 20 Failure recovery | `FACTORY_EXECUTION_MODEL` §6 | |
| 21 Dashboard | this doc §8 | |
| 22 Institutional readiness | `AFIT_INSTITUTIONAL_READINESS` | |
