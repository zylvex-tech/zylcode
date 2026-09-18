# ZylCode Roadmap v2.0

> 16 phases · 6 epochs · gates · dependencies · deliverables · benchmarks.
> Governing specification for all implementation agents.

**Status:** GOVERNING
**Supersedes:** 14-stage sequential roadmap (`docs/governance/superseded/ZYLCODE_ROADMAP_V2.md`)
**Companion to:** `docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md`

---

## 0. How to Read This Document

Every phase has:

| Field | Meaning |
|---|---|
| **Objective** | What the phase achieves |
| **Input** | What must exist before it begins |
| **Deliverables** | What must ship |
| **Entry points** | Named surfaces a user or agent can invoke (R3 requirement) |
| **Gates** | Verification criteria |
| **Benchmark** | Measurable acceptance criteria |
| **Blocks** | What cannot begin until this is ACCEPTED |
| **Rung target** | The proof rung required at acceptance |

**Sequencing rule:** no phase begins before its dependencies are **ACCEPTED** — not merely
reported complete.

**Status vocabulary:** `ACCEPTED` · `ACCEPTED WITH CONDITIONS` · `NOT ACCEPTED (re-opened)` ·
`BLOCKED` · `NOT STARTED` · `IN PROGRESS`

---

## 1. Dependency Graph

```
                          ┌─────────────────────────────┐
                          │ EPOCH I — TRUST & UNDERSTAND │
                          └──────────────┬──────────────┘
                                         │
        1A ─► 1B ─► 1C ─► 1D ─► 2A ─► 2B
                                        ▲
                              (2A NOT ACCEPTED — RE-OPENED)
                                        │
                          ┌─────────────┴───────────────┐
                          │ EPOCH II — PROJECT & AGENT OS│
                          └──────────────┬──────────────┘
                                         │
                     3A (Project) ─► 3B (Mission) ─► 4 (Model Platform)
                          │
                          ├──────────────────────────────┐
                          ▼                              ▼
          ┌─────────────────────────────┐   ┌────────────────────────────┐
          │ EPOCH III — EXTENSIBILITY   │   │ EPOCH IV — SEE WHAT YOU BUILD│
          │        5 (Extension ABI)    │   │  6A Artifacts ─► 6B Preview │
          └──────────────┬──────────────┘   │     ─► 7A Browser ─► 7B CU  │
                         │                  │     ─► 7C Reliability ─► 7D │
                         │                  └──────────────┬─────────────┘
                         │                                 │
                         │                  ┌──────────────┴─────────────┐
                         │                  │ EPOCH V — VISION STUDIO    │
                         │                  │ 8A ─► 8B ─► 8C ─► 9        │
                         │                  └──────────────┬─────────────┘
                         │                                 │
                         └──────────────┬──────────────────┘
                                        ▼
                          ┌─────────────────────────────┐
                          │ EPOCH VI — DEVICES & DELIVERY│
                          └──────────────┬──────────────┘
                                         │
       10 (Android) ─┐
       11 (Mac/iOS) ─┼─► 12 (Proof v2) ─► 13 (Delivery) ─► 14 (Multi-Agent)
                     │                                            │
                     └────────────────────────────────────────────┘
                                                                  ▼
                                              15 (Marketplace) ─► 16 (Public)
```

**Hard ordering rules**

1. `3A` (Project System) precedes `3B`, `4`, `5`, `6A`, `8A` — everything with persistent state.
2. `5` (Extension ABI) precedes `6A`, `7`, `10`, `11`, `13` — anything that would otherwise be a
   hard-coded integration.
3. `6A` (Artifacts) precedes `6B`, `7`, `9` — artifacts need a home before they exist.
4. `8A` (Design representation) precedes `8B`, `8C` — model before canvas.
5. `12` (Proof Engine v2) precedes `13` (Delivery) — nothing ships unproven.
6. `3A` + `3B` precede `14` (Multi-Agent) — no orchestration without shared world state.
7. `5` precedes `15` — no marketplace without a stable ABI.

---

## 2. Program Status Board

| Phase | Title | Status | Rung | Blocks |
|---|---|---|---|---|
| 1A | Real Tool Runtime | ✅ ACCEPTED | R3 | 1B |
| 1B | Agent Execution Loop | ✅ ACCEPTED | R3 | 1C |
| 1C | Model-Driven Autonomous Agent | ✅ ACCEPTED WITH CONDITIONS | R3 | 1D |
| 1D | Durable Memory, Evidence & Recovery | ✅ ACCEPTED WITH CONDITIONS | R3 | 2A |
| **2A** | **Repository Intelligence Foundation** | ❌ **NOT ACCEPTED — RE-OPENED** | R2 | **2B** |
| **2B** | **Repository Reasoning & Impact Analysis** | 🚫 **BLOCKED on 2A** | — | 3A |
| 3A | ZylCode Project System | ⏸ NOT STARTED | — | 3B, 4, 5, 6A, 8A |
| 3B | Mission Engine | ⏸ NOT STARTED | — | 4, 14 |
| 4 | Model Platform / Model Democracy | ⏸ NOT STARTED | — | 14 |
| 5 | Extension Platform | ⏸ NOT STARTED | — | 6A, 7A, 10, 11, 13, 15 |
| 6A | Artifact System | ⏸ NOT STARTED | — | 6B, 7A, 9 |
| 6B | Live Preview Runtime | ⏸ NOT STARTED | — | 7A |
| 7A | Browser Runtime | ⏸ NOT STARTED | — | 7B, 9, 12 |
| 7B | Computer Use Foundation | ⏸ NOT STARTED | — | 7C, 7D |
| 7C | Computer Use Reliability | ⏸ NOT STARTED | — | 7D |
| 7D | Application Adapters | ⏸ NOT STARTED | — | 9, 12 |
| 8A | Design Foundation | ⏸ NOT STARTED | — | 8B, 8C |
| 8B | Professional Design Canvas | ⏸ NOT STARTED | — | 9 |
| 8C | Design ↔ Code | ⏸ NOT STARTED | — | 9 |
| 9 | Visual Intelligence & Self-Repair | ⏸ NOT STARTED | — | 12, 16 |
| 10 | Android Device Lab | ⏸ NOT STARTED | — | 12, 13, 16 |
| 11 | macOS/iOS Worker | ⏸ NOT STARTED | — | 12, 13 |
| 12 | Proof Engine v2 | ⏸ NOT STARTED | — | 13, 16 |
| 13 | Delivery Engine | ⏸ NOT STARTED | — | 16 |
| 14 | Multi-Agent Engineering | ⏸ NOT STARTED | — | 15, 16 |
| 15 | Marketplace and Ecosystem | ⏸ NOT STARTED | — | 16 |
| 16 | Public Commissioning | ⏸ NOT STARTED | — | — |

---

# EPOCH I — TRUST AND UNDERSTANDING

## Phase 1A — Real Tool Runtime ✅

**Objective** — Replace mocked tools with real execution and evidence recording.
**Deliverables** — `RealTool` trait; `FileSystemTool`, `ShellTool`, `GitTool`, `SearchTool`.
**Entry points** — agent tool registry.
**Benchmark** — tool invocation produces real filesystem/shell effects with a ledger entry.
**Rung target** — R3. **Blocks** — 1B.

## Phase 1B — Agent Execution Loop ✅

**Objective** — Iterative reasoning loop with tool execution and observation feedback.
**Deliverables** — loop, observation feedback, step limits, timeouts, cancellation.
**Benchmark** — `test_observation_loop`, `test_repair_loop` pass; loop terminates on budget.
**Rung target** — R3. **Blocks** — 1C.

## Phase 1C — Model-Driven Autonomous Agent ✅ (conditions)

**Objective** — Model-driven decisions via a formal decision protocol.
**Deliverables** — `AgentDecision` enum; approval enforcement; completion verification requiring evidence.
**Conditions** — live multi-provider commissioning outstanding.
**Benchmark** — agent completes a task end-to-end against a live provider with ledger evidence.
**Rung target** — R3. **Blocks** — 1D.

## Phase 1D — Durable Memory, Evidence & Recovery ✅ (conditions)

**Objective** — Durable memory, crash recovery, evidence ledger, permission gate.
**Deliverables** — `LedgerStore` (+ SQLite/memory impls), `SessionCheckpoint`, crash-window
recovery, ambiguous-execution reconciliation against git truth.
**Conditions** — generic verification execution remains partial.
**Benchmark** — kill mid-execution; resume; no non-idempotent work repeated; ambiguous state
reconciled against git.
**Rung target** — R3. **Blocks** — 2A.

---

## Phase 2A — Repository Intelligence Foundation ❌ RE-OPENED

**Objective** — A structured, queryable model of a repository.
**Status** — **NOT ACCEPTED.** See `PHASE2A_INDEPENDENT_AUDIT.md`.
**Current rung** — **R2.**

### Why it was rejected

| Finding | Evidence |
|---|---|
| Headline metric measured build output | 14,900 of 15,064 indexed files under `target/`; repository is 254 files |
| Benchmark fails on re-execution | `Scan should complete in < 120s, took 276.1s` |
| Zero product integration | `grep intelligence::` outside the module → no output |
| "Acceptance Demonstration" is an expectation | Section headed "Expected Output" |
| Workspace tests not green | `225 passed; 1 failed` |
| Report self-inconsistent | committed with `Commit SHA: (Pending)` |

### Required remediation

**P0**
1. Path exclusion fixed — separator-agnostic, and verified against the **real** `.gitignore`.
2. All metrics re-baselined; corrections **visible as corrections**.
3. Benchmark deterministic, self-enforcing, split into performance vs. quality.
4. Timing budget machine-independent (files/second) and met.
5. Committed reproduction transcript — command, environment, raw output, SHA.
6. Registry corrected (`repository_scanner` evidence string, 12 capabilities downgraded).

**P1**
7. Integrate into `AgentLoop` context assembly + one CLI entry point — **or** downgrade the 11
   GREEN capabilities to PARTIAL/R2.
8. Raise quality thresholds to a bar a keyword search would fail: `Precision@10 ≥ 0.60`, and
   Q5/Q6/Q10 non-zero.
9. Resolve the failing `router::tests::synthetic_offline_dispatch_returns_parseable_payload`.
10. Commit or discard the 53 modified files.

**P2**
11. Every future report carries a reproduction block.

### Exit gates

| Gate | Criterion |
|---|---|
| G1 | Code committed, all P0/P1 items addressed |
| G2 | Reproduction block present with raw output |
| G3 | Auditor reproduces benchmark green **and** verifies index scope **and** invokes an entry point |
| G4 | ACCEPTED |

**Rung target at acceptance** — **R3.** **Blocks** — 2B.

---

## Phase 2B — Repository Reasoning & Impact Analysis 🚫 BLOCKED

**Blocked by** — 2A re-acceptance. Reasoning over a polluted index yields confidently wrong
answers, which is worse than no answers.

**Objective** — Turn the graph into actionable engineering intelligence.

**Deliverables**

| Deliverable | Acceptance |
|---|---|
| Impact analysis | given a symbol/file, affected set returned transitively, ranked, with reasons |
| Definition/reference traversal | go-to-definition and find-references across the graph |
| Test-to-code relationships | which tests exercise which code, both directions |
| Change risk | a risk signal per change, with the factors that produced it, not just a number |
| Architectural boundary detection | intended module/layer boundaries identified and violations flagged |
| Task→context retrieval | minimum sufficient working set for a task, token-aware |
| Incremental context assembly | context updated as a task evolves, without full re-scan |
| Repository diagnostics | index health: coverage, staleness, unsupported files |
| Benchmark suite | thresholds that a keyword baseline fails |
| **Agent integration** | reachable from `AgentLoop` **and** the CLI |

**Entry points** — `zylcode repo context <task>` · `zylcode repo impact <symbol>` ·
`AgentLoop::assemble_context`.

**Benchmark** — `Precision@10 ≥ 0.60` and `Recall@10 ≥ 0.60` on ≥ 20 known-answer questions;
Q5/Q6/Q10 non-zero; index scoped to repository source with the scope asserted in a test.

**Rung target** — **R3** minimum, R4 preferred. **Blocks** — 3A.

---

# EPOCH II — THE PROJECT AND AGENT OS

## Phase 3A — ZylCode Project System

**Objective** — Persistent project/product identity and shared world state.
**Input** — 2B ACCEPTED.

**Deliverables**

- Project creation flow: name, repositories, project type, targets, design system, models,
  runtime environments, deployment targets, secrets, team.
- Persistent store: requirements, repositories, architecture, design, conversations, missions,
  agents, decisions, evidence, artifacts, runtimes, model config, build config, releases,
  deployment history.
- Survives restart. Survives machine migration. Stable identity independent of filesystem path.
- **Project Knowledge Graph** — see `docs/architecture/PROJECT_KNOWLEDGE_GRAPH.md`.

**Entry points** — `zylcode project create|open|export|import` · UI Project surface.

**Benchmark** — create a project; restart the app; project is intact; export on machine A,
import on machine B, graph is intact.

**Critical principle** — *A Project is not a directory.*

**Rung target** — R3. **Blocks** — 3B, 4, 5, 6A, 8A.

---

## Phase 3B — Mission Engine

**Objective** — Replace "Ask AI" with **Mission**.
**Input** — 3A ACCEPTED.

**Deliverables**

- Mission with objective + explicit acceptance criteria.
- States: `CREATED · UNDERSTANDING · PLANNING · AWAITING_APPROVAL · EXECUTING · VERIFYING ·
  REPAIRING · BLOCKED · COMPLETE · FAILED · CANCELLED`.
- Resumable, inspectable, evidence-backed.
- `COMPLETE` requires every criterion satisfied by a **cited** proof.

**Entry points** — `zylcode mission create|status|resume` · UI Mission surface.

**Benchmark** — kill the process mid-`EXECUTING`; on restart the mission resumes and completes
with a full evidence bundle. A mission with one unsatisfied criterion **cannot** report COMPLETE.

**Rung target** — R3. **Blocks** — 4, 14.

---

## Phase 4 — Model Platform / Model Democracy

**Objective** — Capability-based routing across providers and local models.
**Input** — 3A, 3B ACCEPTED.

**Deliverables** — capability requirements model · candidate discovery · historical verified
performance store · privacy/cost/latency/context filters · recorded routing decisions with outcomes.

**Providers** — OpenAI · Anthropic · Google · DeepSeek · Z.AI/GLM · OpenRouter · local.

**Entry points** — routing visible and justifiable in UI · `zylcode models route --task <class>`.

**Benchmark** — for a task class, the router produces a selection **and a justification citing
measured history**. A selection without measurement is a dropdown, not Model Democracy.

**Rung target** — R3. **Blocks** — 14.

---

# EPOCH III — EXTENSIBILITY

## Phase 5 — ZylCode Extension Platform

**Objective** — A stable ABI so capability is added without modifying core.
**Input** — 3A ACCEPTED.

**Deliverables** — package manifest (identity, version, engine compatibility, permissions,
contribution points, entry points); contribution types: tools, MCP servers, skills, agents,
commands, hooks, model providers, UI panels, runtimes, templates, design libraries, verification
providers; permission enforcement; version compatibility checks.

**Entry points** — `zylcode ext install|list|inspect` · UI Extensions surface.

**Benchmark** — a **reference package in a separate repository** contributes a tool, an agent and
a UI panel with **zero** core modification.

**Hard rule** — *Extension ABI first. Marketplace second.*

**Rung target** — R3. **Blocks** — 6A, 7, 10, 11, 13, 15.

---

# EPOCH IV — SEE WHAT YOU BUILD

## Phase 6A — Artifact System

**Objective** — A canonical home for everything inspectable.
**Input** — 3A, 5 ACCEPTED.

**Deliverables** — artifact model; kinds (`WEB_PREVIEW · COMPONENT_PREVIEW · DESIGN · DOCUMENT ·
DIAGRAM · IMAGE · DIFF · TEST_REPORT · BUILD_REPORT · SCREENSHOT · VIDEO · DEVICE_SCREEN ·
DATABASE_VIEW · API_RESPONSE`); persistence as Project objects; agent-create, runtime-update,
user-inspect, proof-cite.

**Entry points** — Artifacts panel · `zylcode artifact list|open`.

**Benchmark** — an agent creates an artifact; a runtime updates it; the Proof Engine cites it; the
citation resolves through the ledger to raw output.

**Rung target** — R3. **Blocks** — 6B, 7, 9.

---

## Phase 6B — Live Preview Runtime

**Objective** — Code → build → dev server → artifact → live preview.
**Input** — 6A ACCEPTED.

**Deliverables** — preview with console, network, errors, responsive sizes, accessibility,
reload state, connected to the **real** dev server.

**Entry points** — Preview panel.

**Benchmark** — edit a file; the preview updates; console and network reflect the real server.

**Rung target** — R3. **Blocks** — 7.

---

## Phase 7 — Eyes and Hands
> **Phase 7 carries four subphases, 7A–7D.** This is a **decomposition in place**, not a
> renumbering: phases 8–16 are untouched, and the program remains **16 numbered phases**.
> Rationale: browser automation and arbitrary-GUI automation share a goal but not a trust
> profile. Keeping them in one phase would hide the Permission and evidence obligations of
> desktop control at the architecture layer — which is how a fabricated-grounding defect ships.

### Phase 7A — Browser Runtime

**Objective** — Controlled eyes and hands, inside a browser.
**Input** — 5, 6A, 6B ACCEPTED.

**Deliverables** — launch · navigate · click · type · DOM inspect · console inspect · network
inspect · screenshot · record interaction · run test workflows.

**Entry points** — Browser panel · `zylcode browser run <workflow>`.

**Benchmark** — the full loop runs without human intervention:
`implement → launch → observe → interact → detect problem → repair → reload → retest`.
Transcript captures each arc with real console/network output.

**Rung target** — R3. **Blocks** — 7B, 9, 12.

---

### Phase 7B — Computer Use Foundation

**Objective** — Perception and action primitives for the user's real desktop, behind a permission
gate, with every step recorded.
**Input** — 7A ACCEPTED; **Permissions and Evidence Ledger at R3** (hard prerequisite — see note).

**Deliverables** —
- **Perception** — screen and window capture, window enumeration, accessibility-tree reads.
  Every percept is an artifact with provenance and a timestamp. **No fabricated confidence**
  (Architecture §1.1).
- **Grounding** — mapping percepts to actionable targets with **measured** confidence. A score
  not derived from a measurement is a defect, not a placeholder.
- **Action** — mouse, keyboard, clipboard, window management. Every action is recorded *before*
  it is performed.
- **The canonical loop** — `Observe → Ground → Decide → Permission → Act → Observe → Verify`,
  in that order. No action executes without the Permission step; no action is reported
  successful without the Verify step producing a percept that confirms it (Architecture A11).
- **Risk levels CU-0…CU-4**, each mapped to a required permission tier and verification depth.
- **Agent Flight Recorder** — a replayable artifact sequence per session.

**Entry points** — `zylcode cu capture|click|type|session replay` · Computer Use panel.

**Benchmark** — capture the real screen and assert it is not a constant buffer; click a known
target in a controlled fixture application and assert the **effect** (not `is_ok()`); a
denied permission produces a recorded refusal and **no** action.

**Rung target** — R3. **Blocks** — 7C, 7D.

> **Hard prerequisite.** 7B depends on Permissions and the Evidence Ledger being at R3
> (Architecture §7 hard rule 7, and A7 the rung ceiling). If they are below R3 when 7B is
> scheduled, **7B is blocked** — record the block and wait. Do not build a local allow-list
> "temporarily"; it will never be removed, and it will bypass the audited gate.
>
> **Existing skeleton.** `crates/zylcode-core/src/computer_use/` (commit `29cc936`) is a
> **simulation facade** — 31 simulation sites, all-zero capture buffers, `sleep`-based input,
> hardcoded OCR output, fabricated `confidence` values. It is **not a starting point**. It must
> be **replaced**, and its tests (which assert `is_ok()` against a facade that cannot fail) must
> be **deleted**, not adapted. See `docs/governance/ZYLCODE_ARCHITECTURE_V2.md` §9.

---

### Phase 7C — Computer Use Reliability

**Objective** — Make desktop control trustworthy rather than merely functional.
**Input** — 7B ACCEPTED.

**Deliverables** — grounding accuracy against a labelled set (precision/recall reported);
retry and verification semantics; UI-drift detection and recovery; idempotency handling for
interrupted action sequences; the Flight Recorder as a shipping artifact (browsable, replayable,
citable by the Proof Engine).

**Benchmark** — a scripted task completes on a fixture application across three window sizes and
two DPI settings; grounding precision/recall reported with the measurement method; an injected
mid-sequence failure is detected by the Verify step and recovered without duplicate side effects.

**Rung target** — R3. **Blocks** — 7D.

---

### Phase 7D — Application Adapters

**Objective** — Per-application integrations built on the 7B/7C foundation.
**Input** — 7C ACCEPTED.

**Deliverables** — an adapter registry; adapters for target applications. Each adapter declares
its **risk level**, its **grounding method**, and its **verification strategy** — an adapter that
cannot state how it verifies its actions is not accepted.

**Benchmark** — one adapter drives its application end-to-end on a scripted task, with a
replayable session artifact and a verification observation for every state-changing action.

**Rung target** — R3. **Blocks** — 9, 12.

---

# EPOCH V — VISION STUDIO

## Phase 8A — Design Foundation

**Objective** — The canonical design representation.
**Input** — 3A ACCEPTED.

**Deliverables** — `DesignDocument · Page · Frame · Component · Instance · Variant · Text ·
Vector · Image · Layout · Constraint · Interaction · Token · Variable · Style`; the **UI-IR**.

**Key decision** — *Design must be machine-readable and agent-addressable.* The AI manipulates
primitives, not screenshots.

**Entry points** — design model API; agent design tools.

**Benchmark** — an agent can construct and mutate a design through primitives only, with no
image manipulation.

**Rung target** — R3. **Blocks** — 8B, 8C.

---

## Phase 8B — Professional Design Canvas

**Objective** — A professional design surface.
**Input** — 8A ACCEPTED.

**Deliverables** — selection · move · resize · zoom · pan · alignment · distribution · grids ·
constraints · auto layout · components · variants · typography · design tokens · responsive
frames · prototyping · interactions.

**Benchmark** — a designer completes a realistic multi-screen layout without fighting the tool.

**Rung target** — R3. **Blocks** — 9.

---

## Phase 8C — Design ↔ Code

**Objective** — Bidirectional design↔code through a canonical intermediary.
**Input** — 8A ACCEPTED.

```
DESIGN MODEL ↔ UI INTERMEDIATE REPRESENTATION ↔ FRAMEWORK ADAPTER
```

**Adapters** — React · Next.js · HTML/CSS · Flutter · Jetpack Compose · SwiftUI · React Native.

**Hard rule** — *Do not make React DOM the design model.*

**Benchmark** — round-trip fidelity measured for **two structurally different targets** (e.g.
React and Jetpack Compose), with a fidelity number, not a visual impression.

**Rung target** — R3. **Blocks** — 9.

---

## Phase 9 — Visual Intelligence & Self-Repair

**Objective** — ZylCode sees its own work.
**Input** — 6B, 7, 8B, 8C ACCEPTED.

```
EXPECTED DESIGN → RENDERED SOFTWARE → SCREENSHOT → VISUAL ANALYSIS
   → DIFFERENCE MODEL → REPAIR PLAN → CODE/DESIGN CHANGE → RENDER AGAIN
```

**Benchmark** — a seeded visual defect (spacing/colour/layout) is detected and repaired without
human intervention, with before/after artifacts and the diff as evidence.

**Rung target** — R3. **Blocks** — 12, 16.

---

# EPOCH VI — DEVICES, DELIVERY AND ECOSYSTEM

## Phase 10 — Android Device Lab

**Objective** — Build **and use** the Android application.
**Input** — 5, 7 ACCEPTED.

**Deliverables** — Android SDK · emulator · ADB · physical devices · logcat · screenshots ·
install APK · launch · tap/type/swipe · inspect crash · Gradle build/test · release artifact.

**Benchmark** — a captured `adb` session: install, launch, interact, screenshot the running app.
*"Compiles successfully" is R2. Using the application is R3.*

**Rung target** — R3. **Blocks** — 12, 13, 16.

---

## Phase 11 — macOS/iOS Worker

**Objective** — Native Apple execution from a non-Apple host.
**Input** — 5 ACCEPTED.

```
ZylCode Windows/Linux ──secure worker protocol──► ZylCode Mac Worker
                                                   ├── Xcode
                                                   ├── Simulator
                                                   ├── xcodebuild
                                                   ├── signing
                                                   ├── devices
                                                   └── TestFlight
```

**Deliverables** — authenticated transport · capability advertisement · job isolation · artifact
return · no implicit trust of the remote worker. The Mac worker is **another `ExecutionBackend`**,
interchangeable behind the same contract.

**Benchmark** — a job submitted from Windows builds and runs on a simulator on the Mac worker,
with artifacts returned and the job attributable to a Mission step.

**Rung target** — R3. **Blocks** — 12, 13.

---

## Phase 12 — Proof Engine v2

**Objective** — Unify all verification into a formal Proof Graph.
**Input** — 7D, 9, 10, 11 ACCEPTED.

**Deliverables** — Proof Graph per `ZYLCODE_PROOF_GRAPH.md`; verification for compile · lint ·
unit · integration · browser · visual · accessibility · Android runtime · iOS runtime · security ·
packaging · deployment; **computed** rung assignment from evidence.

**Entry points** — `zylcode verify <claim>` · Evidence/Proof panel.

**Benchmark** — every capability in the registry carries a rung **computed from evidence**, not
asserted by hand. A capability claiming a rung above its evidence is flagged as a defect.

**Rung target** — R3. **Blocks** — 13, 16.

---

## Phase 13 — Delivery Engine

**Objective** — Understand how to finish the job.
**Input** — 12 ACCEPTED.

**Deliverables** — Git · PRs · CI/CD · Windows installers · macOS bundles · Linux packages ·
Docker · web deployment · Android AAB/APK · Play Store · iOS IPA/TestFlight/App Store · release
evidence bundles.

**Hard rule** — the Delivery Engine **cannot publish** an artifact whose required proofs are
below threshold.

**Benchmark** — a tagged release produced end-to-end with a complete release evidence bundle,
reproducible by a third party.

**Rung target** — R3/R4. **Blocks** — 16.

---

## Phase 14 — Multi-Agent Engineering

**Objective** — Sophisticated multi-agent behaviour, on top of shared world state.
**Input** — 3A, 3B, 4 ACCEPTED.

**Roles** — Planner · Architect · Developer · Designer · Tester · Reviewer · Debugger · Security ·
Release.

**Hard rule** — *Agents must not talk to one another by dumping giant chat transcripts.* They
share:

```
Project Graph · Repository Graph · Mission state · Artifacts · Evidence · Decisions
```

**Benchmark** — two agents coordinate on a task whose handoff is a **graph mutation**, not a
message; the handoff is inspectable and replayable.

**Rung target** — R3. **Blocks** — 15, 16.

---

## Phase 15 — Marketplace and Ecosystem

**Objective** — Distribution on top of the stable ABI.
**Input** — 5, 14 ACCEPTED.

**Deliverables** — plugins · skills · agents · MCP · models · themes · design systems · templates ·
runtimes · deployment providers; signed packages · permissions · versioning · compatibility ·
ratings · publisher identity · updates · security review.

**Benchmark** — a third party publishes a signed package; it installs, is permission-gated, and
contributes capability without core modification.

**Rung target** — R3. **Blocks** — 16.

---

## Phase 16 — Public Commissioning

**Objective** — The big demonstration and ecosystem push.
**Input** — 9, 10, 12, 13, 14, 15 ACCEPTED.

**Deliverables** — the **ZylCode Engineering Benchmark** measuring: task completion · build
success · test success · repair success · repository retrieval · visual fidelity · token
consumption · cost · latency · human intervention. Published with reproducible evidence.

**Also** — the governing 60-second demonstration (Constitution §1.4) executed end-to-end and
captured.

**Benchmark** — a third party reproduces the engineering benchmark against a tagged release and
obtains comparable numbers.

**Rung target** — R4/R5.

---

## 3. Continuous Maturity Tracks

| Track | Floor raised by |
|---|---|
| **Evidence integrity** | every phase (this is the product's core promise) |
| **Proof Engine** | 1D, 2A, 6B, 7, 9, 10, 11, 12, 13 |
| **Intelligence Graph** | 2A, 2B, 3A, 8C, 14 |
| **Model Platform** | 1C, 4, 14, 16 |
| **Security** | every phase; each extension adds attack surface |
| **Performance** | 2A (scan), 6B (preview), 10 (device), 16 (benchmark) |
| **Cross-platform correctness** | every phase; Windows is the primary target |

---

## 4. Gate Definitions

| Gate | Criterion | Decided by |
|---|---|---|
| **G0 Design** | Spec exists; PROPOSED/implemented markings accurate | Product owner |
| **G1 Implementation** | Committed; reachable from a named entry point | Builder |
| **G2 Self-evidence** | Reproduction block with raw output, scope-annotated metrics | Builder |
| **G3 Independent audit** | Third party reproduces; entry points invoked; rung falsified | Auditor |
| **G4 Acceptance** | ACCEPTED / ACCEPTED WITH CONDITIONS / NOT ACCEPTED | Product owner |

A phase failing G3 is **RE-OPENED** and blocks its dependents.

---

## 5. Immediate Actions

| # | Action | Status |
|---|---|---|
| 1 | Audit `0ecea8e` | ✔ DONE — **NOT ACCEPTED** |
| 2 | Publish governance package | ✔ DONE |
| 3 | Supersede six-engine / 14-stage drafts | ✔ DONE |
| 4 | Issue Phase 2A remediation order | ✔ DONE — `PHASE2A_REMEDIATION_ORDER.md` |
| 5 | Re-audit Phase 2A | blocked on #4 execution — **remediation has NOT started** |
| 6 | Authorize Phase 2B | blocked on #5 |
| 7 | Authorize Phase 3A | blocked on #6 |
| 8 | **Restore a green baseline** | **NEW — required before #5** (see below) |
| 9 | **Resolve the dirty tree** | **NEW — required before #5** |

**Phase 2B does not begin until Phase 2A is re-accepted.**

### Newly surfaced blockers (sweep `STATUS_SWEEP_2026-09-16.md`)

| Blocker | Detail |
|---|---|
| ~~**Suite is red**~~ | ✔ **CLOSED** at `8f751ed`. `router.rs:1030` was asserting a retired XML contract while `synthetic_response()` returns `AgentDecision` JSON — and the test could not have detected either state, because `TokenRouter::new()` read a stale `./vector_cache.db`. Fixed hermetically + falsification-proven. Suite: **261 passed, 0 failed**. See `STATUS_SWEEP_2026-09-16.md` §14. |
| **Dirty tree** | **88 entries** — characterised 2026-09-17 (`STATUS_SWEEP_2026-09-16.md` §15). 52 modified source files (4157/1545 raw, 3487/875 whitespace-insensitive), 33 untracked. Compiles clean, clippy-clean. It is mostly real work mixed with formatting churn from `core.autocrlf=true` with **no `.gitattributes`**. Includes untracked source `performance.rs` (409 lines) + `system_integration.rs` (391 lines) — declared in `lib.rs`, never reviewed — and `commissioning_test.rs`, a `#[tokio::test]` attempting **real provider inference** that **asserts nothing** and returns `Ok(())` on failure. Committing that as-is puts a no-op, network-dependent test into CI. Per-group disposition in §15.7. **Owner decision.** |
| **Retracted claim re-entered** | **Finding F.** `PHASE1C_COMPLETION_REPORT.md:59` records `156 tools` as removed-retracted. It is present in **8 files**, including **tracked** `FINAL_SUMMARY.md`. Measured: **112** tool IDs. Corrected in `FINAL_SUMMARY.md`; standing grep check added to `README_INDEX.md` §7. Should become a CI step. |
| **Master prompt clobbered** | **Finding G.** `docs/governance/DEEPSEEK_MASTER_PROMPT_V21.md` — committed 522 lines → working tree 156 lines of a stale v1.0 `ZYLCODE_COMMERCIAL_MODEL.md`, **missing the v1.1 gate correction**. Recoverable from HEAD. **Not reverted** (another session's in-flight edit). **Owner decision.** |
| **⛔ MAIN DOES NOT COMPILE** | **Highest priority.** A clean worktree at `1338d0b` fails `cargo check --lib` with **10 errors**: `E0583` ×2 (`lib.rs` declares `pub mod performance;` / `pub mod system_integration;` but **neither file is in git** — they exist only as *untracked* working-tree files) + `E0599` ×8 (`enhanced_bridge.rs` **at HEAD line 87–94 calls** eight `get_more_*_tools()` methods it **does not define**; they exist only in the dirty tree, +775 lines). **Two independent defects, both committed.** The working tree compiles *because* of the uncommitted work — so the defect is invisible to whoever is editing it. `.github/workflows/ci.yml:69` runs `cargo check` on a clean checkout, so CI must be red (CI state itself **NOT OBSERVABLE** — invalid token). See `PARALLEL_EXECUTION_MANIFEST.md` §9. **Repair sequence: FIX-1 commit the 2 modules → FIX-2 commit the 8 methods → verify a clean worktree builds.** Owner decision: this commits another session's unreviewed work. |
| **Dirty tree is load-bearing** | The 87-entry dirty tree **cannot be discarded** as previously recommended — it holds the only copy of the two missing modules and the 775 lines of `get_more_*` implementations. Discarding it leaves `main` permanently unbuildable. `STATUS_SWEEP §15.7`'s "revert Class D" advice stands, but "delete the untracked completion reports" must now exclude any file the build depends on. |
| **Build blocker was misdiagnosed** | The sweep recorded `cargo build` as *"blocked by sandbox (build-script execution denied)"*. **Falsified 2026-09-17.** Two real causes, neither a sandbox restriction: (1) `git worktree add /tmp/…` resolves to `C:/tmp/…` — outside the writable tree; use a path **inside** the project. (2) `--offline` against an empty registry fails on `clap_derive`; the first build needs network. **Worktrees are viable** — proven by building one (`.wt/probe`, then removed). Blocker #3 is re-classified: not `BLOCKED BY SANDBOX` but **blocked by broken HEAD**. |
| **Registry has no rungs** | 23 of 34 entries carry `rung: null`. Remediation P1-6 was half-applied: the GREEN count was corrected, but the honest downgrade was not completed. |
