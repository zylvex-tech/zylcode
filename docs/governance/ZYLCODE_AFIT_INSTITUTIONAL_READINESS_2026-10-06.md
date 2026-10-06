# ZYLCODE — INSTITUTIONAL / AFIT DEMONSTRATION READINESS

**Date:** 2026-10-06
**Status:** Assessment. **No claim in this document is made beyond what the evidence supports.**
**Covers directive phase:** 22.

> **This is not an AFIT-branded product document.** AFIT is treated as **one possible institutional
> user**, never as the product's architecture. No AFIT name appears in any code, config or schema.
> The institutional requirement is used as a **design test** — "could this support a university
> software laboratory?" — and the answer below is honest about how far away that is.

---

## 1. The question this document answers

> *What can ZylCode truthfully demonstrate TODAY to an engineering or research institution?*

Not "what could it do", not "what is planned". What can be put in front of a professor, a lab
director or a research group, run live, and survive scrutiny.

---

## 2. Classification

The directive's four buckets, applied without optimism.

### 2.1 DEMONSTRABLE NOW

These run today, produce real output, and can be shown live. Each is backed by evidence produced in
this session or a documented prior session.

| # | Capability | What is shown live | Evidence |
|---|---|---|---|
| D1 | **Repository intelligence on a real repository** | point it at a real repo; it indexes files/symbols/packages and answers "where is X", "what depends on Y" | `intelligence/*`; live log "444 files, 2111 symbols, 6 packages"; deterministic retrieval benchmark P@10=0.48 R@10=1.00 across 10 identical runs |
| D2 | **A dependency-aware engineering plan** | a task graph with real dependencies, validated for cycles, executed in dependency order | `factory/graph.rs`; this wave |
| D3 | **Real tool execution with a permission gate** | a file write and a command run, each gated, each recorded | `real_tools.rs`; `integration_test.rs`; this wave's `factory_job_e2e.rs` |
| D4 | **Evidence for every action** | an evidence graph tracing each step back to the originating intent; a hash-chained claim store | `evidence_graph.rs`, `claim.rs`; traversable ancestry asserted in this wave's e2e test |
| D5 | **A durable job that survives a restart** | kill it mid-job, restart, watch it resume without repeating completed work | `factory_job_e2e.rs::a_job_resumes_from_the_store_and_does_not_repeat_completed_work` |
| D6 | **Honest blocking** | a model-requiring task reports `BLOCKED_PROVIDER` and produces no claim | `factory_job_e2e.rs::a_model_task_is_blocked_provider_and_never_faked` |
| D7 | **A human approval gate** | a risky operation parks; approving resumes it; approving the wrong task is refused | `factory_job_e2e.rs::a_human_approval_gate_parks_and_then_resumes` |
| D8 | **A real build and test pipeline** | `cargo build`, `cargo test`, packaging steps | `delivery.rs`; this session's own battery |
| D9 | **A failure that is recorded, not hidden** | a command that fails its expectation produces a `CONTRADICTED` claim and a `Failure` record | `factory_job_e2e.rs::a_failed_dependency_cancels_its_dependents` |
| D10 | **Retracted-claim enforcement** | a CI-style guard that fails on a known-unsupported claim re-entering the docs | `check_retracted_claims.py`, falsification-proven |

**The honest framing for D1–D10:** *this is a governed engineering environment with real tools and
real evidence, demonstrated on the deterministic half of the workflow.*

### 2.2 ACTIVE DEVELOPMENT

Real code exists; it is not yet demonstrable end-to-end to an institution.

| Capability | State |
|---|---|
| Agent kernel with a live model | **BLOCKED** — no usable provider |
| Best-of-N patch verification | implemented; candidate generation is model-dependent |
| MCP bridge to external servers | descriptor registry real; **no outbound client demonstrated** |
| Factory dashboard | **DESIGNED** — no surface |
| Parallel agents over worktrees | worktree primitives exist; **no scheduler** |
| Editor save from the GUI | **broken** (gap G-03: the invoke command does not exist) |

### 2.3 RESEARCH DIRECTION

Designed in this wave; no code.

| Capability | Document |
|---|---|
| Research workspace with epistemic typing | `RESEARCH_MODE_ARCHITECTURE` §1, §3 |
| Reproducible computational research ("Reproduce Experiment 14") | `RESEARCH_MODE_ARCHITECTURE` §2 |
| Research-to-product provenance pipeline | `RESEARCH_MODE_ARCHITECTURE` §4 |
| Institutional mode (org/lab/course/roles) | `INSTITUTIONAL_MODE_ARCHITECTURE` §2 |
| Education mode (EXPLAIN/GUIDE/PAIR/ASSESS/REVIEW) | `INSTITUTIONAL_MODE_ARCHITECTURE` §3 |
| Private / local / sovereign provider policy | `INSTITUTIONAL_MODE_ARCHITECTURE` §4 |
| Agent evaluation framework | `AGENT_EVALUATION_MODEL` |
| Security engineering (SAST, secret scan, supply chain) | `AGENT_EVALUATION_MODEL` §4 |
| Decision memory (ADR) | `EVIDENCE_AND_PROVENANCE_MODEL` §5 |

### 2.4 BLOCKED

| Capability | Blocker | Nature |
|---|---|---|
| Any model-dependent demonstration | `AGENT-01` — OpenRouter/Anthropic 401, DeepSeek 402 | external |
| Remote CI as a public quality signal | GitHub Actions billing lock | external |
| Release installers | same billing lock | external |
| Phase 2A acceptance | independent re-acceptance not performed | internal governance |

---

## 3. The honest demonstration scenario

The directive proposes a scenario and warns: *"Do not fabricate this demonstration if the required
pieces do not yet work."*

### 3.1 The proposed scenario, assessed line by line

> *"An engineering research group creates a software project, ZylCode indexes the repository,
> produces an evidence-backed implementation plan, executes a bounded coding task, runs tests,
> records evidence and allows a supervisor to review the result."*

| Clause | Works today? | Notes |
|---|---|---|
| creates a software project | **partially** | a workspace is created; the project-identity surface is thin (B-03) |
| ZylCode indexes the repository | **yes** | D1 |
| produces an evidence-backed implementation plan | **partially** | the *task graph* is evidence-backed and dependency-aware; a *model-authored* plan is blocked. A deterministic plan cites real ranked context. |
| executes a bounded coding task | **yes** | D3 — a real file write, gated and recorded |
| runs tests | **yes** | D8 — a real command with an exit-code expectation |
| records evidence | **yes** | D4 |
| allows a supervisor to review the result | **partially** | the approval gate exists (D7) and the evidence is inspectable; a dedicated *supervisor review surface* does not exist |

**Verdict: the scenario is demonstrable with one substitution.** Replace "implementation plan" with
"dependency-aware task graph derived from real repository context", and replace "supervisor review
surface" with "the approval gate plus the evidence record". Everything else runs.

**What must NOT be said during such a demonstration:** that the agent wrote the code (it did not —
no provider), that the plan was model-authored (it was not), or that a security review occurred (no
scanner exists).

### 3.2 A truthfully-labelled demonstration

```text
1. Open a small real repository.                      → live index (D1)
2. Ask a question about it.                           → real answer with citations (D1)
3. Build a factory job: a dependency-aware graph.     → validated DAG, shown (D2)
4. Execute it: a file write, then a command.          → gated, recorded (D3)
5. Show the evidence graph, traced to the intent.     → traversable provenance (D4)
6. Kill the process mid-job; restart; show resume.    → durable resume (D5)
7. Show a model task reporting BLOCKED_PROVIDER.      → honesty (D6)
8. Show a risky step parking for approval.            → HITL (D7)
9. Show a failed expectation producing a
   CONTRADICTED claim and a Failure record.           → failure honesty (D9)
10. Show the retracted-claim guard failing on a
    re-introduced unsupported claim.                  → governance (D10)
```

Steps 1–10 all work today. **Steps 7, 9 and 10 are the most valuable**, because they demonstrate the
thing an institution cannot get elsewhere: a system that reports its own limits accurately.

---

## 4. What an institution would actually be evaluating

Not features. Three properties:

1. **Can it be trusted about its own state?** (D4, D5, D6, D9, D10) — **yes, demonstrably.**
2. **Does it do real work with real tools?** (D1, D3, D8) — **yes, on the deterministic half.**
3. **Can it do AI-driven engineering?** — **no, blocked externally.**

**A truthful institutional conversation therefore has this shape:** *"Here is a governed environment
where AI-assisted engineering can be built safely and measured. The AI half is blocked on provider
access we do not currently have. The governance, evidence and tooling half is real and we can show
it now."*

That is a weaker pitch than a demo of an agent writing code. It is also the only one that survives a
follow-up question.

---

## 5. Readiness by institutional requirement

Using the directive's own list (master architecture §1).

| Requirement | Ready? | Gap |
|---|---|---|
| university software laboratory | **no** | multi-user identity, roles |
| research group | **no** | research workspace |
| engineering software project | **partially** | project identity surface thin |
| AI/robotics research team | **no** | reproducible research |
| indigenous software development | **partially** | offline mode partial; no policy layer |
| student software engineering | **no** | education mode |
| faculty-led research | **no** | supervisor review surface |
| reproducible computational research | **no** | 4 capture fields missing |
| secure/private development | **no** | provider policy |
| collaborative human/AI engineering | **partially** | HITL gates exist; single-user |
| research-to-product translation | **no** | provenance pipeline |

**Zero requirements are fully ready. Four are partially ready. Seven are not.**

---

## 6. Recommended next moves (not commitments)

Ordered by *value per unit of work*, and by what is actually unblocked:

1. **Build the five provider-free evaluation cases** (`AGENT_EVALUATION_MODEL` §3, rows 4–7). They
   measure honesty, they run today, and their failure would be the most damaging.
2. **Fix gap G-03** — the editor save command. It is the broken link in the core loop and it is
   small.
3. **Add the four reproduction capture fields** and make one experiment reproducible end to end.
   This converts a research *direction* into a research *capability*.
4. **Request-scoped actor identity**, then provider policy, then the four institutional policy
   sentences.
5. **Resolve the provider blocker.** Everything in the AI half of this document is downstream of it.

---

## 7. Closing statement

ZylCode today is a **governed engineering environment with real tools, real evidence, and an honest
account of its own limits.** It is not yet a software factory, not yet a research platform, and not
yet institutional. This wave built the durable orchestration foundation — a dependency-aware task
graph, a durable job, and a runner that executes through real tools and records what happened — and
it designed the rest without pretending any of it exists.

**The single most defensible claim it can make to an institution is this:** *it will tell you what
it did not do.*
