# ZylCode Master Execution Plan

> The governing program for building ZylCode.
> Companion to `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` and `ZYLCODE_ARCHITECTURE_V2.md`.

**Status:** GOVERNING
**Supersedes:** 14-stage sequential roadmap (`docs/governance/superseded/ZYLCODE_ROADMAP_V2.md`)
**Program shape:** **16 controlled phases, grouped into 6 epochs**

---

## 0. Why this plan is not a 14-stage feature roadmap

The previous plan treated the program as fourteen large sequential feature phases. That shape
has a specific failure mode:

> If phases are built as isolated blocks, we can reach Stage 10 before discovering that the
> **Project model**, the **extension ABI**, the **design representation**, or the **runtime
> contract** was wrong.

Those four are not features. They are contracts that everything else is written against.
Discovering a contract error at Stage 10 means rewriting every phase built on it.

This plan therefore distinguishes three kinds of work:

| Kind | Examples | Behaviour |
|---|---|---|
| **Foundations** | Project System, Extension ABI, design representation, runtime contract | Must be *correct* before dependents are built. Hard gates. |
| **Product surfaces** | Design canvas, Artifacts, Live Preview, Device Lab | Built once foundations exist. |
| **Continuous maturity** | Proof Engine, Model Platform, Intelligence Graph, Security | Never "done". Each phase raises their floor. |

A phase is not "finished forever". It is **closed at a rung** and reopened later at a higher rung.

---

## 1. North Star

**ZylCode is an evidence-first Software Creation Operating System.**

Its job is not merely to generate code. Its job is to take software from intent to a verified
deliverable:

> **Imagine → Specify → Design → Build → Run → See → Test → Repair → Verify → Ship**

The differentiator is the **closed loop**. See Constitution §1.3.

---

## 2. The Eight Core Systems

| # | System | Responsibility |
|---|---|---|
| 1 | **Project System** | Persistent project/workspace/product identity and shared state |
| 2 | **Agent Kernel** | Reasoning, planning, tools, workflows, models, permissions, memory |
| 3 | **Intelligence Graph** | Repository, symbols, dependencies, architecture, history, runtime knowledge |
| 4 | **Vision Studio** | UI/UX design, design systems, prototypes and design↔code |
| 5 | **Execution Engine** | Shell, browser, desktop, containers, Android, iOS, cloud |
| 6 | **Proof Engine** | Build/test/runtime/visual/security/evidence verification |
| 7 | **Delivery Engine** | Git, CI, packaging, deployment, app stores, releases |
| 8 | **Computer-Use Engine** | Real desktop/app perception and control, under permission, with replayable evidence (v2.1) |

Cross-cutting platforms: **Extension Platform · Model Platform · Artifact Bus**
Trust foundation: **Evidence Ledger · Permissions · Recovery · Audit**

---

## 3. Program Map

```
EPOCH I    TRUST AND UNDERSTANDING          ← we are here
           1A Real Tool Runtime              ✔ implemented
           1B Agent Execution Loop           ✔ implemented
           1C Model-Driven Autonomous Agent  ✔ implemented (live commissioning outstanding)
           1D Durable Memory, Evidence, Recovery ✔ implemented (generic verification partial)
           2A Repository Intelligence Foundation  ⚠ AUDIT FAILED — RE-OPENED
           2B Repository Reasoning & Impact Analysis  ⏸ BLOCKED on 2A

EPOCH II   THE PROJECT AND AGENT OS
           3A ZylCode Project System
           3B Mission Engine
           4  Model Platform / Model Democracy

EPOCH III  EXTENSIBILITY
           5  ZylCode Extension Platform

EPOCH IV   SEE WHAT YOU BUILD
           6A Artifact System
           6B Live Preview Runtime
           7A Browser Runtime
           7B Computer Use Foundation
           7C Computer Use Reliability
           7D Application Adapters

EPOCH V    VISION STUDIO
           8A Design Foundation
           8B Professional Design Canvas
           8C Design ↔ Code
           9  Visual Intelligence & Self-Repair

EPOCH VI   DEVICES, DELIVERY AND ECOSYSTEM
           10 Android Device Lab
           11 macOS/iOS Worker
           12 Proof Engine v2
           13 Delivery Engine
           14 Multi-Agent Engineering
           15 Marketplace and Ecosystem
           16 Public Commissioning
```

---

# EPOCH I — TRUST AND UNDERSTANDING

## Phase 1A — Real Tool Runtime ✔ IMPLEMENTED

Real filesystem, shell, git and search tools replacing mocks, with evidence recording.
**Rung: R3.**

## Phase 1B — Agent Execution Loop ✔ IMPLEMENTED

Iterative reasoning loop with tool execution, observation feedback, and bounded execution.
**Rung: R3.**

## Phase 1C — Model-Driven Autonomous Agent ✔ IMPLEMENTED / PARTIAL

`AgentDecision` protocol (Think, Plan, ToolCall, RequestApproval, Verify, Complete, Fail),
model-driven decisions, approval enforcement, completion verification.

**Outstanding:** live multi-provider commissioning. **Rung: R3 (partial).**

## Phase 1D — Durable Memory, Evidence & Recovery ✔ IMPLEMENTED / PARTIAL

Durable engineering memory, checkpoint-based crash recovery, ambiguous-execution
reconciliation against git truth, evidence ledger.

**Outstanding:** generic verification execution. **Rung: R3 (partial).**

---

## Phase 2A — Repository Intelligence Foundation ⚠ RE-OPENED

**Status: NOT ACCEPTED.** See `PHASE2A_INDEPENDENT_AUDIT.md`.

The module exists and is unit-tested, but:

- the headline "14,973 files indexed" measured `target/` build output, not the repository;
- the benchmark suite **fails** on independent re-execution;
- the subsystem has **zero** product integration.

**Rung: R2.**

**Exit criteria for re-acceptance**

1. Path exclusion fixed and verified against the **real** repository `.gitignore`.
2. All metrics re-baselined against a correctly-scoped index; corrections visible as corrections.
3. Benchmark deterministic, self-enforcing, and split into performance vs. quality suites.
4. Timing budget machine-independent (files/second), and met.
5. A committed reproduction transcript — command, environment, raw output, SHA.
6. Either integrated into `AgentLoop` context assembly + one CLI entry point, **or** the 11 GREEN
   capabilities downgraded in the registry.
7. Quality thresholds raised to a bar that a keyword search would fail (`Precision@10 ≥ 0.60`,
   Q5/Q6/Q10 non-zero).

---

## Phase 2B — Repository Reasoning & Impact Analysis ⏸ BLOCKED

**Blocked by:** Phase 2A re-acceptance. Building reasoning on a polluted index would produce
confidently wrong answers, which is worse than no answers.

**Purpose:** turn 2A's graph into actionable engineering intelligence.

**Deliverables**

- **Impact analysis** — given a change, what is affected (transitively), ranked.
- **Definition/reference traversal** — go-to-definition and find-all-references across the graph.
- **Test-to-code relationships** — which tests exercise which code, and the reverse.
- **Change risk** — a defensible risk signal per change, with the factors that produced it.
- **Architectural boundary detection** — identify and enforce intended module/layer boundaries.
- **Task → context retrieval** — given a task, assemble the minimum sufficient working set.
- **Incremental context assembly** — update context as the task evolves, without full re-scan.
- **Repository diagnostics** — health of the index itself, including staleness and coverage.
- **Benchmark suite** — known-answer questions with thresholds that fail a keyword baseline.
- **Agent integration** — reachable from `AgentLoop` and the CLI; this is not optional.

**2B is where repository intelligence starts helping autonomous engineering.**

**Exit criteria:** every deliverable reachable from a product surface, benchmark committed and
deterministic, independent audit at **R3 minimum**.

---

# EPOCH II — THE PROJECT AND AGENT OS

## Phase 3A — ZylCode Project System

**Why before multi-agent and Vision Studio:** a Project is the shared world state. Agents
without shared state communicate by dumping transcripts; Vision Studio without a Project has
nowhere to persist design intent.

**Creation flow**

```
New Project

Name
Repositories
Project type
Targets
Design system
Models
Runtime environments
Deployment targets
Secrets
Team
```

**Persisted**

requirements · repositories · architecture · design · conversations · missions · agents ·
decisions · evidence · artifacts · runtimes · model configuration · build configuration ·
releases · deployment history

**Requirements**

- Survives application restart.
- Survives machine migration (export/import, or versioned project store).
- Has a stable identity independent of filesystem path.

### Critical principle

> **A Project is not a directory.**
> It is ZylCode's persistent representation of a software product.

**Also delivered in this phase:** the **Project Knowledge Graph** — see §4.

**Specification:** `docs/architecture/PROJECT_SYSTEM.md`

---

## Phase 3B — Mission Engine

**This is the next major leap in autonomy.**

Instead of *"Ask AI"*, ZylCode presents **Mission**.

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

**States**

```
CREATED
UNDERSTANDING
PLANNING
AWAITING_APPROVAL
EXECUTING
VERIFYING
REPAIRING
BLOCKED
COMPLETE
FAILED
CANCELLED
```

**Requirements**

- Missions are **resumable** (builds directly on Phase 1D recovery).
- Missions are **inspectable** — state, steps, evidence, cost, elapsed time.
- Missions are **evidence-backed** — `COMPLETE` requires every acceptance criterion satisfied
  by a cited proof. Not a summary. A citation.

**Exit criteria:** a mission survives a process kill mid-`EXECUTING`, resumes, and completes
with a complete evidence bundle. Independently audited.

---

## Phase 4 — Model Platform / Model Democracy

**The system must not merely ask "which model do you want?"**

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

**Providers:** OpenAI · Anthropic · Google · DeepSeek · Z.AI/GLM · OpenRouter · local models.

**Hard requirement:** routing decisions and their outcomes are recorded as evidence, so
"model A is better at task type T" is a **measured** claim with a citation.

> **Model Democracy is PARTIAL until routing is based on measured results.**
> A dropdown isn't Model Democracy.

**Exit criteria:** for a given task class, the router can produce and justify a selection with
historical evidence, and the justification is visible in the UI.

---

# EPOCH III — EXTENSIBILITY

## Phase 5 — ZylCode Extension Platform

**Why this is earlier than previously planned:** if we wait until Phase 9, we will build half
the product with hard-coded integrations and later have to redesign them as extensions.

**Define the stable extension architecture early.**

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

This enables future Android, AWS, Unity, Unreal, Blender, FreeCAD, Kubernetes, database and
cloud integrations **without bloating core**.

**Manifest must declare:** identity, version, engine compatibility range, permissions,
contributed capability points, and entry points.

### Marketplace comes later.

> **Extension ABI first. Marketplace second.**

**Specification:** `docs/architecture/EXTENSION_PLATFORM.md`

**Exit criteria:** an out-of-tree package can contribute at least a tool, an agent and a UI
panel without any modification to core. Proven by a reference package in a separate repository.

---

# EPOCH IV — SEE WHAT YOU BUILD

## Phase 6A — Artifact System

**Why before full browser automation:** artifacts need a canonical architecture and a home
before they start being produced.

**Kinds**

```
WEB_PREVIEW      COMPONENT_PREVIEW   DESIGN           DOCUMENT
DIAGRAM          IMAGE               DIFF             TEST_REPORT
BUILD_REPORT     SCREENSHOT          VIDEO            DEVICE_SCREEN
DATABASE_VIEW    API_RESPONSE
```

**Properties**

- Artifacts are **persistent Project objects**.
- An **agent** can create one.
- A **runtime** can update one.
- The **user** can inspect one.
- The **Proof Engine** can cite one.

**Specification:** `docs/architecture/ARTIFACT_SYSTEM.md`

---

## Phase 6B — Live Preview Runtime

```
Code
 ↓
Build
 ↓
Dev server
 ↓
Artifact
 ↓
Live preview
```

**Preview surface must include:** console · network · errors · responsive sizes · accessibility ·
reload state.

This is the Claude-artifact-style capability — but connected to **actual project execution**,
not a sandboxed snippet.

**Exit criteria:** editing a file causes the preview to update; the preview reflects the real
dev server, real console, real network.

---

## Phase 7 — Eyes and Hands

> **Phase 7 carries four subphases, 7A–7D — a decomposition in place, not a renumbering.**
> Phases 8–16 are untouched; the program remains **16 numbered phases**. Browser automation and
> arbitrary-GUI automation share a goal but not a trust profile: desktop control reaches the
> user's real machine, so its Permission and evidence obligations must be visible at the
> architecture layer rather than folded into the browser phase. Each subphase is independently
> gated — 7B is not accepted because 7A passed.

### Phase 7A — Browser Runtime

**Give ZylCode controlled eyes and hands, inside a browser.**

**Capabilities:** launch browser · navigate · click · type · inspect DOM · inspect console ·
inspect network · screenshot · record interaction · run test workflows.

**Then the agent can do:**

```
Implement
 ↓
Launch
 ↓
Observe
 ↓
Interact
 ↓
Detect problem
 ↓
Repair
 ↓
Reload
 ↓
Retest
```

**Blocks:** 7B, 9, 12.

### Phase 7B — Computer Use Foundation

**Give ZylCode eyes and hands on the user's real desktop — under permission, with evidence.**

**Capabilities:** screen/window capture · window enumeration · accessibility-tree reads ·
element grounding with **measured** confidence · mouse/keyboard/clipboard synthesis · window
management · risk levels CU-0…CU-4 · the canonical loop
`Observe → Ground → Decide → Permission → Act → Observe → Verify` · Agent Flight Recorder.

**Hard prerequisite:** Permissions and the Evidence Ledger must be at **R3**. This is
Architecture §7 hard rule 7 and the rung ceiling (A7). If they are below R3, **7B is blocked** —
record the block and wait; do not build a local allow-list in the meantime.

**Existing skeleton:** `crates/zylcode-core/src/computer_use/` (commit `29cc936`) is a
**simulation facade** — 31 simulation sites, all-zero capture, `sleep`-based input, hardcoded OCR,
fabricated confidences. **Replace, do not extend.** Its tests assert `is_ok()` against a facade
that cannot fail and must be deleted rather than adapted.

**Blocks:** 7C, 7D.

### Phase 7C — Computer Use Reliability

**Make desktop control trustworthy rather than merely functional.**

**Capabilities:** grounding accuracy measured against a labelled set (precision/recall) ·
retry and verification semantics · UI-drift detection and recovery · idempotency for interrupted
action sequences · Flight Recorder as a shipping, replayable, citable artifact.

**Blocks:** 7D.

### Phase 7D — Application Adapters

**Per-application integrations built on the 7B/7C foundation.**

**Capabilities:** adapter registry · target-application adapters, each declaring its **risk
level**, **grounding method**, and **verification strategy**. An adapter that cannot state how it
verifies its actions is not accepted.

**Blocks:** 9, 12.

**That loop is crucial.** It is the first time ZylCode closes the loop without a human in it.

**Specification:** `docs/architecture/EXECUTION_ENGINE.md`

---

# EPOCH V — VISION STUDIO

## Phase 8A — Design Foundation

**Do not start by attempting all of Figma.** Build the canonical design representation first.

```
DesignDocument   Page        Frame       Component
Instance         Variant     Text        Vector
Image            Layout      Constraint  Interaction
Token            Variable    Style
```

### The key architectural decision

> **Design must be machine-readable and agent-addressable.**

The AI should not manipulate screenshots when it can manipulate structured design primitives.

**Specification:** `docs/architecture/VISION_STUDIO.md`

---

## Phase 8B — Professional Design Canvas

**Then build:** selection · move · resize · zoom · pan · alignment · distribution · grids ·
constraints · auto layout · components · variants · typography · design tokens · responsive
frames · prototyping · interactions.

At that point we approach the workflow characteristics of Figma/Penpot.

---

## Phase 8C — Design ↔ Code

**Potentially one of ZylCode's defining technologies.**

```
DESIGN MODEL
     ↕
UI INTERMEDIATE REPRESENTATION
     ↕
FRAMEWORK ADAPTER
```

**Adapters (eventually):** React · Next.js · HTML/CSS · Flutter · Jetpack Compose · SwiftUI ·
React Native.

> **Do not make React DOM the design model.**
> Otherwise Vision Studio becomes permanently web-centric.

**Exit criteria:** round-trip fidelity for at least two structurally different targets (e.g.
React and Jetpack Compose) with measured fidelity, not "looks right".

---

## Phase 9 — Visual Intelligence & Self-Repair

**Combine Vision Studio + Runtime.**

```
EXPECTED DESIGN
       ↓
RENDERED SOFTWARE
       ↓
SCREENSHOT
       ↓
VISUAL ANALYSIS
       ↓
DIFFERENCE MODEL
       ↓
REPAIR PLAN
       ↓
CODE/DESIGN CHANGE
       ↓
RENDER AGAIN
```

**This is where ZylCode begins genuinely seeing its own work.**

**Exit criteria:** a seeded visual defect (injected spacing/colour/layout error) is detected and
repaired without human intervention, with before/after artifacts and the diff as evidence.

---

# EPOCH VI — DEVICES, DELIVERY AND ECOSYSTEM

## Phase 10 — Android Device Lab

**Capabilities:** Android Studio/SDK · emulator · ADB · physical devices · logcat · screenshots ·
install APK · launch · tap/type/swipe · inspect crash · Gradle build/test · release artifact.

> **The agent must be able to build and actually use the Android application.**

Not "compile successfully". *Use it.*

---

## Phase 11 — macOS/iOS Worker

**Windows cannot provide native iOS simulator/Xcode execution.**

```
ZylCode Windows/Linux
        │
        │ secure worker protocol
        ↓
ZylCode Mac Worker
        │
        ├── Xcode
        ├── Simulator
        ├── xcodebuild
        ├── signing
        ├── devices
        └── TestFlight
```

**Mac worker becomes another Execution Engine backend** — interchangeable behind the same
`ExecutionBackend` contract, not a special case.

**Requirements:** authenticated transport, capability advertisement, job isolation, artifact
return, and no implicit trust of the remote worker.

---

## Phase 12 — Proof Engine v2

**Unify all verification.**

**Proof may include:** compile · lint · unit tests · integration tests · browser tests · visual
tests · accessibility · Android runtime · iOS runtime · security · packaging · deployment.

**This is where the canonical Proof Graph becomes formal.** See `ZYLCODE_PROOF_GRAPH.md`.

The proof ladder:

```
R0 — CLAIMED
R1 — OBSERVED
R2 — EXECUTED
R3 — VERIFIED
R4 — REPRODUCIBLE
R5 — COMMISSIONED
```

> This definition is better suited to ZylCode than a build-centric ladder because it applies to
> **any capability**, not merely source code.

**Exit criteria:** every capability in the registry carries a rung that is computed from
evidence, not asserted by hand.

---

## Phase 13 — Delivery Engine

**ZylCode should understand how to finish the job.**

Git · PRs · CI/CD · Windows installers · macOS bundles · Linux packages · Docker · web
deployment · Android AAB/APK · Play Store · iOS IPA/TestFlight/App Store · release evidence.

> The installer incident is exactly why Delivery deserves its own engine rather than being
> scattered shell scripts.

**Hard rule:** the Delivery Engine cannot publish an artifact whose required proofs are below
threshold. Nothing ships unproven.

**Specification:** `docs/architecture/DELIVERY_ENGINE.md`

---

## Phase 14 — Multi-Agent Engineering

**Only now do we want sophisticated multi-agent behaviour.**

**Roles:** Planner · Architect · Developer · Designer · Tester · Reviewer · Debugger · Security ·
Release

### Shared world state is critical

> Agents shouldn't talk to one another by dumping giant chat transcripts.

They share:

```
Project Graph
Repository Graph
Mission state
Artifacts
Evidence
Decisions
```

**Exit criteria:** two agents coordinate on a task whose handoff is a **graph mutation**, not a
message. The handoff is inspectable and replayable.

---

## Phase 15 — Marketplace and Ecosystem

**Once the extension ABI is stable:**

```
ZylCode Marketplace

Plugins · Skills · Agents · MCP · Models · Themes
Design Systems · Templates · Runtimes · Deployment Providers
```

**Requirements:** signed packages · permissions · versioning · compatibility · ratings ·
publisher identity · update mechanism · security review.

---

## Phase 16 — Public Commissioning

**Only then the major public push.** But useful versions release much earlier — this phase is
about the **big demonstration and ecosystem push**, not the first public binary.

**Create the ZylCode Engineering Benchmark.**

**Measures:** task completion · build success · test success · repair success · repository
retrieval · visual fidelity · token consumption · cost · latency · human intervention.

**Then publish reproducible evidence.** Reproducible is the operative word: the benchmark must
be runnable by a third party against a tagged release and produce comparable numbers.

---

## 4. Project Knowledge Graph (delivered in Phase 3A)

A capability between ordinary memory and repository intelligence.

Every Project develops an evolving knowledge graph:

```
"Authentication uses JWT"
        │
        ├── decided in ADR-004
        ├── implemented by auth.rs
        ├── tested by auth_test.rs
        ├── introduced commit abc123
        └── verified run E-9237
```

It combines four sources that are normally separate:

> **what the code says + what humans decided + what agents learned + what runtime evidence proved**

This is a long-term differentiator and is the natural extension of the `Provenance` and
`ContextResult.reason` modelling already present in the Intelligence Graph.

**Specification:** `docs/architecture/PROJECT_KNOWLEDGE_GRAPH.md`

---

## 5. Gate Model

No phase begins before its dependencies are **ACCEPTED**, not merely "reported complete".

| Gate | Meaning | Who decides |
|---|---|---|
| **G0 Design** | Spec exists, marked PROPOSED/implemented accurately | Product owner |
| **G1 Implementation** | Code committed, reachable from a product surface | Builder |
| **G2 Self-evidence** | Reproduction block present with raw output | Builder |
| **G3 Independent audit** | Third party reproduces and inspects | Auditor |
| **G4 Acceptance** | ACCEPTED / ACCEPTED WITH CONDITIONS / NOT ACCEPTED | Product owner |

**A phase that fails G3 is RE-OPENED.** Phase 2A is currently at G3 → **NOT ACCEPTED**.

---

## 6. Continuous Maturity Tracks

These never "close". Each phase must raise their floor.

| Track | Raised by |
|---|---|
| **Proof Engine** | 1D, 2A, 6B, 7, 9, 10, 12, 13 |
| **Intelligence Graph** | 2A, 2B, 3A, 8C, 14 |
| **Model Platform** | 1C, 4, 14, 16 |
| **Security** | every phase; extensions add attack surface |
| **Performance** | 2A (scan), 6B (preview), 10 (device), 16 (benchmark) |
| **Evidence integrity** | every phase; this is the product's core promise |

---

## 7. Cross-Cutting Rules for Every Phase

1. Every agent prompt opens with the mandatory preamble (Constitution §9.2).
2. Every phase has a written specification marked PROPOSED until implemented.
3. Every completion report carries a reproduction block.
4. No capability is marked GREEN on unit tests alone.
5. Builders do not certify their own work.
6. Where implementation and documentation disagree, **investigate** — do not silently choose.
7. Every phase raises the Proof Engine's floor or explicitly states why not.

---

## 8. Immediate Actions

| # | Action | Owner | Status |
|---|---|---|---|
| 1 | Audit `0ecea8e` | Independent auditor | ✔ **DONE — NOT ACCEPTED** |
| 2 | Publish governance package (Constitution, Architecture, Plan, Capability Model, Proof Graph, Roadmap, 9 architecture specs) | Product owner | ✔ **DONE** |
| 3 | Supersede the six-engine / 14-stage drafts | Product owner | ✔ **DONE** |
| 4 | Issue remediation order to DeepSeek for Phase 2A P0 items | Product owner | **NEXT** |
| 5 | Re-audit Phase 2A | Independent auditor | blocked on #4 |
| 6 | Only then: authorize Phase 2B | Product owner | blocked on #5 |

**Phase 2B does not begin until Phase 2A is re-accepted.**
