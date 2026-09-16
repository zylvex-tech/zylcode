# ZylCode Product Constitution v2.0

> **ZylCode is an evidence-first Software Creation Operating System.**
>
> **Imagine → Specify → Design → Build → Run → See → Test → Repair → Verify → Ship**

**Status:** GOVERNING
**Supersedes:** `docs/governance/superseded/ZYLCODE_CONSTITUTION.md` (six-engine draft, never committed)
**Amendments:** append-only, see §14

---

## 0. Preamble

This document is the supreme governing specification for ZylCode.

Every architectural decision, every implementation phase, every agent — human or AI — that
works on ZylCode, and every capability claim made about ZylCode must be consistent with this
Constitution.

Where an implementation conflicts with this Constitution, **this Constitution wins** unless an
amendment is recorded under §14 with rationale, date, and author.

Where an implementation conflicts with this Constitution and the conflict is not an
amendment, the correct response is **not** to silently choose one. It is to **investigate the
discrepancy and record it**. See §9.

This is a living document. Amendments are append-only. Nothing is silently rewritten.

---

## 1. Product Definition

### 1.1 What ZylCode Is

ZylCode is a **Software Creation Operating System**.

It is a single integrated environment in which a person — or a team of humans and AI agents —
takes a software product from intent through specification, architecture, visual design, code,
build, test, runtime inspection, verification, packaging, and release.

The IDE is the shell. Beneath it are engines that make software creation **composable,
verifiable, and observable**.

### 1.2 What ZylCode Is Not

- Not an IDE with AI features bolted on.
- Not a chatbot that edits files.
- Not a wrapper around a single model provider.
- Not a Figma clone with code export.
- Not a CI/CD platform with a text editor.
- Not a marketplace with a runtime.

It is the integration of those concerns into one coherent system in which the boundaries
between design, code, build, test, and delivery are dissolved.

### 1.3 The North Star — The Closed Loop

ZylCode's job is not to generate code. Its job is to move software from **intent** to
**verified deliverable**.

```
INTENT
  ↓
PROJECT
  ↓
UNDERSTAND
  ↓
PLAN
  ↓
DESIGN ←─────────────┐
  ↓                  │
IMPLEMENT             │
  ↓                  │
RUN                   │
  ↓                  │
OBSERVE               │
  ↓                  │
VERIFY                │
  ↓                   │
PASS? ── NO → DIAGNOSE → REPAIR
  │
 YES
  ↓
PROOF
  ↓
DELIVER
```

**Every capability is justified by the loop.** If a proposed feature does not strengthen some
arc of this loop, it is not a priority — regardless of how impressive it is in isolation.

The differentiator is the closed loop, not any single engine.

### 1.4 The Governing Demonstration

Product decisions are governed by one workflow, not by a feature list.

A developer opens ZylCode and says: *"New Project → Build a SaaS analytics dashboard."*

```
 0– 5s   Project created, intent captured
 5–10s   Project and Mission created
10–15s   Planner produces architecture and tasks
15–20s   Vision Studio produces editable UI
20–30s   Implementation proceeds; live Artifact updates
30–35s   ZylCode launches the application
35–40s   ZylCode interacts with it and detects a UI/runtime defect
40–45s   ZylCode repairs the defect itself
45–50s   Tests pass
50–55s   Android build launches on the emulator
55–60s   Evidence opens:

           Architecture       VERIFIED
           Design             VERIFIED
           Build              VERIFIED
           Browser workflow   VERIFIED
           Tests              VERIFIED
           Visual QA          VERIFIED
           Android            VERIFIED
```

**The timings are aspirational. The workflow is the requirement.** A literal 60-second
execution is not promised; a complete, evidence-backed loop of this shape is.

---

## 2. The Eight Core Systems

ZylCode is composed of **eight** core systems.

Two of them are additions made after the original six-engine draft, and neither is optional:

- The **seventh** — the **Project System** — is not a UI folder. Projects define the persistent
  identity and shared world state of everything ZylCode does. Without it, the other systems have
  nowhere to agree on what they are building.
- The **eighth** — the **Computer-Use Engine** (v2.1) — drives the user's real desktop and
  arbitrary third-party applications. It is not a browser feature. Driving GUI applications
  carries its own perception, grounding, permission, action, verification and evidence
  obligations; those obligations must be visible at the architecture layer or they will be
  forgotten in the implementation layer.

| # | System | Responsibility |
|---|---|---|
| **1** | **Project System** | Persistent project / workspace / product identity and shared state |
| **2** | **Agent Kernel** | Reasoning, planning, tools, workflows, models, permissions, memory |
| **3** | **Intelligence Graph** | Repository, symbols, dependencies, architecture, history, runtime knowledge |
| **4** | **Vision Studio** | UI/UX design, design systems, prototypes, and design↔code |
| **5** | **Execution Engine** | Shell, browser, desktop, containers, Android, iOS, cloud |
| **6** | **Proof Engine** | Build / test / runtime / visual / security / evidence verification |
| **7** | **Delivery Engine** | Git, CI, packaging, deployment, app stores, releases |
| **8** | **Computer-Use Engine** | Perceiving and driving real desktop applications, under permission, with replayable evidence |

### 2.0.1 The acting principle (v2.1)

> **Acting is not the same as having acted.**

An action directed at the outside world is not complete when it is issued. It is complete when
its effect has been **re-observed**. Any system that reports actions as successful without a
verification observation is violating this principle, and evidence drawn from it is not
admissible. This principle is what separates the Computer-Use Engine from a macro recorder.

### 2.0.2 The grounding principle (v2.1)

> **A confidence value that is not derived from a measurement is a defect, not a placeholder.**

A fabricated element position with a fabricated `0.85` confidence is worse than no element at
all, because downstream reasoning cannot distinguish it from a real one. Every percept must
carry its provenance and the method that produced it.

### 2.1 System Topology

```
                 ZYLCODE SOFTWARE CREATION OS

 ┌──────────────── PRODUCT EXPERIENCE ────────────────┐
 │ Projects | Code | Design | Artifacts | Run | Ship │
 └────────────────────────────────────────────────────┘

 ┌────────────────── CORE SYSTEMS ────────────────────┐
 │ Project System                                     │
 │ Agent Kernel                                       │
 │ Intelligence Graph                                 │
 │ Vision Studio                                      │
 │ Execution Engine                                   │
 │ Proof Engine                                       │
 │ Delivery Engine                                    │
 │ Computer-Use Engine              (v2.1)            │
 └────────────────────────────────────────────────────┘

 ┌────────────── CROSS-CUTTING PLATFORMS ─────────────┐
 │ Extension Platform | Model Platform | Artifact Bus │
 └────────────────────────────────────────────────────┘

 ┌──────────────── TRUST FOUNDATION ──────────────────┐
 │ Evidence Ledger | Permissions | Recovery | Audit   │
 └────────────────────────────────────────────────────┘
```

### 2.2 Cross-Cutting Platforms

- **Extension Platform** — the ABI through which tools, skills, agents, MCP servers, model
  providers, runtimes, design libraries and verification providers are added *without*
  modifying core.
- **Model Platform** — capability-based model routing across providers and local models.
- **Artifact Bus** — the canonical channel by which agents, runtimes and the Proof Engine
  produce and cite persistent, inspectable outputs.

### 2.3 Trust Foundation

- **Evidence Ledger** — append-only record of what was claimed, executed, observed, and proven.
- **Permissions** — fail-closed authorization for every action with side effects.
- **Recovery** — durable, resumable execution across process death.
- **Audit** — independent verification of every capability claim.

---

## 3. The Evidence Principle

This is the defining law of ZylCode.

> **A claim is not a fact until it is independently reproducible.**

ZylCode is *evidence-first* not as a slogan but as an engineering constraint. This applies to
ZylCode's own development with the same force as it applies to the software ZylCode builds.

### 3.1 The Proof Ladder

Every capability, in every system, carries a proof rung:

| Rung | Name | Meaning |
|---|---|---|
| **R0** | **CLAIMED** | Asserted in documentation or a report. No artifact. |
| **R1** | **OBSERVED** | Seen once, informally. No reproducible procedure. |
| **R2** | **EXECUTED** | Ran under automated test or script. Not reachable by a user or agent. |
| **R3** | **VERIFIED** | Reachable and usable through a product surface, with captured evidence. |
| **R4** | **REPRODUCIBLE** | A third party re-runs a committed procedure and obtains the same result. |
| **R5** | **COMMISSIONED** | Independently audited by a party that did not build it. Accepted. |

**The full definition is `ZYLCODE_PROOF_GRAPH.md`. It governs all capability claims.**

Rungs apply to *any* capability — a UI feature, a parser, a deployment path, a model route —
not only to source code. The former build-centric ladder in `README.md` is superseded by this
one for governance purposes.

### 3.2 Non-Negotiable Rules

1. **Builders do not certify their own work.** A phase is complete when an *independent* audit
   accepts it, not when the author reports it done.
2. **Unit tests are R2, never R3.** A tested module that no user or agent can reach is not a
   capability. It is a library.
3. **Every completion report carries a reproduction block** — exact commands, environment,
   commit SHA, and raw captured output. A report without one is a draft.
4. **Metrics must be scoped to the thing they name.** A count of "files indexed" that is
   dominated by build output is not a count of repository files. See the Phase 2A audit.
5. **Never silently correct a published number.** Corrections are visible as corrections.
6. **Absence of CI is not permission to lower the bar.** When remote CI is unavailable, local
   reproduction becomes the *only* evidence, and therefore becomes *more* important.
7. **Fail closed.** Unsupported input returns an explicit `UNSUPPORTED_*` status. It never
   silently degrades to a plausible-looking wrong answer.

### 3.3 The Rung Ceiling Rule

A system's rung cannot exceed the rung of the weakest link in its critical path.

> If the scanner is R2 and the query API is R3, the Intelligence Graph is **R2**.

Capability status is determined by the *lowest* proven component, not the highest.

---

## 4. The Project Principle

> **A Project is not a directory.**
>
> A Project is ZylCode's persistent representation of a software product.

A directory is a filesystem location. A Project is the durable identity that survives
application restart, machine migration, agent turnover, and model changes.

A Project owns and persists:

requirements · repositories · architecture · design · conversations · missions · agents ·
decisions · evidence · artifacts · runtimes · model configuration · build configuration ·
releases · deployment history

**Corollary:** the Project System must exist before advanced multi-agent behaviour, before
Vision Studio, and before the Extension Platform. Building those first would mean rebuilding
them once the shared world state is defined. This is why Phase 3A precedes Phase 4 and beyond.

---

## 5. The Mission Principle

ZylCode does not present "Ask AI" as its primary interaction. It presents **Mission**.

A Mission is a unit of intent with a stated objective and explicit acceptance criteria:

```
MISSION      Implement password reset.

OBJECTIVE    Users can request and complete password resets.

ACCEPTANCE   ✓ Request form
             ✓ Email flow
             ✓ Token expiration
             ✓ Password validation
             ✓ Unit tests
             ✓ Integration tests
             ✓ Browser verification
```

Mission lifecycle:

```
CREATED → UNDERSTANDING → PLANNING → AWAITING_APPROVAL
        → EXECUTING → VERIFYING → REPAIRING
        → COMPLETE | BLOCKED | FAILED | CANCELLED
```

Missions are **resumable, inspectable, and evidence-backed**. A Mission cannot reach
`COMPLETE` without satisfying every acceptance criterion with evidence. Acceptance criteria
are the contract; the Proof Engine is the enforcement.

---

## 6. The Model Democracy Principle

> **A dropdown is not Model Democracy.**

The system must not merely ask *"which model do you want?"* It must reason:

```
Task
 ↓
Capability requirements
 ↓
Candidate models
 ↓
Historical verified performance
 ↓
Privacy requirements
 ↓
Cost
 ↓
Latency
 ↓
Context requirement
 ↓
Selected model
```

Providers include OpenAI, Anthropic, Google, DeepSeek, Z.AI/GLM, OpenRouter and local models.

**Model Democracy is PARTIAL until routing is based on measured results.** Routing decisions
must be recorded as evidence and their outcomes verified, so that "this model is better at
this task" is a measured claim rather than a preference.

---

## 7. The Extensibility Principle

> **Extension ABI first. Marketplace second.**

If extensibility is deferred, core accumulates hard-coded integrations that must later be
redesigned as extensions — at a cost that grows with every integration added.

A ZylCode Package may provide:

```
manifest
├── tools
├── MCP servers
├── skills
├── agents
├── commands
├── hooks
├── model providers
├── UI panels
├── runtimes
├── templates
├── design libraries
└── verification providers
```

This is what allows Android, AWS, Unity, Unreal, Blender, FreeCAD, Kubernetes, database and
cloud integrations to exist **without bloating core**.

The **marketplace** — signed packages, permissions, versioning, compatibility, ratings,
publisher identity, updates, security review — is a later phase. The **ABI** is not.

---

## 8. The Vision Principle

### 8.1 Design must be machine-readable and agent-addressable

The AI must manipulate **structured design primitives**, not screenshots. Screenshots are
observation. Primitives are the medium.

### 8.2 The canonical design representation comes first

Before a canvas, before components, before variants: define `DesignDocument`, `Page`, `Frame`,
`Component`, `Instance`, `Variant`, `Text`, `Vector`, `Image`, `Layout`, `Constraint`,
`Interaction`, `Token`, `Variable`, `Style`.

### 8.3 Design must not become web-centric

```
DESIGN MODEL
     ↕
UI INTERMEDIATE REPRESENTATION
     ↕
FRAMEWORK ADAPTER
```

Adapters: React · Next.js · HTML/CSS · Flutter · Jetpack Compose · SwiftUI · React Native.

**React DOM must never be the design model.** If it is, Vision Studio is permanently
web-centric and every non-web target becomes a translation of a translation.

### 8.4 Seeing your own work

Visual intelligence closes the loop:

```
EXPECTED DESIGN → RENDERED SOFTWARE → SCREENSHOT → VISUAL ANALYSIS
   → DIFFERENCE MODEL → REPAIR PLAN → CODE/DESIGN CHANGE → RENDER AGAIN
```

This is where ZylCode begins to **see its own work** rather than assume it.

---

## 9. Governance and the Agent Protocol

### 9.1 The Governing Documents

```
docs/governance/
    ZYLCODE_PRODUCT_CONSTITUTION_V2.md     ← this document (supreme)
    ZYLCODE_ARCHITECTURE_V2.md
    ZYLCODE_MASTER_EXECUTION_PLAN.md
    ZYLCODE_CAPABILITY_MODEL.md
    ZYLCODE_PROOF_GRAPH.md
    ZYLCODE_AGENT_OPERATING_PROTOCOL.md

docs/roadmap/
    ZYLCODE_ROADMAP_V2.md

docs/architecture/
    PROJECT_SYSTEM.md
    AGENT_KERNEL.md
    INTELLIGENCE_GRAPH.md
    EXTENSION_PLATFORM.md
    ARTIFACT_SYSTEM.md
    VISION_STUDIO.md
    EXECUTION_ENGINE.md
    PROOF_ENGINE.md
    DELIVERY_ENGINE.md
```

### 9.2 Mandatory Agent Preamble

**Every** prompt issued to DeepSeek, Codex, ZCode, or any other implementation agent must
begin with:

> Read the ZylCode Constitution, Architecture, Master Execution Plan, Capability Model and
> Proof Graph, plus the current phase specification. They govern implementation.
> Where implementation conflicts with documentation, **investigate the discrepancy rather
> than silently choosing one**.

This exists to solve a specific, repeatedly observed failure: **agents forgetting what
product they are building while they implement an individual task.**

### 9.3 PROPOSED Marking

Any architecture document describing a system that has not been implemented must state
**PROPOSED** at the top of the document and at each unimplemented section.

Documentation must never imply that a system exists because it is described. Describing a
system is not building it. Marking is mandatory and is itself auditable.

### 9.4 Independent Audit

> **DeepSeek builds. It does not certify its own work.**

When a phase is reported complete, the report is brought to an **independent auditor** who
inspects the actual commit, the implementation, and the available CI evidence before the phase
is accepted.

A phase is one of: **ACCEPTED**, **ACCEPTED WITH CONDITIONS**, or **NOT ACCEPTED (re-opened)**.

---

## 10. Safety and Trust Boundaries

### 10.1 Permissions

Every action with side effects is classified by risk. Destructive or external actions require
explicit authorization. Permissions are **fail-closed**: absence of a grant is denial.

### 10.2 Recovery

Execution is durable. A crash mid-mission must be resumable without re-executing completed,
non-idempotent work. Ambiguous post-crash states are *reconciled against external truth*
(e.g. git state), never guessed.

### 10.3 Local-first / sovereignty

Indexing and analysis are local by default. Secrets are excluded from indexing by construction.
No repository content leaves the machine without explicit user action.

### 10.4 Secret handling

Secret files are excluded from the intelligence index. This exclusion is a security control
and is subject to the same proof requirements as any other capability — it must be verified
against the *real* repository, not a fixture.

---

## 11. Truth-in-Advertising

ZylCode's public claims — README, website, release notes, capability registry — must be
traceable to evidence at the claimed rung.

- No capability is described as working before it is **R3**.
- No metric is published without a reproduction procedure.
- No status is described as complete while its proof rung says otherwise.
- Where a claim is corrected, the correction is visible.

**The capability registry is a public artifact and is held to the same standard as the code.**

---

## 12. Quality Bar

1. **Correctness before capability.** A slow, correct scanner beats a fast, wrong one.
2. **Explicit failure over plausible wrongness.** Fail closed, always.
3. **Determinism where measurement is involved.** A benchmark that varies between runs cannot gate anything.
4. **Reachability.** If a user or agent cannot invoke it, it is not shipped.
5. **Reversibility.** Prefer changes that can be undone.
6. **Boring, readable code.** This system will be maintained by many agents over years.

---

## 13. Amendment Procedure

1. Amendments are **append-only**. History is never rewritten.
2. Every amendment records: date, author, the changed clause, the rationale, and the evidence.
3. An amendment that changes §1–§3 requires independent review.
4. Implementation that contradicts this Constitution without an amendment is a **defect**,
   to be recorded as such — not quietly resolved in either direction.

### Amendments

*(none yet — this is the initial ratified version)*

---

## 14. Ratification

**Version:** 2.0
**Ratified:** 2026-09-16
**Authority:** Product owner
**Supersedes:** six-engine draft constitution (uncommitted, archived under `docs/governance/superseded/`)
**Governing scope:** all ZylCode development, all agents, all published claims

The immediate consequence of ratification:

- The **seven**-system model replaces the six-engine model everywhere.
- The **16-phase / 6-epoch** program replaces the 14-stage program everywhere.
- The **R0–R5** proof ladder replaces the build-centric ladder for governance.
- **Phase 2A is NOT ACCEPTED** pending the remediation in `PHASE2A_INDEPENDENT_AUDIT.md`.
- **Phase 2B does not begin** until Phase 2A is re-audited and accepted.
