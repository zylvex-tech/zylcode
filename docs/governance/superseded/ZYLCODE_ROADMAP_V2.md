> # ⚠️ SUPERSEDED — DO NOT IMPLEMENT FROM THIS DOCUMENT
>
> **Status:** SUPERSEDED, never committed, archived 2026-09-16.
> **Superseded by:** `docs/roadmap/ZYLCODE_ROADMAP_V2.md`
>
> **Why it was superseded:** it describes a **14-stage sequential** program. The approved program
> is **16 phases grouped into 6 epochs**, because treating foundations (Project model, extension
> ABI, design representation, runtime contract) as isolated sequential blocks risks reaching
> Stage 10 before discovering one of those contracts was wrong.
>
> Retained for history only. **Do not implement from this document.**

---

# ZylCode Roadmap v2.0

> 14-stage program with stage boundaries, gates, dependencies, and deliverables.
> Governing specification for all implementation agents.

---

## 0. How to Read This Document

Each stage has:

- **Objective** — what the stage achieves
- **Input** — what must exist before the stage begins
- **Deliverables** — what must ship when the stage completes
- **Gates** — verification criteria that must pass
- **Dependencies** — which other stages this stage requires
- **Blocks** — which stages cannot begin until this stage completes
- **Benchmark** — measurable acceptance criteria
- **Duration Estimate** — rough sizing (not a commitment)

Stages are sequential where arrows point, parallel where branches
exist. No stage begins before its dependencies are COMPLETE.

---

## 1. Dependency Graph

```
Stage 1 (Truth) ──────────────────────────────────────────────────►
     │
     ▼
Stage 2 (Understanding) ──────────────────────────────────────────►
     │
     ▼
Stage 3 (Intelligence) ───────────────────────────────────────────►
     │
     ▼
Stage 4 (Extensibility) ──────────────────────────────────────────►
     │
     ├──────────────────────┬──────────────────────────────────────┐
     ▼                      ▼                                      ▼
Stage 5 (Execution)   Stage 6 (Vision)                        Stage 8 (Mobile)
     │                      │                                      │
     ▼                      ▼                                      ▼
Stage 7 (Visual Intelligence) ◄────────────────────────────── Stage 9 (Apple)
     │
     ▼
Stage 10 (Teams) ◄─────────────────────────────────────────────────
     │
     ▼
Stage 11 (Delivery) ◄──────────────────────────────────────────────
     │
     ▼
Stage 12 (Ecosystem)
     │
     ▼
Stage 13 (Benchmark)
     │
     ▼
Stage 14 (Launch)
```

### Parallel Opportunities

After Stage 4 completes:
- **Stream A**: Stage 5 → Stage 7 (Execution + Visual Intelligence)
- **Stream B**: Stage 6 → Stage 7 (Vision → Visual Intelligence)
- **Stream C**: Stage 8 → Stage 9 (Mobile → Apple)

Stage 7 requires both Stage 5 and Stage 6.
Stage 10 requires Stage 4 + at least one of Stage 5/6.
Stage 11 requires Stage 5 + Stage 8 (or Stage 9).

---

## 2. Stage 1 — Truth

### Objective

Build the foundational agent runtime: tool execution, agent loop,
evidence ledger, memory, crash recovery, and approval model.

### Status: ✅ COMPLETE

### Input

- Empty repository

### Deliverables

| Deliverable | Status | Evidence |
|-------------|--------|----------|
| Agent loop with decision protocol | ✅ Complete | `agent.rs`, `agent_protocol.rs` |
| Evidence Ledger (append-first, hash-chained) | ✅ Complete | `ledger.rs`, `sqlite_ledger.rs` |
| Crash recovery (checkpoint/resume) | ✅ Complete | `agent.rs`, `crash_recovery.rs` tests |
| Fail-closed verification | ✅ Complete | `agent.rs` |
| Risk-based approval model | ✅ Complete | `agent_protocol.rs` |
| MCP server | ✅ Complete | `zylcode-mcp` crate |
| CLI interface | ✅ Complete | `zylcode-cli` crate |
| Desktop shell (Tauri v2) | ✅ Complete | `zylcode-desktop` |
| React frontend | ✅ Complete | `apps/zylcode-desktop/src/` |
| Tool runtime (filesystem, shell, git) | ✅ Complete | `zylcode-mcp/src/real_tools.rs` |

### Gates

| Gate | Criterion | Status |
|------|-----------|--------|
| All tests pass | 337+ tests green | ✅ |
| Clippy clean | Zero warnings | ✅ |
| Crash recovery works | Resume after kill | ✅ |
| Evidence integrity | Hash chain unbroken | ✅ |
| Frontend builds | `pnpm build` succeeds | ✅ |

### Benchmark

| Metric | Target | Actual |
|--------|--------|--------|
| Test suite | ≥100 tests | 337 ✅ |
| Crash recovery | 3+ crash window tests | 5 ✅ |
| Agent loop | E2E test passing | ✅ |

---

## 3. Stage 2 — Understanding

### Objective

Build ZylCode's structured understanding of the software being created:
file inventory, manifest parsing, symbol extraction, dependency graph,
entry points, architectural fingerprint, git intelligence, persistence,
query API, context retrieval, and commissioning benchmark.

### Status: ✅ COMPLETE

### Input

- Stage 1 complete (agent loop, evidence ledger)

### Deliverables

| Deliverable | Status | Evidence |
|-------------|--------|----------|
| Repository scanner | ✅ | `intelligence/scanner.rs` |
| Language classifier | ✅ | `intelligence/classifier.rs` |
| Manifest parser (Cargo, pnpm, npm) | ✅ | `intelligence/manifest.rs` |
| Symbol extractor (Rust, TypeScript) | ✅ | `intelligence/symbols.rs` |
| Dependency graph | ✅ | `intelligence/dependency.rs` |
| Entry-point discovery | ✅ | `intelligence/entry_points.rs` |
| Architectural fingerprint | ✅ | `intelligence/architecture.rs` |
| Git/Change intelligence | ✅ | `intelligence/git.rs` |
| SQLite persistence | ✅ | `intelligence/store.rs` |
| Query API (14 methods) | ✅ | `intelligence/query.rs` |
| Context retriever | ✅ | `intelligence/context.rs` |
| Commissioning benchmark | ✅ | `tests/repo_intelligence_benchmark.rs` |

### Gates

| Gate | Criterion | Status |
|------|-----------|--------|
| All tests pass | 79 intelligence tests + 337 workspace | ✅ |
| Clippy clean | Zero warnings | ✅ |
| Benchmark passes | P@10≥0.25, R@10≥0.30 | ✅ (0.30, 0.53) |
| Performance | Scan <120s for 15K files | ✅ (12.7s) |
| Frontend builds | `pnpm build` succeeds | ✅ |

### Benchmark

| Metric | Target | Actual |
|--------|--------|--------|
| Files indexed | ≥1000 | 14,973 ✅ |
| Symbols extracted | ≥100 | 1,444 ✅ |
| Packages discovered | ≥2 | 5 ✅ |
| Entry points | ≥1 | 15 ✅ |
| Precision@10 | ≥0.25 | 0.30 ✅ |
| Recall@10 | ≥0.30 | 0.53 ✅ |
| Scan duration | <120s | 12.7s ✅ |

---

## 4. Stage 3 — Intelligence

### Objective

Add model routing, context optimization, model benchmarking, and
deeper intelligence (import graph, reference graph, semantic index).

### Status: NEXT

### Input

- Stage 2 complete (Intelligence Graph with query API)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Model Router | Task classification → model selection |
| Provider Registry | Register/manage model providers (OpenAI, Anthropic, DeepSeek, Ollama, etc.) |
| Performance Tracker | Record per-task-model outcomes |
| Context Optimizer | Token-aware context assembly with priority ranking |
| Import Graph | File→file import relationships |
| Reference Graph | Symbol→symbol references and calls |
| Semantic Index | Vector embeddings for semantic retrieval |
| Model Benchmark | Per-model performance measurement |
| Sovereign Fallback | Graceful degradation to local models |

### Gates

| Gate | Criterion |
|------|-----------|
| All tests pass | Full workspace green |
| Model routing | ≥80% correct task→model classification |
| Context optimization | ≥20% improvement in benchmark recall |
| Sovereign mode | System functional with Ollama only |
| Import graph | ≥90% import detection for Rust/TS |
| Semantic index | Vector search returns relevant results |

### Benchmark

| Metric | Target |
|--------|--------|
| Model routing accuracy | ≥0.80 |
| Context recall improvement | ≥20% over Stage 2 |
| Import detection rate | ≥0.90 |
| Semantic search relevance | ≥0.70 |
| Sovereign mode: code gen | Functional (reduced quality) |
| Sovereign mode: intelligence | Full capability |

### Dependencies

- Stage 2 ✅

### Blocks

- Stage 4 (needs model routing for plugin model providers)

---

## 5. Stage 4 — Extensibility

### Objective

Build the plugin/skill/MCP/agent/workflow runtime that makes ZylCode
composable. Everything becomes a plugin.

### Status: PLANNED

### Input

- Stage 3 complete (model routing, context optimization)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Plugin Runtime | Define, approve, activate, update, stop, undefine plugins |
| Service Registry | Register and discover services |
| Event Bus | Publish/subscribe for system events |
| Skill System | Register, load, and invoke skills |
| MCP Client | Connect to external MCP servers |
| Agent Registry | Register and manage agent configurations |
| Workflow Engine | Multi-step, multi-agent orchestration |
| Plugin Sandbox | Process isolation, permission enforcement |
| Plugin Manifest | Standard manifest format for all plugin types |
| Creator Studio (basic) | Visual plugin/skill/workflow builder |

### Gates

| Gate | Criterion |
|------|-----------|
| Plugin lifecycle | Define→Activate→Run→Stop works |
| Service registry | Services discoverable across plugins |
| Event bus | Events delivered to all subscribers |
| Sandbox isolation | Plugins cannot access unauthorized resources |
| Plugin load time | <500ms for typical plugin |
| MCP client | Connect to external MCP server |
| Workflow engine | Multi-step workflow executes correctly |

### Benchmark

| Metric | Target |
|--------|--------|
| Plugin activation time | <500ms |
| Plugin isolation | 100% permission enforcement |
| MCP connectivity | ≥3 external MCP servers |
| Workflow execution | 3+ step workflow completes |
| Creator Studio | Visual plugin creation works |

### Dependencies

- Stage 3 (model providers are plugins)

### Blocks

- Stage 5 (execution adapters are plugins)
- Stage 6 (design tools are plugins)
- Stage 8 (device adapters are plugins)
- Stage 10 (agents are plugins)
- Stage 12 (marketplace distributes plugins)

---

## 6. Stage 5 — Execution

### Objective

Build browser and desktop runtime inspection: the agent can launch
software, see it running, take screenshots, and interact with it.

### Status: PLANNED

### Input

- Stage 4 complete (plugin runtime, execution adapters as plugins)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Chrome CDP Adapter | Launch, navigate, screenshot, DOM snapshot, click, type, evaluate JS |
| Firefox Adapter | Launch, navigate, screenshot via Marionette |
| Desktop Adapter | Tauri window control, IPC |
| Build Controller | Detect build system, run builds, capture output |
| Inspection Pipeline | Build→Launch→Render→Screenshot→Inspect |
| Screenshot Capture | Full-page and element screenshots |
| DOM Snapshot | Structured DOM tree for agent consumption |
| Console Capture | JavaScript console output |
| Network Capture | HTTP request/response logging |
| Sandbox Manager | Process isolation, resource limits |

### Gates

| Gate | Criterion |
|------|-----------|
| Chrome control | Launch, navigate, screenshot works |
| Build integration | Cargo + npm builds succeed |
| Inspection pipeline | Build→screenshot in <30s |
| DOM snapshot | Structured tree returned |
| Console capture | Errors captured |
| Sandbox | Process isolated, timeout enforced |

### Benchmark

| Metric | Target |
|--------|--------|
| Build→screenshot time | <30s |
| Screenshot capture | <2s |
| DOM snapshot | <1s |
| Chrome launch | <5s |
| Console capture | 100% error capture |

### Dependencies

- Stage 4 (execution adapters as plugins)

### Blocks

- Stage 7 (needs runtime inspection for visual QA)
- Stage 10 (agents need execution access)
- Stage 11 (needs build/test for delivery)

---

## 7. Stage 6 — Vision

### Objective

Build the integrated Vision Studio: a Figma/Penpot-class canvas where
canvas and code are two representations of the same product.

### Status: PLANNED

### Input

- Stage 4 complete (design tools as plugins)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Canvas Engine | WebGL/WebGPU rendering, Flexbox/Grid layout |
| Code Bridge | Bidirectional AST-aware sync between canvas and code |
| Component System | Component definition, variants, instances, overrides |
| Design Token System | Color, typography, spacing, radius, shadow, motion tokens |
| Theme System | Light/dark themes, brand variants |
| Responsive Layout | Breakpoints, constraints, auto-layout |
| Asset Management | Images, icons, SVGs, fonts |
| Prototype Links | Click-through prototyping |
| Design System Intelligence | AI understands tokens and constraints |
| Canvas ↔ Agent | Agent can read/modify canvas state |

### Gates

| Gate | Criterion |
|------|-----------|
| Canvas rendering | 60fps with 1000+ elements |
| Code bridge: Canvas→Code | Property change reflects in source |
| Code bridge: Code→Canvas | Source change reflects on canvas |
| Component system | Variants and instances work |
| Token system | Tokens applied to elements |
| Responsive layout | Breakpoints switch correctly |
| Agent integration | Agent can inspect and modify canvas |

### Benchmark

| Metric | Target |
|--------|--------|
| Canvas framerate | ≥60fps at 1000 elements |
| Canvas→Code fidelity | ≥0.95 property preservation |
| Code→Canvas fidelity | ≥0.95 visual preservation |
| Component rendering | Variants render correctly |
| Token application | 100% token usage verification |

### Dependencies

- Stage 4 (design tools as plugins)

### Blocks

- Stage 7 (needs canvas for design comparison)

---

## 8. Stage 7 — Visual Intelligence

### Objective

Combine Execution Engine (Stage 5) and Vision Studio (Stage 6) to
create design↔code sync with screenshot inspection and automatic repair.

### Status: PLANNED

### Input

- Stage 5 complete (runtime inspection)
- Stage 6 complete (Vision Studio)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Visual QA Pipeline | Screenshot→Inspect→Compare→Report→Repair loop |
| Design Comparison | Pixel diff, structural diff, token verification |
| Screenshot Inspector | Vision model analyzes rendered output |
| Auto-Repair | Agent fixes visual issues detected by inspection |
| Visual Regression | Detect unintended visual changes |
| Accessibility Audit | Automated WCAG compliance checking |
| Design System Verification | Verify components use design tokens correctly |

### Gates

| Gate | Criterion |
|------|-----------|
| Visual QA pipeline | End-to-end loop works |
| Design comparison | Pixel diff detects real differences |
| Auto-repair | Agent fixes ≥50% of detected issues |
| Visual regression | ≥90% change detection rate |
| Accessibility audit | WCAG 2.1 AA automated checks |

### Benchmark

| Metric | Target |
|--------|--------|
| Defect detection rate | ≥0.80 |
| Auto-repair success | ≥0.50 |
| Visual regression accuracy | ≥0.90 |
| Accessibility coverage | WCAG 2.1 AA |
| Full pipeline time | <60s |

### Dependencies

- Stage 5 (runtime inspection)
- Stage 6 (Vision Studio)

### Blocks

- Stage 10 (agents use visual QA)
- Stage 13 (benchmark includes visual tasks)

---

## 9. Stage 8 — Mobile

### Objective

Add Android device lab: emulator and physical device control for
build, install, launch, interact, screenshot, and inspect.

### Status: PLANNED

### Input

- Stage 4 complete (device adapters as plugins)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| ADB Controller | Device discovery, install, launch, shell commands |
| Emulator Manager | Create, start, stop, configure emulators |
| UI Automator | Element interaction (tap, swipe, type, scroll) |
| Android Build Controller | Gradle build, APK/AAB signing |
| Android Inspector | Screenshots, logcat, UI hierarchy |
| Android Sandbox | Process isolation for device interaction |

### Gates

| Gate | Criterion |
|------|-----------|
| Emulator launch | Emulator starts in <60s |
| APK install | Install succeeds |
| App launch | App launches on emulator |
| Screenshot | Screenshot captured |
| UI interaction | Tap, type, swipe work |
| Logcat capture | Logs captured |

### Benchmark

| Metric | Target |
|--------|--------|
| Build→emulator time | <120s |
| Emulator launch | <60s |
| Screenshot capture | <5s |
| UI interaction | <1s response |

### Dependencies

- Stage 4 (device adapters as plugins)

### Blocks

- Stage 9 (mobile patterns inform iOS)
- Stage 11 (needs mobile delivery)

---

## 10. Stage 9 — Apple

### Objective

Add iOS/macOS device lab through remote macOS worker: simulator
and physical device control.

### Status: PLANNED

### Input

- Stage 8 complete (mobile patterns established)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Remote macOS Worker | SSH-based remote build and test worker |
| Xcode Build Controller | xcodebuild integration |
| Simulator Controller | xcrun simctl for simulator management |
| XCUITest Framework | UI testing on simulator and device |
| iOS Inspector | Screenshots, console, UI hierarchy |
| libimobiledevice | Physical iOS device control |
| iOS Sandbox | Process isolation for device interaction |

### Gates

| Gate | Criterion |
|------|-----------|
| Remote worker | SSH connection to macOS works |
| Simulator launch | Simulator starts |
| App install | IPA installs on simulator |
| Screenshot | Screenshot captured |
| XCUITest | UI tests run on simulator |
| Physical device | Basic device control works |

### Benchmark

| Metric | Target |
|--------|--------|
| Build→simulator time | <180s |
| Simulator launch | <30s |
| Screenshot capture | <5s |
| XCUITest execution | <60s |

### Dependencies

- Stage 8 (mobile patterns)

### Blocks

- Stage 11 (needs iOS delivery)

---

## 11. Stage 10 — Teams

### Objective

Enable multi-agent engineering organization: orchestrator assigns
roles, agents work in parallel on different branches, quality gates
ensure integration.

### Status: PLANNED

### Input

- Stage 4 complete (agents as plugins)
- At least one of Stage 5/6 complete (agents need execution or design)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Orchestrator | Mission decomposition, agent assignment, monitoring |
| Agent Roles | Product, Architect, Designer, Frontend, Backend, Test, Security, Reviewer, Release |
| Task Ownership | File-level ownership prevents conflicts |
| Branch Strategy | One branch per agent, merge via quality gate |
| Handoff Protocol | Structured message passing between agents |
| Shared World State | Intelligence Graph, evidence, design system as shared context |
| Quality Gate | Automated merge verification |
| Progress Dashboard | Real-time mission progress visualization |

### Gates

| Gate | Criterion |
|------|-----------|
| Multi-agent execution | 2+ agents work in parallel |
| Task ownership | No file conflicts between agents |
| Quality gate | All merges pass verification |
| Handoff protocol | Agent-to-agent communication works |
| Progress dashboard | Real-time updates visible |

### Benchmark

| Metric | Target |
|--------|--------|
| Parallel agents | ≥3 agents simultaneously |
| Task completion rate | ≥0.70 for multi-agent tasks |
| Merge conflict rate | <10% of merges |
| Quality gate pass rate | ≥0.90 |

### Dependencies

- Stage 4 (agents as plugins)
- Stage 5 or Stage 6 (execution or design)

### Blocks

- Stage 13 (benchmark includes multi-agent tasks)

---

## 12. Stage 11 — Delivery

### Objective

Build CI/CD, installers, cloud deployment, and app store submission
capabilities.

### Status: PLANNED

### Input

- Stage 5 complete (build/test pipeline)
- Stage 8 or Stage 9 complete (mobile delivery)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Windows Installer | NSIS via Tauri |
| macOS Installer | DMG via Tauri |
| Linux Packages | DEB, RPM, AppImage, Flatpak via Tauri |
| Web Deployment | Static hosting, Docker, CDN |
| Android Delivery | APK/AAB signing, Play Store submission |
| iOS Delivery | IPA signing, App Store submission |
| CI Pipeline | GitHub Actions (when billing resolved) |
| Release Pipeline | Verify→Package→Sign→Test→Stage→Deploy |
| Rollback | Automated rollback on failure |
| Evidence | Full delivery evidence trail |

### Gates

| Gate | Criterion |
|------|-----------|
| Windows installer | Installs and runs on clean Windows |
| macOS installer | Installs and runs on clean macOS |
| Linux package | Installs and runs on clean Linux |
| Web deployment | Deploys and serves correctly |
| Android APK | Installs and runs on emulator |
| CI pipeline | Full pipeline green (when available) |
| Rollback | Rollback works within 60s |

### Benchmark

| Metric | Target |
|--------|--------|
| Build→installer time | <300s |
| Installer size | <100MB |
| Install time | <60s |
| Rollback time | <60s |

### Dependencies

- Stage 5 (build/test pipeline)
- Stage 8 or 9 (mobile delivery)

### Blocks

- Stage 14 (launch requires delivery)

---

## 13. Stage 12 — Ecosystem

### Objective

Build marketplace and Creator Studio for distributing and creating
plugins, skills, MCP servers, themes, design systems, and templates.

### Status: PLANNED

### Input

- Stage 4 complete (plugin system)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Marketplace Protocol | Package format, signing, versioning, dependencies |
| Marketplace Client | Search, install, update, remove packages |
| Marketplace Server | Registry, authentication, package storage |
| Publisher Tools | Package creation, upload, documentation |
| Trust System | Ratings, security audits, permission declarations |
| Creator Studio | Visual builder for plugins, skills, workflows, tools |
| Template System | Project templates with scaffolding |
| Theme System | Distributable themes for workspace and canvas |
| Design System Distribution | Distributable design systems with tokens and components |

### Gates

| Gate | Criterion |
|------|-----------|
| Package install | Install from marketplace works |
| Package signing | Signed packages verified |
| Creator Studio | Visual plugin creation works |
| Trust system | Permissions displayed before install |
| Template system | Template scaffolding works |

### Benchmark

| Metric | Target |
|--------|--------|
| Package install time | <10s |
| Marketplace search | <1s response |
| Creator Studio | Visual creation of 3+ plugin types |
| Template scaffolding | <30s |

### Dependencies

- Stage 4 (plugin system)

### Blocks

- Stage 14 (launch needs ecosystem)

---

## 14. Stage 13 — Benchmark

### Objective

Create a public, reproducible autonomous-engineering benchmark that
demonstrates ZylCode's capability.

### Status: PLANNED

### Input

- Stages 1-12 substantially complete

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Benchmark Tasks | 7 difficulty levels from bug fix to full autonomous ship |
| Benchmark Harness | Automated task execution and measurement |
| Benchmark Metrics | Completion rate, evidence quality, visual QA, test coverage, time |
| Benchmark Results | Published results for ZylCode |
| Benchmark Protocol | Documented protocol for third-party comparison |
| Reproducibility Package | Docker/VM image for independent verification |

### Benchmark Tasks

| Level | Task | Measures |
|-------|------|----------|
| 1 | Fix this bug (given failing test) | Code correctness |
| 2 | Add this feature (given spec) | Implementation quality |
| 3 | Refactor this module (given goals) | Code quality improvement |
| 4 | Build this component (given design) | Design→code fidelity |
| 5 | Build this application (given description) | Full-stack capability |
| 6 | Build with mobile support (given description) | Multi-platform capability |
| 7 | Build, test, and ship (given description) | End-to-end autonomy |

### Gates

| Gate | Criterion |
|------|-----------|
| Task completion | ≥60% of Level 5 tasks complete |
| Evidence completeness | ≥90% of actions have evidence |
| Visual QA | ≥80% of visual tasks pass |
| Test coverage | ≥70% coverage for generated code |
| Reproducibility | Third-party can reproduce results |

### Dependencies

- Stages 1-12 substantially complete

### Blocks

- Stage 14 (launch needs benchmark results)

---

## 15. Stage 14 — Launch

### Objective

Take ZylCode from internal development to public availability
through developer preview, alpha, beta, and stable releases.

### Status: PLANNED

### Input

- Stage 13 complete (benchmark demonstrates capability)

### Deliverables

| Deliverable | Description |
|-------------|-------------|
| Developer Preview | Early access for developers, limited features |
| Alpha | Feature-complete but known rough edges |
| Beta | Feature-complete, polished, community feedback |
| Stable | Production-ready release |
| Documentation | Complete user docs, API docs, contributor docs |
| Website | Product website with downloads, docs, community |
| Community | Discord/forum, contribution guide, governance |
| Marketing | Launch materials, demo videos, blog posts |

### Launch Phases

| Phase | Audience | Duration | Criteria to Advance |
|-------|----------|----------|-------------------|
| Developer Preview | Invited developers | 4-8 weeks | Feedback collected, critical bugs fixed |
| Alpha | Open registration | 4-8 weeks | Feature-complete, benchmark passing |
| Beta | Open | 4-8 weeks | Community feedback positive, stability good |
| Stable | General availability | Ongoing | All gates pass |

### Gates

| Gate | Criterion |
|------|-----------|
| Developer Preview | Core workflow works, 5+ invited users |
| Alpha | All Stage 1-7 features working |
| Beta | All Stage 1-12 features working |
| Stable | Benchmark passing, documentation complete, no P0 bugs |

### Dependencies

- Stage 13 (benchmark)

---

## 16. Summary Table

| Stage | Name | Status | Dependencies | Blocks | Key Metric |
|-------|------|--------|-------------|--------|-----------|
| 1 | Truth | ✅ COMPLETE | — | 2 | 337 tests pass |
| 2 | Understanding | ✅ COMPLETE | 1 | 3 | P@10=0.30, R@10=0.53 |
| 3 | Intelligence | NEXT | 2 | 4 | Model routing ≥0.80 |
| 4 | Extensibility | PLANNED | 3 | 5,6,8,10,12 | Plugin load <500ms |
| 5 | Execution | PLANNED | 4 | 7,10,11 | Build→screenshot <30s |
| 6 | Vision | PLANNED | 4 | 7 | Canvas 60fps |
| 7 | Visual Intelligence | PLANNED | 5,6 | 10 | Defect detection ≥0.80 |
| 8 | Mobile | PLANNED | 4 | 9,11 | Build→emulator <120s |
| 9 | Apple | PLANNED | 8 | 11 | Build→simulator <180s |
| 10 | Teams | PLANNED | 4+5/6 | 13 | ≥3 parallel agents |
| 11 | Delivery | PLANNED | 5+8/9 | 14 | Build→installer <300s |
| 12 | Ecosystem | PLANNED | 4 | 14 | Install <10s |
| 13 | Benchmark | PLANNED | 1-12 | 14 | ≥60% Level 5 completion |
| 14 | Launch | PLANNED | 13 | — | Stable release |

---

## 17. Risk Register

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Canvas↔Code sync complexity | High | Start with limited component set, expand iteratively |
| iOS device lab (no macOS) | High | Remote macOS worker, cloud Mac services |
| Model routing accuracy | Medium | Start with manual override, improve with data |
| Multi-agent coordination | High | Start with 2 agents, expand gradually |
| GitHub CI billing | Medium | EXTERNAL_BLOCKER, alternative CI when needed |
| Vision Studio scope | High | Phased: basic canvas → components → full design system |
| Performance at scale | Medium | Benchmark at each stage, optimize early |
| Plugin security | High | Sandbox-first, permission declarations, audit |

---

*ZylCode Roadmap v2.0*
*Companion to ZylCode Product Constitution v2.0 and Architecture v2.0*
*Status: DRAFT — pending review*
