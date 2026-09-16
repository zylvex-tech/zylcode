# ZylCode Architecture v2.1

> Technical specification for the **eight-system** Software Creation OS.
> Companion to `ZYLCODE_PRODUCT_CONSTITUTION_V2.md`.

**Status:** GOVERNING
**Supersedes:** `docs/governance/superseded/ZYLCODE_ARCHITECTURE_V2.md` (six-engine draft, never committed);
v2.0 (seven-system revision, commit `1af2072`)
**Revision note (v2.1, 2026-09-16):** adds the **Computer-Use Engine** as the eighth core system;
records Artifact Composer, Workspace Composer, Time Travel and the Engineering Knowledge Graph;
splits Phase 7 into 7A–7D **without renumbering 8–16**. See `DEEPSEEK_MASTER_PROMPT_V21.md`.
**Implementation reality:** every system below carries an explicit rung. See §9.

---

## 0. Scope

This document defines the technical architecture of ZylCode:

1. System topology and process model
2. The eight core systems and their contracts
3. The three cross-cutting platforms
4. The trust foundation
5. Data ownership and persistence
6. The dependency order between systems
7. Cross-cutting concerns: IPC, security, observability
8. Concurrency and resource model
9. **Current implementation status of every system** (mandatory, non-optional)

All implementation must conform to this document unless a Constitution amendment is recorded.

---

## 1. Design Axioms

These are the invariants from which everything else follows. If a design decision contradicts
an axiom, the design is wrong.

| # | Axiom | Consequence |
|---|---|---|
| **A1** | **The loop is the product.** | Every system must strengthen some arc of Imagine→…→Ship. A system that only serves itself is deferred. |
| **A2** | **The Project is the shared world state.** | Agents, runtimes and engines do not communicate by passing transcripts. They read and write Project state. |
| **A3** | **Evidence is a first-class data type.** | Proof is stored, addressable, and citable — not a log line. |
| **A4** | **Design is structured data.** | The design model is machine-readable and agent-addressable. Screenshots are observation, never the medium. |
| **A5** | **Extensibility is an ABI, not a plugin folder.** | Integration points are versioned contracts from day one. |
| **A6** | **Fail closed.** | Unknown input yields an explicit `UNSUPPORTED_*` / `*_FAILED` status. Never a plausible wrong answer. |
| **A7** | **The rung ceiling holds.** | A system's proof rung cannot exceed the weakest link in its critical path. |
| **A8** | **Local-first, sovereign by default.** | Indexing and analysis are local. Egress is explicit, never implicit. |
| **A9** | **Determinism where measurement occurs.** | Anything used as a gate must be reproducible. |
| **A10** | **Reachability is part of correctness.** | An unreachable subsystem is not a capability, regardless of test coverage. |
| **A11** | **Acting is not the same as having acted.** | Every action against the outside world must be *observed* to have taken effect before it is reported as done. An action whose effect was not re-perceived is UNVERIFIED, not successful. (v2.1 — added for the Computer-Use Engine.) |

### 1.1 The grounding rule (v2.1)

Introduced with the Computer-Use Engine, and generalisable to every perception-consuming system:

> **A confidence value that is not derived from a measurement is a defect, not a placeholder.**

Fabricated grounding is worse than absent grounding, because it is indistinguishable from real
grounding at the type level. `UiElement.confidence: 0.85` on a hardcoded literal is not a
missing feature — it is an **architectural violation**, and it was found in this repository
(see §9, Computer-Use Engine). Any type that carries a confidence score must also carry the
provenance and the method that produced it, or it must not carry a score at all.

---

## 2. Process Model

ZylCode runs as a **multi-process system**. This is not incidental — it is what makes
crash-isolation, resource limits, and permission boundaries enforceable.

```
┌───────────────────────────────────────────────────────────────┐
│                      ZYLCODE HOST PROCESS                     │
│                                                               │
│  ┌────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ Project    │  │ Agent        │  │ Intelligence         │   │
│  │ System     │  │ Kernel       │  │ Graph                │   │
│  └─────┬──────┘  └──────┬───────┘  └──────────┬───────────┘   │
│        │                │                     │               │
│  ┌─────┴────────────────┴─────────────────────┴───────────┐   │
│  │                    SERVICE REGISTRY                    │   │
│  │  Project · Models · Tools · Skills · MCP · Memory      │   │
│  │  Context · Verification · Execution · Delivery · Vision│   │
│  │  Computer-Use                          (v2.1)          │   │
│  └────────────────────────┬───────────────────────────────┘   │
│                           │                                   │
│  ┌────────────────────────┴───────────────────────────────┐   │
│  │                   EVIDENCE LEDGER                      │   │
│  │  append-only · hash-chained · citable by artifact id   │   │
│  └────────────────────────────────────────────────────────┘   │
└───────────────┬──────────────────────────┬────────────────────┘
                │                          │
      ┌─────────▼──────────┐    ┌──────────▼───────────┐
      │  EXECUTION WORKERS │    │  ARTIFACT BUS        │
      │  shell · browser   │    │  previews · reports  │
      │  desktop · docker  │    │  diffs · screenshots │
      │  android · mac     │    │  traces · evidence   │
      └────────────────────┘    └──────────────────────┘

      ┌────────────────────────────────────────────────┐
      │  COMPUTER-USE WORKER  (v2.1 — PROPOSED)        │
      │  perception · grounding · action               │
      │  driven application GUI · always under         │
      │  Permission gate · every step → Flight Recorder│
      └────────────────────────────────────────────────┘
```

**Host process** owns state, scheduling, permissions and the ledger.
**Execution workers** own side effects and are killable.
**Artifact Bus** is the only sanctioned channel for large or inspectable outputs.

The **Computer-Use worker** is drawn separately because its trust profile differs from every
other worker: it drives arbitrary applications rather than a sandboxed toolchain, so its side
effects reach a user's real desktop. It is therefore subject to the Permission gate at *every*
step, not once per mission. See `docs/architecture/COMPUTER_USE_ENGINE.md`.

### 2.1 Why separate workers

- A hung browser must not freeze the UI.
- A crashed Android emulator must not corrupt the Project.
- Resource limits must be enforceable per-worker.
- Side effects must be attributable to a specific Mission step.

---

## 3. The Eight Core Systems

Each system is specified by: **Responsibility · Owns · Depends on · Exposes · Never does.**

### 3.1 Project System

| | |
|---|---|
| **Responsibility** | Persistent project/product identity and shared world state |
| **Owns** | Requirements, repositories, architecture records, design references, conversations, missions, agents, decisions, evidence references, artifacts, runtimes, model config, build config, releases, deployment history |
| **Depends on** | Nothing (foundational) |
| **Exposes** | Project read/write API, Project Graph, Project Knowledge Graph |
| **Never does** | Execute code. Reason. Render UI. |

> **A Project is not a directory.** It is the durable representation of a software product.

Full specification: `docs/architecture/PROJECT_SYSTEM.md`

### 3.2 Agent Kernel

| | |
|---|---|
| **Responsibility** | Reasoning, planning, tool invocation, workflows, model selection, permissions, memory |
| **Owns** | Agent definitions, decision protocol, step budgets, approval gates, context assembly |
| **Depends on** | Project System, Intelligence Graph, Execution Engine, Model Platform |
| **Exposes** | Agent loop, tool registry, approval interface, memory API |
| **Never does** | Own the truth about the repository (that is the Intelligence Graph). Store its own copy of project state. |

Full specification: `docs/architecture/AGENT_KERNEL.md`

### 3.3 Intelligence Graph

| | |
|---|---|
| **Responsibility** | Repository, symbol, dependency, architecture, history and runtime knowledge |
| **Owns** | The repository index, symbol table, dependency graph, architectural fingerprint, change graph, retrieval/ranking |
| **Depends on** | Project System (scoping) |
| **Exposes** | Typed query API with relevance, reason and provenance on every result |
| **Never does** | Act as the Evidence Ledger (an index is not a ledger). Pretend regex extraction is semantic resolution. |

Full specification: `docs/architecture/INTELLIGENCE_GRAPH.md`

### 3.4 Vision Studio

| | |
|---|---|
| **Responsibility** | UI/UX design, design systems, prototypes, design↔code |
| **Owns** | The canonical `DesignDocument` model and its UI-IR |
| **Depends on** | Project System, Intelligence Graph (for code targets) |
| **Exposes** | Design model API, UI-IR, framework adapters, visual diff model |
| **Never does** | Use React DOM as the design model. Manipulate screenshots where primitives exist. |

Full specification: `docs/architecture/VISION_STUDIO.md`

### 3.5 Execution Engine

| | |
|---|---|
| **Responsibility** | Shell, browser, desktop, containers, Android, iOS, cloud |
| **Owns** | Worker lifecycle, sandboxes, resource limits, capability negotiation |
| **Depends on** | Project System, Permissions |
| **Exposes** | Uniform `ExecutionBackend` trait; every backend is interchangeable |
| **Never does** | Decide *whether* an action is permitted (that is Permissions). Interpret results (that is Proof). |

Full specification: `docs/architecture/EXECUTION_ENGINE.md`

### 3.6 Proof Engine

| | |
|---|---|
| **Responsibility** | Build/test/runtime/visual/security/evidence verification |
| **Owns** | The Proof Graph, acceptance-criteria evaluation, rung assignment |
| **Depends on** | Execution Engine, Evidence Ledger, Artifact Bus |
| **Exposes** | `verify(claim) → ProofResult`, proof graph queries |
| **Never does** | Mark something verified without a citation to an artifact. Accept "tests passed" without captured output. |

Full specification: `docs/architecture/PROOF_ENGINE.md`

### 3.7 Delivery Engine

| | |
|---|---|
| **Responsibility** | Git, PRs, CI/CD, packaging, deployment, app stores, releases |
| **Owns** | Release graph, artifact signing, distribution targets, release evidence |
| **Depends on** | Proof Engine (nothing ships unproven), Project System |
| **Exposes** | `plan_release`, `package`, `publish`, release evidence bundle |
| **Never does** | Publish an artifact whose required proofs are below threshold. |

Full specification: `docs/architecture/DELIVERY_ENGINE.md`

### 3.8 Computer-Use Engine

| | |
|---|---|
| **Responsibility** | Perceiving and driving the user's real desktop and arbitrary third-party applications, under permission, with evidence |
| **Owns** | Screen/window capture, accessibility-tree reads, element grounding, input synthesis (mouse/keyboard/clipboard), window management, per-application adapters, the Agent Flight Recorder |
| **Depends on** | Permissions (every step), Evidence Ledger, Artifact Bus, Project System (scoping), Execution Engine (worker lifecycle) |
| **Exposes** | The canonical loop `Observe → Ground → Decide → Permission → Act → Observe → Verify`; risk-level declarations; adapter registry; replayable session artifacts |
| **Never does** | Act without passing the Permission step. Report an action as successful without the Verify step re-observing its effect (A11). Carry a confidence value not derived from a measurement (§1.1). Own a browser DOM — that is the browser worker's job; this engine drives *applications*. |

> **Computer Use is a first-class engine, not a browser feature** (v2.1). Driving arbitrary GUI
> applications carries its own perception, grounding, permission, action, verification and
> evidence obligations. Folding it into the browser phase would hide its permission and evidence
> semantics at the architecture layer — which is exactly how a fabricated-confidence defect
> becomes a shipped trust defect, as it already did here.

**Risk levels.** Every action declares one, and each level maps to a required permission tier
and a required verification depth:

| Level | Scope | Example |
|---|---|---|
| **CU-0** | Observe only | Capture a screen region. No side effects. |
| **CU-1** | Navigate within a scoped application | Switch tabs, scroll, focus a window |
| **CU-2** | Interact, non-destructive | Click a form field, type into a scratch document |
| **CU-3** | Modify user data | Save a file, send a message, edit a record |
| **CU-4** | Irreversible or externally visible | Delete, purchase, publish, transmit |

**PROPOSED — non-functional skeleton present.** No real perception, no real action, no
permission gate, no verification, and no Flight Recorder exist. The tree contains
`crates/zylcode-core/src/computer_use/` (7 files, 1,763 lines, commit `29cc936`), wired into
`lib.rs` with a lazy `ZylCodeEngine::computer_use_system()` accessor, containing **31
simulation sites** — every perception returns fabricated data and every action is a
`tokio::time::sleep`. Its only reachable surface is a CLI `computer-use stats` command that
prints zeroed counters. Its tests assert `is_ok()` against a facade that cannot fail. It must be
**replaced, not extended**. Full finding: `DEEPSEEK_MASTER_PROMPT_V21.md`, and the catalogue
entry in §9.

Full specification: `docs/architecture/COMPUTER_USE_ENGINE.md`

---

## 4. Cross-Cutting Platforms

### 4.1 Extension Platform

The ABI by which capability is added **without modifying core**.

```
ZylCode Package
├── manifest        (identity, version, compatibility, permissions)
├── tools
├── mcp_servers
├── skills
├── agents
├── commands
├── hooks
├── model_providers
├── ui_panels
├── runtimes
├── templates
├── design_libraries
└── verification_providers
```

**Rule:** anything that would otherwise be a hard-coded integration must be expressible as a
package. Marketplace comes later. ABI first.

Full specification: `docs/architecture/EXTENSION_PLATFORM.md`

### 4.2 Model Platform

Capability-based routing, not a dropdown.

```
Task → capability requirements → candidate models → historical verified performance
     → privacy → cost → latency → context → selection
```

Routing decisions and their outcomes are recorded as evidence, so "model A is better at task
type T" becomes a **measured** claim.

**Status:** PARTIAL until routing is driven by measured results.

### 4.3 Artifact Bus

The canonical channel for anything a human or the Proof Engine needs to *inspect*.

Artifact kinds: `WEB_PREVIEW · COMPONENT_PREVIEW · DESIGN · DOCUMENT · DIAGRAM · IMAGE · DIFF ·
TEST_REPORT · BUILD_REPORT · SCREENSHOT · VIDEO · DEVICE_SCREEN · DATABASE_VIEW · API_RESPONSE`

Artifacts are **persistent Project objects**. An agent can create one. A runtime can update one.
The Proof Engine can cite one. The user can inspect one.

Full specification: `docs/architecture/ARTIFACT_SYSTEM.md`

---

## 5. Trust Foundation

### 5.1 Evidence Ledger

Append-only, hash-chained record of every claim, execution, observation and proof.

- An index is **not** a ledger. The Intelligence Graph index may be rebuilt; the ledger may not.
- Ledger entries are immutable. Corrections are new entries referencing the old.
- Every artifact cited by the Proof Engine resolves to a ledger entry.

### 5.2 Permissions

Risk-classified, fail-closed authorization.

- Absence of a grant is denial.
- Destructive and external actions require explicit approval.
- Every decision (allow *and* deny) is recorded with the rule that produced it.

### 5.3 Recovery

Durable, resumable execution.

- Execution state is checkpointed.
- Post-crash ambiguous states are **reconciled against external truth** (git, filesystem,
  remote APIs) — never guessed.
- Non-idempotent completed work is not re-executed.

### 5.4 Audit

Independent verification of capability claims. A capability is not accepted until audited by a
party that did not build it.

---

## 6. Data Ownership

**One writer per fact.** Duplicated truth is the most expensive architectural mistake available
to us.

| Fact | Owner | Everyone else |
|---|---|---|
| What the repository contains | Intelligence Graph | reads via query API |
| What a file says on disk | Filesystem | reads |
| What the Project is | Project System | reads via Project API |
| What was proven | Evidence Ledger | appends via ledger API |
| What a Mission requires | Project System (Mission) | reads |
| What a design is | Vision Studio | reads via design model |
| What a model did | Model Platform | appends evidence |
| What shipped | Delivery Engine | reads proofs |

**Corollary — the context-assembly rule:** the Agent Kernel must not maintain a parallel
notion of repository structure. It asks the Intelligence Graph. Today it does not, and that is
recorded as a defect in the Phase 2A audit.

---

## 7. Dependency Order

This order is **normative**. Building out of order is how the Project model, extension ABI,
design representation, or runtime contract ends up wrong at Stage 10.

```
Project System ──┬─► Agent Kernel ──► Mission Engine
                 │
                 ├─► Intelligence Graph ──► Repository Reasoning
                 │
                 ├─► Extension Platform
                 │
                 ├─► Artifact Bus ──► Live Preview ──► Browser Runtime
                 │
                 ├─► Vision Studio ──► Visual Intelligence
                 │
                 ├─► Permissions ══► Computer-Use Engine (v2.1)
                 │                    ▲ hard gate: Permissions precedes CU
                 │
                 └─► Execution Engine ──► Device Labs ──► Proof Engine v2 ──► Delivery
```

**Hard rules**

1. Project System precedes everything with persistent state.
2. Extension ABI precedes any second hard-coded integration of the same shape.
3. Artifact Bus precedes full browser automation (artifacts must have a home before they exist).
4. Design representation precedes the design canvas.
5. Proof Engine v2 precedes Delivery Engine — nothing ships unproven.
6. Multi-agent orchestration comes **after** shared world state exists.
7. **(v2.1) Permissions and the Evidence Ledger precede the Computer-Use Engine.** The `══►`
   edge is a hard gate, not a preference. A Computer-Use capability cannot exceed the rung of
   the Permission gate it depends on (A7, the rung ceiling). If Permissions is below R3 when
   Phase 7B is scheduled, **7B is blocked** — record the block; do not build around it.

---

## 8. Cross-Cutting Concerns

### 8.1 IPC

- Typed messages. No stringly-typed payloads across process boundaries.
- Every worker message carries `mission_id` and `step_id` for attribution.
- Workers are killable; the host must survive any worker death.

### 8.2 Security

- Sandboxed execution per worker.
- Secret exclusion is a security control and must be **verified against the real repository**,
  not a fixture.
- No implicit egress. Local-first indexing.
- Extension packages declare permissions; undeclared capability is denied.

### 8.3 Observability

- Structured logs, correlated by `project_id` / `mission_id` / `step_id`.
- Cost, latency and token accounting per model call.
- Every proof carries a citation chain to raw output.

### 8.4 Concurrency

- Scan/index work is parallelisable and must be (see the Phase 2A timing defect).
- The Evidence Ledger is append-only and therefore concurrency-safe by construction.
- Project state mutations are serialised per project.

---

## 9. Implementation Status — MANDATORY SECTION

> Per Constitution §9.3, this section must reflect **reality**, not intention.
> A system described here is not thereby built.

Rung definitions: `ZYLCODE_PROOF_GRAPH.md`.

| System | Rung | Status | Notes |
|---|---|---|---|
| **Trust foundation** — Evidence Ledger | **R3** | PARTIAL | Ledger, permissions gate, checkpoint recovery implemented (Phases 1A–1D). Generic verification execution remains partial. |
| **Agent Kernel** | **R3** | PARTIAL | Real tool runtime, iterative reasoning loop, model-driven decisions. **Live multi-provider commissioning outstanding.** |
| **Intelligence Graph** | **R2** | PARTIAL | Module implemented and unit-tested (79 tests). **Fails benchmark on independent re-run; not integrated into any product surface.** See `PHASE2A_INDEPENDENT_AUDIT.md`. |
| **Project System** | **R0** | PROPOSED | Does not exist. `Project` currently exists only as a UI-level notion. |
| **Mission Engine** | **R0** | PROPOSED | Does not exist. |
| **Model Platform** | **R1** | PARTIAL | Provider configuration and real HTTP dispatch exist. Capability-based routing does not. "Model Democracy" is **not** achieved. |
| **Extension Platform** | **R0** | PROPOSED | MCP bridge and a plugin marketplace module exist, but no stable package ABI. |
| **Artifact Bus** | **R0** | PROPOSED | No canonical artifact model. |
| **Live Preview Runtime** | **R0** | PROPOSED | Does not exist. |
| **Execution Engine** — shell/tools | **R3** | PARTIAL | Real filesystem/shell/git/search tools. |
| **Execution Engine** — browser | **R0** | PROPOSED | Not implemented. |
| **Execution Engine** — Android | **R0** | PROPOSED | Not implemented. |
| **Execution Engine** — macOS/iOS worker | **R0** | PROPOSED | Not implemented. |
| **Vision Studio** | **R0** | PROPOSED | Does not exist. |
| **Visual Intelligence** | **R0** | PROPOSED | Does not exist. |
| **Computer-Use Engine** | **R0** | **PROPOSED — non-functional skeleton present** | `crates/zylcode-core/src/computer_use/` (7 files, 1,763 lines, commit `29cc936`) is a **simulation facade**: 31 simulation sites; all capture returns `vec![0; …]`; all input is `tokio::time::sleep`; `recognize_text` returns a hardcoded literal; `detect_elements_internal` fabricates a "Submit" button with `confidence: 0.85` from image width alone. No permission gate, no verification, no Flight Recorder. Only reachable surface: CLI `computer-use stats`. Its tests assert `is_ok()` against a facade that cannot fail. **Replace, not extend.** |
| **Proof Engine v2** | **R0** | PROPOSED | A verification rung exists in the pipeline; the Proof Graph does not. |
| **Delivery Engine** | **R1** | PARTIAL | Release workflow configured (NSIS/DMG/DEB). **CI blocked externally.** Installer path has an incident history. |
| **Multi-Agent Engineering** | **R0** | PROPOSED | Does not exist. |
| **Marketplace** | **R0** | PROPOSED | Does not exist. |

### 9.1 Honest summary

ZylCode today is a **partial Agent Kernel on a partial Trust Foundation**, with a
**non-integrated Intelligence Graph**, a **non-functional Computer-Use skeleton that must not be
mistaken for capability**, and **nothing above it**.

That is a legitimate and useful state to be in. It is not the state that
`docs/capability-registry.json` describes, and correcting that discrepancy is a
precondition for everything that follows.

---

## 10. Architectural Risks

| Risk | Why it matters | Mitigation |
|---|---|---|
| **Index-as-ledger confusion** | Rebuilding the index would appear to rewrite history | Separate stores, separate APIs, documented in §5.1 |
| **Web-centric design model** | Permanently forecloses non-web targets | UI-IR mandatory before any canvas work (§A4, Constitution §8.3) |
| **Late extensibility** | Hard-coded integrations become unfixable | Extension ABI in Phase 5, before the integrations exist |
| **Cross-platform path handling** | Windows is the primary target and the current scanner is broken on it | Normalise paths at every boundary; test on the real platform |
| **Self-certification** | Proven to have already produced a false PASS | Independent audit is mandatory (Constitution §3.2) |
| **Metric scope creep** | "Files indexed" counted build output and was published as capability | Every metric names and scopes what it counts |
| **Fabricated grounding (v2.1)** | A facade returning invented element positions and confidence scores is indistinguishable from real perception at the type level, and would silently corrupt Computer-Use reliability work | Percept types must carry provenance and method alongside confidence (§1.1); facades must be labelled and tested as facades |
| **Simulated action (v2.1)** | Input synthesis that sleeps instead of acting passes every test that asserts `is_ok()` | Assert *effect*, not success. An action test must observe the change it claims to cause (A11) |
| **Permission-gate bypass (v2.1)** | A Computer-Use engine built before the Permission gate will hard-code its own allow logic, which is then never removed | Dependency hard rule 7 — Permissions precedes CU; 7B is blocked without it |

---

## 11. Relationship to the Program

This architecture is implemented by the 16-phase / 6-epoch program in
`docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md`, sequenced by
`docs/roadmap/ZYLCODE_ROADMAP_V2.md`.

**v2.1:** Phase 7 is decomposed in place into **7A Browser Runtime · 7B Computer Use
Foundation · 7C Computer Use Reliability · 7D Application Adapters**. Phases 8–16 are **not**
renumbered. The program remains **16 numbered phases**, with Phase 7 carrying four subphases.

Per-system specifications live in `docs/architecture/`.

Where this document and an implementation conflict, **investigate the discrepancy** —
do not silently choose one (Constitution §9.2).
