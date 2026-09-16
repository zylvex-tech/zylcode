> # ⚠️ SUPERSEDED — DO NOT IMPLEMENT FROM THIS DOCUMENT
>
> **Status:** SUPERSEDED, never committed, archived 2026-09-16.
> **Superseded by:** `docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md`
>
> **Why it was superseded:** this draft defines **six** engines. The approved architecture defines
> **seven** — the missing one is the **Project System**, which is the persistent identity and
> shared world state of everything ZylCode does. Building from this draft would produce the wrong
> product.
>
> Retained for history only. **Do not implement from this document.**

---

# ZylCode Product Constitution v2.0

> **ZylCode — an open, evidence-first Software Creation OS.**
>
> **Design it. Build it. Run it. See it. Verify it. Ship it.**

---

## 0. Preamble

This document is the governing specification for ZylCode.

Every architectural decision, every implementation phase, every agent
that works on ZylCode, and every capability claim made about ZylCode
must be consistent with this Constitution.

When a conflict arises between this Constitution and an implementation
detail, this Constitution wins unless an amendment is explicitly recorded
with rationale, date, and author.

This is a living document. Amendments are append-only. Nothing is
silently rewritten.

---

## 1. Product Definition

### 1.1 What ZylCode Is

ZylCode is a **Software Creation Operating System**.

It is a single, integrated environment where a person — or a team of
humans and AI agents — can take a software product from idea through
architecture, visual design, code, build, test, runtime inspection,
verification, packaging, and release.

The IDE is the shell. Underneath are engines that make software creation
composable, verifiable, and observable.

### 1.2 What ZylCode Is Not

- ZylCode is **not** an IDE with AI features bolted on.
- ZylCode is **not** a chatbot that edits files.
- ZylCode is **not** a wrapper around a single model provider.
- ZylCode is **not** a Figma clone with code export.
- ZylCode is **not** a CI/CD platform with a text editor.
- ZylCode is **not** a marketplace with a runtime.

It is the integration of all of those concerns into one coherent system
where the boundary between design, code, build, test, and delivery
is dissolved.

### 1.3 The Six Engines

Beneath the workspace shell, ZylCode is composed of six engines:

```
┌─────────────────────────────────────────────────────────────┐
│                    CREATION WORKSPACE                        │
│                                                             │
│  CODE │ CANVAS │ PREVIEW │ TERMINAL │ TEST │ GIT │ SHIP     │
└─────────────────────────────┬───────────────────────────────┘
                              │
           ┌──────────────────┼──────────────────┐
           ▼                  ▼                  ▼
     AGENT KERNEL    INTELLIGENCE GRAPH   VISION STUDIO
           │                  │                  │
   ┌───────┼───────┐    Repository         UI/UX Canvas
   │       │       │    Architecture        Prototypes
 Models   Tools   Skills   Symbols            Tokens
 Agents   MCP    Plugins   Dependencies       Components
   │       │       │       Evidence           Code Sync
   └───────┼───────┘
           │
           ▼
     EXECUTION ENGINE
           │
 ┌─────────┼──────────┐
 ▼         ▼          ▼
Browser  Desktop   Devices
                      │
               ┌──────┴──────┐
               ▼             ▼
           Android         iOS
                         Remote Mac
           │
           ▼
      PROOF ENGINE
           │
     Build / Test
     Visual QA
     Runtime QA
     Security
     Evidence
     Regression
           │
           ▼
     DELIVERY ENGINE
     Git / CI / Release
     Stores / Deploy
```

Each engine is described in detail in the Architecture v2 document.

### 1.4 One Sentence

> ZylCode is where you go to turn an idea into verified, shipped
> software — with AI agents that can see what they build, prove that
> it works, and explain what they did.

---

## 2. Governing Principles

These principles are non-negotiable. They apply to every engine, every
stage, every implementation decision.

### 2.1 Evidence-First

**Every consequential action must produce verifiable evidence.**

If an agent writes code, there is a ledger entry. If a test passes,
there is proof. If a UI renders correctly, there is a screenshot with
comparison. If a build succeeds, there is a log.

No evidence = no claim.

### 2.2 Fail-Closed

**When in doubt, fail.**

Verification errors resolve to FAILED, not Verified. Missing evidence
is not silence — it is a red flag. Ambiguous states are reconciled or
escalated, never silently passed.

This applies to:
- Agent verification decisions
- Test results
- Build outcomes
- UI inspection
- Security checks
- Model responses
- Plugin execution
- Deployment gates

### 2.3 Composable

**Everything that can be extended, is extended through a uniform
composition model.**

Models, tools, skills, plugins, MCP servers, agents, workflows,
sandboxes, memory providers, design systems, device adapters,
verification providers, and deployment targets are all composable
capabilities.

The composition model is the same whether you are adding a local
Ollama model or an Android device farm.

### 2.4 Observable

**Every agent action is inspectable.**

The user can open a Trajectory view and see:
- What the agent planned
- What it decided
- What tools it called
- What evidence it collected
- What it verified
- What it could not verify
- Why it made each decision

This is not debugging. This is the normal experience.

### 2.5 Sovereign

**ZylCode must be fully functional without the internet.**

Cloud services are optional acceleration, not mandatory infrastructure.
Local models, local tools, local execution, local storage, local
verification — all must work.

A developer on an airplane, in a secure facility, or in a region with
unreliable connectivity must be able to use ZylCode productively.

### 2.6 Truthful

**ZylCode does not advertise capabilities it does not have.**

Documentation describes what exists, not what is planned. Capability
registries reflect runtime reality. Benchmark results are reproducible.
A PARTIAL capability is marked PARTIAL, not promoted to GREEN because
the concept is sound.

### 2.7 Open

**ZylCode is open-source.**

The core engine, the plugin system, the marketplace protocol, the
evidence format, the intelligence graph schema, and the verification
interfaces are all open.

This is not a philosophical position. It is a practical one: an
evidence-first system that cannot be audited is not evidence-first.

### 2.8 Progressive

**Capabilities unlock progressively, not all at once.**

ZylCode should be useful the moment you install it. Each stage adds
capability without breaking what came before. A user at Stage 3 is not
blocked by Stage 8 being incomplete.

---

## 3. The Creation Workspace

### 3.1 Workspace Zones

The workspace is divided into zones, each of which is a first-class
panel that can be opened, closed, resized, docked, and tabbed.

| Zone | Purpose |
|------|---------|
| **Code** | Professional source editor with syntax highlighting, IntelliSense, multi-cursor, refactoring, code lens, minimap |
| **Canvas** | Visual design surface for UI/UX — frames, components, layout, tokens, responsive preview |
| **Preview** | Live runtime preview — web, desktop, mobile — with hot reload |
| **Terminal** | Integrated terminal with multiple sessions, command history, output capture |
| **Test** | Test explorer, test results, coverage visualization, failure navigation |
| **Git** | Source control — commits, branches, diffs, merge, history, blame |
| **Ship** | Release pipeline — build status, CI results, deployment targets, installer generation |
| **Agent** | Agent conversation, trajectory viewer, evidence inspector, approval queue |
| **Trajectory** | Full agent action timeline — every decision, tool call, verification, and evidence item |

### 3.2 Agent-Addressable Context

Every zone produces structured context that agents can consume:

- Selecting a file → file path, language, symbols, content
- Selecting a test → test name, status, failure message, stack trace
- Selecting a UI component → component name, props, visual state, screenshot
- Selecting a Git diff → changed files, hunks, blame, related tests
- Selecting a terminal error → error text, exit code, command history
- Selecting a build failure → build log, affected modules, dependency chain
- Selecting a deployment target → platform, status, logs, version

The agent does not need to ask "what are you looking at?" The workspace
tells it.

### 3.3 Command Palette

The command palette is the primary keyboard-driven interface. It
exposes:

- All workspace commands
- All agent commands
- All installed plugin/skill commands
- All marketplace commands
- Context-sensitive actions based on current selection

---

## 4. The Agent Kernel

### 4.1 Purpose

The Agent Kernel is the runtime that executes AI-driven software
creation tasks. It manages the lifecycle of agents, the routing of
model requests, the execution of tools, and the collection of evidence.

### 4.2 Composable Capabilities

Everything in the Agent Kernel is a composable capability:

| Capability | Description |
|------------|-------------|
| **Model** | An LLM provider (OpenAI, Anthropic, DeepSeek, Gemini, Ollama, etc.) |
| **Agent** | A configured AI entity with persona, tools, and objectives |
| **Tool** | A discrete action the agent can take (file read, shell command, git operation, etc.) |
| **MCP Server** | A Model Context Protocol server providing tools and resources |
| **Skill** | A reusable knowledge package that teaches the agent a domain |
| **Plugin** | A runtime extension that adds services, events, UI, or tools |
| **Workflow** | A multi-step, multi-agent orchestration with phases and gates |
| **Sandbox** | An isolated execution environment with controlled permissions |
| **Memory** | A persistence provider for agent context, session history, and learned facts |
| **Context Provider** | A source of structured context (repository intelligence, design system, etc.) |
| **Design Tool** | A visual creation/manipulation tool (canvas, token editor, component builder) |
| **Device Tool** | A runtime control tool (browser, emulator, device, screenshot) |
| **Verification Provider** | A test/inspection/audit tool that produces evidence |
| **Deployment Provider** | A release/distribution target (app store, cloud, CDN, installer) |

### 4.3 Evidence Ledger

Every consequential action in the Agent Kernel is recorded in the
Evidence Ledger.

The ledger is:
- Append-first
- Hash-chained (SHA-256)
- Idempotent
- Fail-closed
- Crash-recoverable

The ledger is the foundation of ZylCode's trust model. It is the
reason a user can say "prove it" and get an answer.

### 4.4 Session Model

Every agent interaction is a Session. Sessions are:

- Inspectable (full trajectory)
- Resumable (crash recovery)
- Forkable (explore alternative approaches)
- Searchable (find past decisions)
- Auditable (evidence trail)

### 4.5 Approval Model

Actions are classified by risk:

| Risk Level | Policy |
|------------|--------|
| Read | Auto-approve |
| Write | Auto-approve with evidence |
| Execute | Auto-approve with timeout and evidence |
| Network | Approve on first use per session |
| GitWrite | Approve per action |
| Destructive | Always require explicit approval |
| Release | Always require explicit approval |

---

## 5. Model Democracy

### 5.1 Principle

ZylCode is not tied to one intelligence provider. Models are selected
based on measured performance for the task at hand, not provider
preference or marketing.

### 5.2 Model Router

```
                    MODEL ROUTER

 OpenAI ───────┐
 Anthropic ────┤
 DeepSeek ─────┤
 Gemini ───────┤         ┌──────────────┐
 GLM / Z.AI ───┼────────►│ TASK          │
 OpenRouter ───┤         │ CLASSIFIER    │
 Ollama ───────┤         └──────┬───────┘
 Local Models ─┘                │
                         ┌──────┼──────┐
                         ▼      ▼      ▼
                       Plan   Code   Review
                         │      │      │
                         └──────┼──────┘
                                ▼
                            VERIFY
```

### 5.3 Routing Criteria

Models are selected based on:

| Criterion | Weight | Source |
|-----------|--------|--------|
| Task performance | High | Benchmark results per task type |
| Privacy | High | Local vs. cloud, data retention policy |
| Cost | Medium | Token pricing, rate limits |
| Latency | Medium | Measured response times |
| Availability | Medium | API uptime, fallback chains |

### 5.4 Learning

Over time, ZylCode builds a model performance profile:

- Model A: best at Rust refactoring (measured by test pass rate after edit)
- Model B: best at UI design reasoning (measured by visual QA pass rate)
- Model C: cheapest for repository summarization (measured by cost per quality)
- Model D: adequate for private code with Ollama (measured by task completion)

The router uses these profiles, not hardcoded rules.

### 5.5 Sovereign Fallback

When no cloud model is available, ZylCode falls back to local models
through Ollama. The system degrades gracefully:

| Capability | Cloud | Local |
|------------|-------|-------|
| Code generation | Full | Reduced quality, fully functional |
| Visual reasoning | Full | Basic (no vision models) |
| Repository intelligence | Full | Full (local index) |
| Verification | Full | Full (local execution) |
| Design system | Full | Reduced (simpler suggestions) |

---

## 6. Intelligence Graph

### 6.1 Purpose

The Intelligence Graph is ZylCode's structured understanding of the
software being created. It answers: what exists, what owns what, what
depends on what, what changed, and what is relevant.

### 6.2 Layers

| Layer | Content | Status |
|-------|---------|--------|
| File Inventory | Files, languages, roles, content hashes | Phase 2A ✅ |
| Manifest Intelligence | Workspaces, packages, dependencies, scripts | Phase 2A ✅ |
| Symbol Graph | Definitions, types, visibility, parent relationships | Phase 2A ✅ (DEFINITION_INDEXED) |
| Dependency Graph | Package, module, and external dependencies | Phase 2A ✅ |
| Change Graph | Git history, commits, changed files | Phase 2A ✅ |
| Architectural Fingerprint | Frameworks, tools, patterns with evidence | Phase 2A ✅ |
| Import Graph | File-to-file import relationships | Future |
| Reference Graph | Symbol-to-symbol references and calls | Future |
| Semantic Index | Vector embeddings for semantic retrieval | Future |
| Impact Graph | Change impact analysis with confidence | Future |

### 6.3 Query Interface

The Intelligence Graph exposes a typed query API:

```
repository_summary()
find_symbol(name)
symbol_definition(symbol)
symbols_in_file(path)
package_for_file(path)
dependencies_of(subject)
dependents_of(subject)
entry_points()
test_targets()
build_commands()
recent_changes()
architecture()
relevant_context(task)
```

### 6.4 Persistence and Invalidation

The Intelligence Graph is persisted in SQLite. It survives restarts.
When files change, affected intelligence is invalidated and reindexed.

### 6.5 Provenance

Every fact in the Intelligence Graph has provenance:

| Level | Meaning |
|-------|---------|
| Observed | Directly from the filesystem |
| Parsed | Extracted by parsing structured content |
| Derived | Computed from other facts |
| Inferred | Heuristic guess — lower authority |

---

## 7. Vision Studio

### 7.1 Purpose

Vision Studio is ZylCode's integrated visual design workspace. It is
not a Figma integration. It is a genuine canvas-based design environment
built into ZylCode where the canvas and the production code are two
representations of the same product.

### 7.2 The Core Invariant

**Canvas ↔ Code is bidirectional and lossless.**

Opening `src/components/Button.tsx` and pressing "Open in Canvas"
renders the actual component. Changing `radius: 8 → 14` on the canvas
changes the source. Editing the source updates the canvas.

This is not code generation from screenshots. This is two views of
one truth.

### 7.3 Canvas Capabilities

| Category | Capabilities |
|----------|-------------|
| Layout | Frames, Auto-layout, Flex, Grid, Constraints, Responsive breakpoints |
| Content | Text, Shapes, Images, SVG, Icons |
| Components | Component definition, Variants, Instances, Overrides |
| Design System | Tokens (color, typography, spacing, radius, shadow, motion), Variables, Themes |
| Interaction | Prototype links, Transitions, Animations |
| Collaboration | Comments, Annotations, Cursors |
| Accessibility | Contrast checking, Screen reader preview, ARIA inspection |
| Export | Code export, Asset export, Design spec export |

### 7.4 Design System Intelligence

ZylCode maintains a structured design system:

```
Tokens
├── colors
├── typography
├── spacing
├── radius
├── shadows
├── motion
└── breakpoints

Components
├── Button
├── Input
├── Card
├── Dialog
├── Navigation
└── ...

Themes
├── Light
├── Dark
├── Brand A
└── Brand B
```

The AI understands the system. When told "make this dashboard more
professional," it modifies compositions using existing tokens rather
than inventing random colors and spacing.

### 7.5 Technology Approach

Vision Studio is built on:
- **Canvas rendering**: WebGL/WebGPU for high-performance canvas
- **Layout engine**: Flexbox/Grid model matching CSS specification
- **Component model**: React component tree as the source of truth
- **Token system**: JSON/YAML design tokens synced to CSS variables
- **Code bridge**: AST-aware bidirectional sync between canvas and code

The canvas is not a separate application. It is a view mode of the
same codebase.

---

## 8. Execution Engine

### 8.1 Purpose

The Execution Engine runs software and lets agents see and interact
with the results. This is what makes ZylCode an autonomous engineer
rather than a code generator.

### 8.2 Principle

**The agent must look at what it builds.**

The agent does not say "I implemented the page." It:
1. Builds the code
2. Launches the runtime
3. Renders the result
4. Takes a screenshot
5. Inspects the screenshot with vision
6. Compares against the design
7. Repairs if needed
8. Repeats until verified

### 8.3 Runtime Targets

| Target | Status | Control Method |
|--------|--------|---------------|
| Web (Chrome) | Stage 5 | Chrome DevTools Protocol |
| Web (Firefox) | Stage 5 | Marionette |
| Web (Edge) | Stage 5 | Chrome DevTools Protocol |
| Desktop (Windows) | Stage 5 | Tauri runtime + window control |
| Desktop (Linux) | Stage 5 | Tauri runtime + window control |
| Desktop (macOS) | Stage 5 | Tauri runtime + window control |
| Android Emulator | Stage 8 | ADB + UI Automator |
| Android Device | Stage 8 | ADB + UI Automator |
| iOS Simulator | Stage 9 | xcrun simctl + XCUITest |
| iOS Device | Stage 9 | libimobiledevice + XCUITest |
| Remote macOS | Stage 9 | SSH + remote worker |

### 8.4 Inspection Pipeline

```
Code → Build → Launch → Render → Screenshot → Vision → Compare → Repair → Repeat
```

Each step produces evidence. The full pipeline is recorded in the
Evidence Ledger.

### 8.5 Sandbox Model

All execution happens in sandboxes with controlled permissions:

| Sandbox | Purpose | Isolation |
|---------|---------|-----------|
| File Sandbox | File read/write | Path allowlist |
| Shell Sandbox | Command execution | Command allowlist |
| Network Sandbox | HTTP requests | Domain allowlist |
| Browser Sandbox | Web page control | Origin isolation |
| Device Sandbox | Device interaction | ADB/simulator isolation |

---

## 9. Proof Engine

### 9.1 Purpose

The Proof Engine verifies that software works. Not just that tests
pass — that the software actually does what was asked, looks correct,
performs adequately, and is secure.

### 9.2 Verification Layers

| Layer | What It Verifies | Evidence |
|-------|-----------------|----------|
| **Build** | Code compiles, bundles, and produces artifacts | Build log, exit code, artifact hashes |
| **Unit Test** | Individual functions and modules work correctly | Test results, coverage report |
| **Integration Test** | Components work together | Test results, API responses |
| **Visual QA** | UI renders correctly against design | Screenshot comparison, diff image |
| **Runtime QA** | Application behaves correctly in runtime | Interaction traces, console logs, network traces |
| **Security** | No known vulnerabilities, secrets, or injection vectors | Audit results, dependency scan |
| **Performance** | Adequate response times and resource usage | Profiling results, benchmarks |
| **Accessibility** | WCAG compliance, screen reader compatibility | Automated a11y audit results |
| **Evidence** | All above have verifiable evidence | Ledger entries, hash chain |

### 9.3 Fail-Closed Enforcement

Every verification layer follows the same rule:

- No checks provided → FAILED
- Checks provided but not executed → FAILED
- Unexpected result → FAILED
- Verification infrastructure error → FAILED
- Missing evidence → UNVERIFIED

A task is not complete until all required verification layers pass
with evidence.

---

## 10. Delivery Engine

### 10.1 Purpose

The Delivery Engine takes verified software and produces distributable
artifacts — installers, packages, containers, app store submissions,
cloud deployments.

### 10.2 Delivery Targets

| Target | Status | Method |
|--------|--------|--------|
| Git commit/push | Stage 1 ✅ | Git CLI |
| GitHub CI | Stage 1 (BLOCKED_EXTERNAL) | GitHub Actions |
| Windows installer | Stage 11 | NSIS via Tauri |
| macOS installer | Stage 11 | DMG via Tauri |
| Linux packages | Stage 11 | DEB/RPM via Tauri |
| Web deployment | Stage 11 | Static hosting, Docker |
| Android APK/AAB | Stage 11 | Gradle + signing |
| iOS IPA | Stage 11 | Xcode + signing |
| App stores | Stage 12 | Store-specific workflows |

### 10.3 Release Pipeline

```
Verify → Package → Sign → Test → Stage → Deploy → Monitor → Rollback
```

Every step is recorded in the Evidence Ledger.

---

## 11. The Plugin System

### 11.1 Philosophy

Everything in ZylCode is a plugin. This is not an afterthought — it is
the architecture.

The core engine provides:
- A composition runtime
- A service registry
- An event bus
- A plugin lifecycle manager

Everything else — models, tools, skills, MCP servers, agents,
workflows, design tools, device tools, verification providers,
deployment targets — is a plugin that registers with the runtime.

### 11.2 Plugin Contract

Every plugin declares:

| Field | Purpose |
|-------|---------|
| `id` | Unique identifier |
| `version` | Semantic version |
| `capabilities` | What the plugin provides (services, tools, events, UI) |
| `dependencies` | What the plugin requires (other plugins, services) |
| `permissions` | What the plugin needs access to (filesystem, network, devices) |
| `sandbox` | Isolation requirements |

### 11.3 Plugin Types

| Type | Provides | Examples |
|------|----------|---------|
| Model Provider | LLM inference | OpenAI, Anthropic, Ollama |
| Tool Provider | Discrete actions | Filesystem, Shell, Git, Browser |
| MCP Server | Tools + resources via MCP protocol | Any MCP-compliant server |
| Skill | Domain knowledge + behavior patterns | Rust engineering, React UI, Android development |
| Agent | Configured AI entity with persona and objectives | Code reviewer, Test writer, Security auditor |
| Workflow | Multi-step orchestration | Build pipeline, Release process, Migration |
| Context Provider | Structured context for agents | Repository Intelligence, Design System |
| Design Tool | Visual creation/manipulation | Canvas, Token editor, Component builder |
| Device Tool | Runtime control | Browser controller, ADB controller, Simulator |
| Verification Provider | Evidence-producing verification | Test runner, Screenshot comparator, Security scanner |
| Deployment Provider | Release target | App store, Cloud, CDN, Installer |
| UI Plugin | Workspace UI extension | Panel, View, Status bar item, Command |

### 11.4 Creator Studio

ZylCode includes a graphical environment for creating plugins:

- Visual plugin builder
- Skill authoring wizard
- Workflow designer
- Tool definition editor
- MCP server scaffolder
- Theme designer
- Design system builder
- Template creator

This is not a secondary feature. It is the mechanism by which the
ecosystem grows.

---

## 12. Marketplace

### 12.1 Unified Marketplace

One marketplace, not separate stores:

```
ZylCode Marketplace

├── Models
├── MCP Servers
├── Plugins
├── Skills
├── Agents
├── Workflows
├── Themes
├── Design Systems
├── Templates
├── Toolchains
└── Device Adapters
```

### 12.2 Composite Packages

A single marketplace item can bundle multiple capability types:

**Example: "Android Production Engineer"**

```
Android Production Engineer
├── Skill: Android development patterns
├── Tool: ADB controller
├── Tool: Gradle build tool
├── Tool: Emulator controller
├── Workflow: Play Store release
├── Verification: Android test rules
└── Context: Android project structure intelligence
```

**Example: "Mechanical Engineering UI Kit"**

```
Mechanical Engineering UI Kit
├── Design System: Tokens, components, themes
├── Templates: Dashboard, Report, Form
├── Icons: Engineering icon set
├── Skill: Domain-specific AI suggestions
└── Theme: Professional dark/light modes
```

### 12.3 Trust Model

Every marketplace item has:
- Publisher identity
- Version history
- Permission declarations
- Community ratings
- Security audit status
- Evidence of functionality (benchmark results, test coverage)

---

## 13. Autonomous Teams

### 13.1 Vision

Eventually, a user creates a mission:

> "Build an invoicing SaaS for Nigerian SMEs."

ZylCode forms an engineering team:

```
              ORCHESTRATOR

Product Agent ─────┐
Architect Agent ───┤
Designer Agent ────┤
Frontend Agent ────┤
Backend Agent ─────┼──► SHARED WORLD STATE
Test Agent ────────┤
Security Agent ────┤
Reviewer Agent ────┤
Release Agent ─────┘
```

### 13.2 Coordination

Agents cannot all blindly edit files. Coordination is achieved through:

- **Repository Intelligence** — agents understand the codebase structure
- **Task ownership** — each agent owns specific files/modules
- **Git worktrees/branches** — agents work in isolated branches
- **Evidence Ledger** — all actions are recorded and auditable
- **Approval gates** — destructive/release actions require approval
- **Shared world state** — agents see each other's changes through the Intelligence Graph

### 13.3 Quality Gate

No agent's work merges to main until:
- All tests pass
- Visual QA passes
- Security scan passes
- Code review (by Reviewer Agent) passes
- Evidence trail is complete

---

## 14. Roadmap

### 14.1 Stage Overview

| Stage | Name | Objective | Status |
|-------|------|-----------|--------|
| 1 | Truth | Tool runtime, agent loop, evidence, memory, crash recovery | ✅ COMPLETE |
| 2 | Understanding | Repository Intelligence Graph | ✅ COMPLETE (Phase 2A) |
| 3 | Intelligence | Model routing, context optimization, model benchmarking | NEXT |
| 4 | Extensibility | Plugin/skill/MCP/agent/workflow runtime | Future |
| 5 | Execution | Browser + desktop runtime inspection | Future |
| 6 | Vision | Integrated Figma/Penpot-class Vision Studio | Future |
| 7 | Visual Intelligence | Design↔code sync + screenshot inspection + repair | Future |
| 8 | Mobile | Android Device Lab | Future |
| 9 | Apple | Remote macOS/iOS Device Lab | Future |
| 10 | Teams | Multi-agent engineering organization | Future |
| 11 | Delivery | CI/CD, installers, cloud deployment, stores | Future |
| 12 | Ecosystem | Marketplace + Creator Studio | Future |
| 13 | Benchmark | Public reproducible autonomous-engineering benchmark | Future |
| 14 | Launch | Developer preview → Alpha → Beta → Stable | Future |

### 14.2 Stage Boundaries

Each stage has:
- **Input**: What must exist before the stage begins
- **Output**: What must be delivered when the stage completes
- **Gates**: Verification criteria that must pass
- **Dependencies**: Which other stages this stage requires
- **Blocks**: Which stages cannot begin until this stage completes

### 14.3 Parallel Streams

Some stages can proceed in parallel:

```
Stage 1 ──► Stage 2 ──► Stage 3 ──► Stage 4 ──► Stage 5
                                    │
                                    ├──► Stage 6 ──► Stage 7
                                    │
                                    └──► Stage 8 ──► Stage 9

Stage 10 depends on Stages 4 + 5
Stage 11 depends on Stages 5 + 8
Stage 12 depends on Stage 4
Stage 13 depends on Stages 1-12
Stage 14 depends on Stage 13
```

---

## 15. The Demo

### 15.1 The 60-Second Moment

The demonstration that defines ZylCode:

> User: "Build me a task management app with drag-and-drop, dark mode,
> and mobile responsive layout."

ZylCode:

1. **Plans** — generates architecture with component hierarchy
2. **Designs** — creates UI on canvas using design system tokens
3. **Codes** — implements components, state management, API layer
4. **Builds** — compiles and bundles
5. **Launches** — opens in browser preview
6. **Sees** — takes screenshot, inspects with vision model
7. **Repairs** — fixes layout issues detected by visual comparison
8. **Tests** — runs unit and integration tests
9. **Mobile** — previews on mobile viewport, adjusts responsive layout
10. **Packages** — produces web build + optional desktop installer
11. **Evidence** — shows full trajectory with every decision and verification

The user clicks **Trajectory** and sees the complete evidence trail.

### 15.2 What Makes It a Moment

Not the individual capabilities — many tools can generate code, run
tests, or take screenshots.

What makes it a moment is:

1. **One system** — not five tools duct-taped together
2. **Evidence** — every step is provable, not claimed
3. **Visual verification** — the agent actually looked at what it built
4. **Reproducibility** — the same prompt produces verifiably correct output
5. **Transparency** — the user can see exactly what happened and why

---

## 16. Governance

### 16.1 Constitution Authority

This Constitution is the highest authority for ZylCode development.

- Implementation must conform to this Constitution
- Agent instructions must reference this Constitution
- Capability claims must be verifiable against this Constitution
- Documentation must reflect this Constitution

### 16.2 Amendment Process

Amendments to this Constitution require:
1. Proposed change with rationale
2. Impact analysis on existing stages
3. Explicit recording of: what changed, why, when, and who approved
4. No silent rewrites

### 16.3 Audit

Every implementation phase must be auditable against:
- The principles in Section 2
- The architecture in this document
- The stage boundaries in Section 14
- The verification requirements in Section 9

---

## 17. Conclusion

ZylCode is not an IDE with AI features. It is not a chatbot that edits
files. It is not a wrapper around a model provider.

ZylCode is a Software Creation Operating System.

Its purpose is to give a human — or a team of humans and AI agents —
a single place where ideas become verified, shipped software.

Every architectural decision serves that purpose. Every engine exists
to fulfill that mission. Every principle ensures that what is built is
trustworthy.

**Design it. Build it. Run it. See it. Verify it. Ship it.**

---

*ZylCode Product Constitution v2.0*
*Author: Zylvex Tech*
*Date: 2026-09-15*
*Status: DRAFT — pending review*
