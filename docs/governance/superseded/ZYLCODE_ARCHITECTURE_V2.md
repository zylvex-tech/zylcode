> # ⚠️ SUPERSEDED — DO NOT IMPLEMENT FROM THIS DOCUMENT
>
> **Status:** SUPERSEDED, never committed, archived 2026-09-16.
> **Superseded by:** `docs/governance/ZYLCODE_ARCHITECTURE_V2.md`
>
> **Why it was superseded:** two reasons.
> 1. It describes **six** engines, omitting the **Project System**.
> 2. It contains **no `PROPOSED` markers** — it describes systems that do not exist without
>    distinguishing them from systems that do. Per Constitution §9.3 that is prohibited.
>
> Retained for history only. **Do not implement from this document.**

---

# ZylCode Architecture v2.0

> Technical specification for the six-engine Software Creation OS.
> Companion to ZylCode Product Constitution v2.0.

---

## 0. Scope

This document defines the technical architecture of ZylCode. It covers:

1. The system topology
2. Each of the six engines in detail
3. The plugin/composition runtime
4. The model democracy subsystem
5. The evidence and trust model
6. The sovereign/local-first architecture
7. The execution sandbox model
8. The vision studio architecture
9. The autonomous teams coordination model
10. Cross-cutting concerns: persistence, networking, IPC, security

This is the governing technical specification. All implementation must
conform to it unless a Constitution amendment is recorded.

---

## 1. System Topology

### 1.1 Process Model

ZylCode runs as a multi-process system:

```
┌─────────────────────────────────────────────────────────┐
│                    ZYLCODE HOST PROCESS                  │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Agent        │  │ Intelligence│  │ Plugin          │  │
│  │ Kernel       │  │ Graph       │  │ Runtime         │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │           │
│  ┌──────┴────────────────┴───────────────────┴────────┐  │
│  │              SERVICE REGISTRY                       │  │
│  │  Models · Tools · Skills · MCP · Memory · Context   │  │
│  │  Verification · Execution · Delivery · Vision       │  │
│  └──────────────────────┬──────────────────────────────┘  │
│                         │                                 │
│  ┌──────────────────────┴──────────────────────────────┐  │
│  │              EVIDENCE LEDGER                         │  │
│  │  Append-first · Hash-chained · Fail-closed          │  │
│  └─────────────────────────────────────────────────────┘  │
└─────────────────────────┬───────────────────────────────┘
                          │ IPC
┌─────────────────────────┴───────────────────────────────┐
│                    CREATION WORKSPACE                     │
│                    (Browser / WebView)                     │
│                                                         │
│  ┌──────┐ ┌──────┐ ┌───────┐ ┌──────┐ ┌──────┐ ┌─────┐  │
│  │Code  │ │Canvas│ │Preview│ │Term  │ │Test  │ │Agent│  │
│  │Editor│ │      │ │       │ │      │ │      │ │     │  │
│  └──────┘ └──────┘ └───────┘ └──────┘ └──────┘ └─────┘  │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Crate Topology (Current + Planned)

```
crates/
├── zylcode-core          # Agent kernel, evidence, intelligence, verification
│   ├── src/
│   │   ├── agent.rs              # Agent loop (Stage 1)
│   │   ├── agent_protocol.rs     # Agent decision types (Stage 1)
│   │   ├── ledger.rs             # Evidence ledger types (Stage 1)
│   │   ├── sqlite_ledger.rs      # SQLite ledger store (Stage 1)
│   │   ├── memory_ledger.rs      # In-memory ledger for tests (Stage 1)
│   │   ├── intelligence/         # Repository Intelligence (Stage 2)
│   │   ├── models/               # Model routing (Stage 3) [PLANNED]
│   │   ├── verification/         # Proof engine (Stage 5+) [PLANNED]
│   │   ├── execution/            # Execution engine (Stage 5) [PLANNED]
│   │   ├── vision/               # Vision engine (Stage 6+) [PLANNED]
│   │   └── delivery/             # Delivery engine (Stage 11) [PLANNED]
│   └── tests/
├── zylcode-mcp           # MCP server, tools, plugins, telemetry
├── zylcode-cli           # CLI interface
└── zylcode-desktop       # Tauri desktop shell
apps/
└── zylcode-desktop/
    └── src-tauri/        # Tauri backend
    └── src/              # React frontend
```

### 1.3 Workspace Layout

```
packages/
├── zylcode-plugin-runtime    # Plugin composition engine (Stage 4) [PLANNED]
├── zylcode-vision-canvas     # Vision Studio canvas (Stage 6) [PLANNED]
├── zylcode-device-lab        # Device control (Stage 8+) [PLANNED]
├── zylcode-marketplace       # Marketplace client (Stage 12) [PLANNED]
└── zylcode-shared            # Shared types, utilities
```

---

## 2. Engine 1: Agent Kernel

### 2.1 Purpose

The Agent Kernel is the central execution runtime for all AI-driven
software creation tasks. It manages the agent lifecycle, model routing,
tool execution, evidence collection, and session state.

### 2.2 Architecture

```
┌──────────────────────────────────────────────────────┐
│                   AGENT KERNEL                        │
│                                                      │
│  ┌──────────────┐     ┌──────────────────────────┐   │
│  │ Session       │     │ Model Router              │   │
│  │ Manager       │     │                           │   │
│  │               │     │ ┌───────┐ ┌───────┐       │   │
│  │ • Create      │────►│ │Task   │→│Model  │       │   │
│  │ • Resume      │     │ │Classi-│ │Select │       │   │
│  │ • Fork        │     │ │fier   │ │       │       │   │
│  │ • Inspect     │     │ └───────┘ └───────┘       │   │
│  │ • Replay      │     └──────────────────────────┘   │
│  └──────────────┘                                     │
│         │                                             │
│  ┌──────┴────────────────────────────────────────┐    │
│  │              AGENT LOOP                        │    │
│  │                                               │    │
│  │  Observe → Think → Plan → Act → Verify → ...  │    │
│  │                                               │    │
│  │  Decision Types:                              │    │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────────┐  │    │
│  │  │Think │ │Plan  │ │Tool  │ │RequestApproval│  │    │
│  │  └──────┘ └──────┘ └──────┘ └──────────────┘  │    │
│  │  ┌──────┐ ┌──────┐ ┌──────┐                   │    │
│  │  │Verify│ │Compl.│ │Fail  │                   │    │
│  │  └──────┘ └──────┘ └──────┘                   │    │
│  └───────────────────────────────────────────────┘    │
│         │                                             │
│  ┌──────┴────────────────────────────────────────┐    │
│  │              TOOL EXECUTOR                     │    │
│  │                                               │    │
│  │  Tool Registry                                │    │
│  │  ├── Filesystem tools                         │    │
│  │  ├── Shell tools                              │    │
│  │  ├── Git tools                                │    │
│  │  ├── Build tools                              │    │
│  │  ├── Test tools                               │    │
│  │  ├── Browser tools          (Stage 5)         │    │
│  │  ├── Device tools           (Stage 8+)        │    │
│  │  ├── Vision tools           (Stage 6+)        │    │
│  │  ├── Design tools           (Stage 6)         │    │
│  │  ├── Deployment tools       (Stage 11)        │    │
│  │  └── Plugin-contributed tools                 │    │
│  └───────────────────────────────────────────────┘    │
│         │                                             │
│  ┌──────┴────────────────────────────────────────┐    │
│  │              EVIDENCE COLLECTOR                │    │
│  │                                               │    │
│  │  Every action → Ledger entry                  │    │
│  │  Every decision → Record                      │    │
│  │  Every verification → Proof                   │    │
│  └───────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────┘
```

### 2.3 Agent Loop

The agent loop follows the protocol defined in `agent_protocol.rs`:

```
OBSERVE → THINK → PLAN → ACT → VERIFY → COMPLETE/FAIL/LOOP
```

Each cycle:
1. **Observe** — gather context (file state, terminal output, test results,
   screenshots, intelligence graph query)
2. **Think** — internal reasoning about current state and next action
3. **Plan** — decompose task into steps with dependencies
4. **Act** — execute one or more tool calls
5. **Verify** — check that action produced expected result
6. **Complete/Fail/Loop** — finish task, report failure, or continue

### 2.4 Session Model

| Field | Type | Purpose |
|-------|------|---------|
| `session_id` | UUID | Unique session identifier |
| `parent_session_id` | UUID? | Parent session for fork/resume |
| `checkpoint_cursor` | String? | Crash recovery cursor into ledger |
| `created_at` | Timestamp | Session creation time |
| `status` | Enum | Active / Paused / Completed / Failed |
| `context` | Object | Session-specific context (repo, design system, etc.) |

Sessions support:
- **Create** — new task session
- **Resume** — continue from checkpoint after crash
- **Fork** — branch session to explore alternative approach
- **Inspect** — read full session trajectory
- **Replay** — re-execute session from any point
- **Search** — find past decisions across sessions

### 2.5 Tool Registry

Tools are registered with metadata:

```json
{
  "tool_id": "file_read",
  "name": "Read File",
  "description": "Read file contents at a path",
  "risk_level": "Read",
  "parameters": {
    "path": { "type": "string", "required": true }
  },
  "returns": {
    "content": { "type": "string" },
    "encoding": { "type": "string" }
  },
  "sandbox": "file",
  "provider": "builtin"
}
```

### 2.6 Existing Implementation (Stage 1)

| Component | File | Status |
|-----------|------|--------|
| Agent loop | `agent.rs` | ✅ Complete |
| Decision types | `agent_protocol.rs` | ✅ Complete |
| Evidence ledger | `ledger.rs` | ✅ Complete |
| SQLite ledger | `sqlite_ledger.rs` | ✅ Complete |
| Crash recovery | `agent.rs` | ✅ Complete |
| Hash chain integrity | `agent.rs` | ✅ Complete |
| Approval model | `agent.rs` | ✅ Complete |
| Risk classification | `agent_protocol.rs` | ✅ Complete |

---

## 3. Engine 2: Intelligence Graph

### 3.1 Purpose

The Intelligence Graph is ZylCode's structured, persistent, queryable
understanding of the software being created. It is the foundation for
all context-aware agent operations.

### 3.2 Architecture

```
┌──────────────────────────────────────────────────────┐
│               INTELLIGENCE GRAPH                     │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │ Scanner       │  │ Manifest     │  │ Symbol     │  │
│  │               │  │ Parser       │  │ Extractor  │  │
│  │ Files         │  │              │  │            │  │
│  │ Languages     │  │ Workspaces   │  │ Definitions│  │
│  │ Roles         │  │ Packages     │  │ Types      │  │
│  │ Hashes        │  │ Dependencies │  │ Visibility │  │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘  │
│         │                 │                │          │
│  ┌──────┴─────────────────┴────────────────┴──────┐   │
│  │              UNIFICATION LAYER                   │   │
│  │                                                 │   │
│  │  Repository Model                               │   │
│  │  ├── FileNode (language, role, hash)            │   │
│  │  ├── Package (name, deps, build/test commands)  │   │
│  │  ├── Symbol (kind, visibility, parent)          │   │
│  │  ├── Dependency (from, to, kind)                │   │
│  │  ├── EntryPoint (kind, evidence)                │   │
│  │  ├── ArchitecturalFact (fact, evidence)         │   │
│  │  └── GitCommit (sha, files, message)            │   │
│  └────────────────────────┬────────────────────────┘   │
│                           │                            │
│  ┌────────────────────────┴────────────────────────┐   │
│  │              QUERY API                           │   │
│  │                                                 │   │
│  │  repository_summary()                           │   │
│  │  find_symbol(name)                              │   │
│  │  symbol_definition(symbol)                      │   │
│  │  symbols_in_file(path)                          │   │
│  │  package_for_file(path)                         │   │
│  │  dependencies_of(subject)                       │   │
│  │  dependents_of(subject)                         │   │
│  │  entry_points()                                 │   │
│  │  test_targets()                                 │   │
│  │  build_commands()                               │   │
│  │  recent_changes()                               │   │
│  │  architecture()                                 │   │
│  │  relevant_context(task)                         │   │
│  └────────────────────────┬────────────────────────┘   │
│                           │                            │
│  ┌────────────────────────┴────────────────────────┐   │
│  │              CONTEXT RETRIEVER                   │   │
│  │                                                 │   │
│  │  Multi-signal ranking:                          │   │
│  │  ├── Symbol name match                          │   │
│  │  ├── Filename match                             │   │
│  │  ├── Dependency distance                        │   │
│  │  ├── Test relationship                          │   │
│  │  └── Recent-change relationship                 │   │
│  │                                                 │   │
│  │  Token-aware assembly with priority:            │   │
│  │  HIGH (direct match) → MEDIUM (related) → LOW   │   │
│  └─────────────────────────────────────────────────┘   │
│                                                      │
│  ┌─────────────────────────────────────────────────┐   │
│  │              PERSISTENCE                         │   │
│  │                                                 │   │
│  │  SQLite store with tables for:                  │   │
│  │  files, symbols, packages, dependencies,        │   │
│  │  entry_points, git_commits, commit_files,       │   │
│  │  repo_facts, architectural_facts, repo_metadata │   │
│  └─────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

### 3.3 Intelligence Layers

| Layer | Data | Quality | Stage |
|-------|------|---------|-------|
| File Inventory | files, languages, roles, hashes | COMPLETE | 2 ✅ |
| Manifest Intelligence | workspaces, packages, deps | COMPLETE | 2 ✅ |
| Symbol Graph | definitions, types, visibility | DEFINITION_INDEXED | 2 ✅ |
| Dependency Graph | package, module, external | COMPLETE | 2 ✅ |
| Change Graph | commits, files, diffs | COMPLETE | 2 ✅ |
| Architecture Fingerprint | frameworks, tools, patterns | COMPLETE | 2 ✅ |
| Import Graph | file→file imports | PLANNED | 3+ |
| Reference Graph | symbol→symbol calls/uses | PLANNED | 3+ |
| Semantic Index | vector embeddings | PLANNED | 3+ |
| Impact Analysis | change impact with confidence | PLANNED | 4+ |

### 3.4 Provenance Model

Every fact has provenance:

```json
{
  "fact": "package zylcode-core depends on rusqlite",
  "provenance": "PARSED",
  "source": "crates/zylcode-core/Cargo.toml",
  "confidence": 1.0,
  "timestamp": "2026-09-15T10:30:00Z"
}
```

| Level | Meaning | Authority |
|-------|---------|-----------|
| OBSERVED | Direct filesystem observation | Highest |
| Parsed | Extracted by structured parser | High |
| Derived | Computed from other facts | Medium |
| Inferred | Heuristic guess | Low |

### 3.5 Benchmark

The Intelligence Graph includes a commissioning benchmark:

- 10 known-answer questions against the ZylCode repository
- Precision@K and Recall@K metrics
- Measured at each stage to track improvement
- Current results: P@10=0.30, R@10=0.53

### 3.6 Existing Implementation

| Component | File | Status |
|-----------|------|--------|
| Module root | `intelligence/mod.rs` | ✅ Complete |
| Types | `intelligence/types.rs` | ✅ Complete |
| Classifier | `intelligence/classifier.rs` | ✅ Complete |
| Scanner | `intelligence/scanner.rs` | ✅ Complete |
| Manifest parser | `intelligence/manifest.rs` | ✅ Complete |
| Symbol extractor | `intelligence/symbols.rs` | ✅ Complete |
| Dependency graph | `intelligence/dependency.rs` | ✅ Complete |
| Entry points | `intelligence/entry_points.rs` | ✅ Complete |
| Architecture | `intelligence/architecture.rs` | ✅ Complete |
| Git intelligence | `intelligence/git.rs` | ✅ Complete |
| SQLite store | `intelligence/store.rs` | ✅ Complete |
| Query API | `intelligence/query.rs` | ✅ Complete |
| Context retriever | `intelligence/context.rs` | ✅ Complete |
| Benchmark | `tests/repo_intelligence_benchmark.rs` | ✅ Complete |

---

## 4. Engine 3: Execution Engine

### 4.1 Purpose

The Execution Engine runs software in controlled environments and lets
agents observe and interact with the results. This is what makes ZylCode
an autonomous engineer rather than a code generator.

### 4.2 Core Invariant

**The agent must look at what it builds.**

The execution pipeline:

```
Code → Build → Launch → Render → Screenshot → Inspect → Repair → Verify
```

Every step produces evidence. The agent cannot claim success without
runtime verification.

### 4.3 Architecture

```
┌──────────────────────────────────────────────────────┐
│               EXECUTION ENGINE                        │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │ Build         │  │ Launch        │  │ Inspect    │  │
│  │ Controller    │  │ Controller    │  │ Controller │  │
│  │               │  │               │  │            │  │
│  │ cargo build   │  │ Chrome CDP    │  │ Screenshot │  │
│  │ vite build    │  │ Tauri launch  │  │ DOM dump   │  │
│  │ gradle build  │  │ ADB launch    │  │ Console    │  │
│  │ xcodebuild    │  │ xcrun simctl  │  │ Network    │  │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘  │
│         │                 │                │          │
│  ┌──────┴─────────────────┴────────────────┴──────┐   │
│  │              RUNTIME ADAPTERS                    │   │
│  │                                                 │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐  │   │
│  │  │ Web    │ │Desktop │ │Android │ │ iOS      │  │   │
│  │  │Adapter │ │Adapter │ │Adapter │ │ Adapter  │  │   │
│  │  │        │ │        │ │        │ │          │  │   │
│  │  │Chrome  │ │Tauri   │ │ADB     │ │xcrun     │  │   │
│  │  │Firefox │ │Windows │ │UIAuto  │ │XCUITest  │  │   │
│  │  │Edge    │ │Linux   │ │Emulator│ │libimob.  │  │   │
│  │  │        │ │macOS   │ │Device  │ │Remote Mac│  │   │
│  │  └────────┘ └────────┘ └────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────┘   │
│                                                      │
│  ┌─────────────────────────────────────────────────┐   │
│  │              SANDBOX MANAGER                     │   │
│  │                                                 │   │
│  │  Process isolation                              │   │
│  │  Network allowlists                             │   │
│  │  Filesystem allowlists                          │   │
│  │  Timeout enforcement                            │   │
│  │  Resource limits (CPU, memory, disk)            │   │
│  └─────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────┘
```

### 4.4 Runtime Adapters

Each runtime target implements the same interface:

```json
{
  "adapter_id": "web-chrome",
  "capabilities": [
    "launch",
    "navigate",
    "screenshot",
    "dom_snapshot",
    "console_capture",
    "network_capture",
    "click",
    "type",
    "evaluate_js",
    "wait_for",
    "close"
  ]
}
```

| Adapter | Stage | Control Method | Capabilities |
|---------|-------|---------------|-------------|
| Web (Chrome) | 5 | Chrome DevTools Protocol | Full browser control |
| Web (Firefox) | 5 | Marionette | Full browser control |
| Web (Edge) | 5 | Chrome DevTools Protocol | Full browser control |
| Desktop (Windows) | 5 | Tauri runtime | Window control, IPC |
| Desktop (Linux) | 5 | Tauri runtime | Window control, IPC |
| Desktop (macOS) | 5 | Tauri runtime | Window control, IPC |
| Android Emulator | 8 | ADB + UI Automator | Device control |
| Android Device | 8 | ADB + UI Automator | Device control |
| iOS Simulator | 9 | xcrun simctl + XCUITest | Device control |
| iOS Device | 9 | libimobiledevice + XCUITest | Device control |
| Remote macOS | 9 | SSH + remote worker | Remote control |

### 4.5 Build Integration

The Execution Engine integrates with build systems:

| Build System | Detection | Build Command | Test Command |
|-------------|-----------|---------------|-------------|
| Cargo | `Cargo.toml` | `cargo build` | `cargo test` |
| npm/pnpm | `package.json` | `npm run build` | `npm test` |
| Gradle | `build.gradle` | `./gradlew build` | `./gradlew test` |
| Xcode | `.xcodeproj` | `xcodebuild` | `xcodebuild test` |
| Vite | `vite.config.*` | `vite build` | `vitest` |

### 4.6 Inspection Pipeline

```
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌───────────┐
│ Build   │───►│ Launch  │───►│ Render   │───►│ Screenshot│
│         │    │         │    │          │    │           │
│ Exit    │    │ Process │    │ DOM      │    │ Image     │
│ code    │    │ running │    │ complete │    │ captured  │
│ Log     │    │ Port    │    │ Network  │    │           │
│ Artifacts│   │ PID     │    │ idle     │    │           │
└─────────┘    └─────────┘    └──────────┘    └─────┬─────┘
                                                     │
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌─────┴─────┐
│ Verify  │◄───│ Repair  │◄───│ Compare  │◄───│ Inspect   │
│         │    │         │    │          │    │           │
│ Tests   │    │ Code    │    │ Visual   │    │ Vision    │
│ pass    │    │ changed │    │ diff     │    │ model     │
│ Perf OK │    │         │    │ Layout   │    │ analysis  │
│ A11y OK │    │         │    │ check    │    │           │
└─────────┘    └─────────┘    └──────────┘    └───────────┘
```

Each step records evidence in the ledger.

---

## 5. Engine 4: Proof Engine

### 5.1 Purpose

The Proof Engine verifies that software works across multiple
dimensions. It is not just a test runner — it is a comprehensive
verification system that produces evidence.

### 5.2 Architecture

```
┌──────────────────────────────────────────────────────┐
│                  PROOF ENGINE                         │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              VERIFICATION PIPELINE               │  │
│  │                                                 │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌───────┐  │  │
│  │  │ Build  │─►│ Unit   │─►│ Integ. │─►│ Visual│  │  │
│  │  │ Verif. │  │ Test   │  │ Test   │  │ QA    │  │  │
│  │  └────────┘  └────────┘  └────────┘  └───────┘  │  │
│  │       │           │          │          │        │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌───────┐  │  │
│  │  │Runtime │─►│Security│─►│ Perf   │─►│ A11y  │  │  │
│  │  │QA      │  │ Scan   │  │ Test   │  │ Check │  │  │
│  │  └────────┘  └────────┘  └────────┘  └───────┘  │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              VERIFICATION PROVIDERS              │  │
│  │                                                 │  │
│  │  Built-in:                                      │  │
│  │  ├── cargo test runner                          │  │
│  │  ├── npm/pnpm test runner                       │  │
│  │  ├── screenshot comparator                      │  │
│  │  ├── console error detector                     │  │
│  │  └── build artifact verifier                    │  │
│  │                                                 │  │
│  │  Plugin-contributed:                            │  │
│  │  ├── Security scanners                          │  │
│  │  ├── Performance profilers                      │  │
│  │  ├── Accessibility auditors                     │  │
│  │  ├── Visual regression tools                    │  │
│  │  └── Custom domain verifiers                    │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              EVIDENCE AGGREGATOR                 │  │
│  │                                                 │  │
│  │  Collects evidence from all verification layers │  │
│  │  Produces: VerificationReport {                 │  │
│  │    layers: [LayerResult],                       │  │
│  │    overall: PASS | FAIL | PARTIAL,              │  │
│  │    evidence: [LedgerEntry],                     │  │
│  │    confidence: float                            │  │
│  │  }                                              │  │
│  └─────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 5.3 Verification Layers

| Layer | Verifies | Evidence | Fail-Closed |
|-------|----------|----------|-------------|
| Build | Compilation, bundling, artifacts | Build log, exit code, artifact hashes | ✅ |
| Unit Test | Function/module correctness | Test results, coverage | ✅ |
| Integration | Component interaction | Test results, API traces | ✅ |
| Visual QA | UI matches design | Screenshot diff, pixel comparison | ✅ |
| Runtime QA | App behaves correctly | Interaction traces, console logs | ✅ |
| Security | No vulnerabilities/secrets | Audit results, dependency scan | ✅ |
| Performance | Adequate response times | Profiling, benchmarks | ✅ |
| Accessibility | WCAG compliance | Automated a11y audit | ✅ |
| Evidence | All above have proof | Ledger entries, hash chain | ✅ |

### 5.4 Visual QA Pipeline

The Visual QA pipeline is the key differentiator:

```
1. Agent implements UI
2. Build succeeds
3. Launch in browser/device
4. Take screenshot
5. Vision model inspects screenshot
   ├── Layout correct?
   ├── Colors match design tokens?
   ├── Text readable?
   ├── Spacing consistent?
   ├── Responsive breakpoints working?
   └── Components rendered?
6. Compare against design (if canvas exists)
   ├── Pixel diff overlay
   ├── Structural diff
   └── Token usage verification
7. Report findings
8. If issues found → repair loop
9. If issues resolved → record evidence
```

### 5.5 Fail-Closed Rules

The Proof Engine follows strict fail-closed rules:

```rust
fn verify(task: &Task) -> VerificationResult {
    match run_checks(task) {
        Ok(results) if results.all_pass() => Verified(results),
        Ok(results) => Failed(results),
        Err(e) => Failed(e),
        // NEVER:
        // - return Verified without evidence
        // - return Verified on missing checks
        // - return Verified on infrastructure error
        // - return Verified with stale data
    }
}
```

---

## 6. Engine 5: Vision Studio

### 6.1 Purpose

Vision Studio is ZylCode's integrated visual design workspace. It
provides a Figma/Penpot-class canvas where the canvas and production
code are two representations of the same product.

### 6.2 Core Invariant

**Canvas ↔ Code is bidirectional and lossless.**

This is not code generation from screenshots. This is two views of
one truth.

### 6.3 Architecture

```
┌──────────────────────────────────────────────────────┐
│                  VISION STUDIO                        │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              CANVAS ENGINE                       │  │
│  │                                                 │  │
│  │  Rendering: WebGL / WebGPU                      │  │
│  │  Layout: Flexbox / Grid (CSS spec)              │  │
│  │  Hit testing: Point-in-element                  │  │
│  │  Transform: Pan, zoom, rotate                   │  │
│  │  Selection: Single, multi, marquee              │  │
│  │  Manipulation: Move, resize, rotate, align      │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              CODE BRIDGE                         │  │
│  │                                                 │  │
│  │  Canvas → Code:                                 │  │
│  │  1. Identify source file from element           │  │
│  │  2. Parse AST                                   │  │
│  │  3. Locate property in AST                      │  │
│  │  4. Update value                                │  │
│  │  5. Write back to file                          │  │
│  │  6. Trigger hot reload                          │  │
│  │                                                 │  │
│  │  Code → Canvas:                                 │  │
│  │  1. Watch source files for changes              │  │
│  │  2. Parse updated AST                           │  │
│  │  3. Extract component tree                      │  │
│  │  4. Render component in canvas                  │  │
│  │  5. Update canvas elements                      │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              DESIGN SYSTEM                       │  │
│  │                                                 │  │
│  │  Tokens:                                        │  │
│  │  ├── colors: { primary: "#2563EB", ... }        │  │
│  │  ├── typography: { heading: { size: 24, ... }}  │  │
│  │  ├── spacing: { xs: 4, sm: 8, md: 16, ... }    │  │
│  │  ├── radius: { sm: 4, md: 8, lg: 16 }          │  │
│  │  ├── shadows: { sm: "0 1px 2px ...", ... }      │  │
│  │  └── motion: { fast: "150ms", normal: "300ms" } │  │
│  │                                                 │  │
│  │  Components:                                    │  │
│  │  ├── Definition (props, variants, slots)        │  │
│  │  ├── Instances (overrides allowed)              │  │
│  │  └── Documentation (usage examples)             │  │
│  │                                                 │  │
│  │  Themes:                                        │  │
│  │  ├── Light / Dark                               │  │
│  │  └── Brand variants                             │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              AI INTEGRATION                      │  │
│  │                                                 │  │
│  │  Agent can:                                     │  │
│  │  ├── Read canvas state as structured context    │  │
│  │  ├── Modify canvas through tool calls           │  │
│  │  ├── Inspect rendered output with vision        │  │
│  │  ├── Compare rendered output against design     │  │
│  │  ├── Generate components from description       │  │
│  │  └── Suggest design improvements                │  │
│  │                                                 │  │
│  │  AI understands:                                │  │
│  │  ├── Design system tokens and constraints       │  │
│  │  ├── Component relationships                    │  │
│  │  ├── Responsive breakpoint behavior             │  │
│  │  └── Accessibility requirements                 │  │
│  └─────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 6.4 Canvas Capabilities

| Category | Capabilities |
|----------|-------------|
| Layout | Frames, Auto-layout, Flex, Grid, Constraints, Responsive breakpoints |
| Content | Text, Shapes, Images, SVG, Icons, Rich text |
| Components | Definition, Variants, Instances, Overrides, Slots |
| Design System | Tokens, Variables, Themes, Typography scale |
| Interaction | Prototype links, Transitions, Animations, Hover states |
| Collaboration | Comments, Annotations, Multi-cursor (future) |
| Accessibility | Contrast checking, Screen reader preview, ARIA labels |
| Export | Code export, Asset export, Design spec, CSS variables |

### 6.5 Code Bridge Protocol

The bidirectional sync between canvas and code:

```
Canvas Edit → Code Update:

1. User drags element to new position
2. Canvas calculates new CSS properties
3. Code Bridge identifies source file + AST node
4. AST modification: update property value
5. File write
6. Hot reload triggers
7. Preview updates
8. Canvas confirms sync

Code Edit → Canvas Update:

1. User edits source file
2. File watcher detects change
3. AST parser extracts component tree
4. Canvas reconciles: match existing elements to new tree
5. Update changed properties
6. Add/remove elements as needed
7. Canvas re-renders
8. Preview updates
```

### 6.6 Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Canvas rendering | WebGL/WebGPU | High-performance 2D rendering |
| Layout engine | Custom Flexbox/Grid | CSS-spec-compliant layout |
| AST parsing | tree-sitter | Language-aware, incremental |
| Component model | React tree | Native React component as source of truth |
| Token system | JSON/YAML → CSS variables | Standards-based, toolable |
| File watching | chokidar / notify | Cross-platform file system events |

---

## 7. Engine 6: Delivery Engine

### 7.1 Purpose

The Delivery Engine takes verified software and produces distributable
artifacts with full evidence trails.

### 7.2 Architecture

```
┌──────────────────────────────────────────────────────┐
│               DELIVERY ENGINE                         │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              BUILD PIPELINE                      │  │
│  │                                                 │  │
│  │  Source → Compile → Bundle → Package → Sign     │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              DISTRIBUTION TARGETS                │  │
│  │                                                 │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌──────────┐  │  │
│  │  │ Git    │ │ Web    │ │Desktop │ │ Mobile   │  │  │
│  │  │        │ │        │ │        │ │          │  │  │
│  │  │Commit  │ │Static  │ │NSIS    │ │APK/AAB   │  │  │
│  │  │Push    │ │Host    │ │DMG     │ │IPA       │  │  │
│  │  │Tag     │ │Docker  │ │DEB/RPM │ │Store     │  │  │
│  │  │Branch  │ │CDN     │ │AppImg  │ │Deploy    │  │  │
│  │  └────────┘ └────────┘ └────────┘ └──────────┘  │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              RELEASE PIPELINE                    │  │
│  │                                                 │  │
│  │  Verify → Package → Sign → Test → Stage →       │  │
│  │  Deploy → Monitor → Rollback                    │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              EVIDENCE                            │  │
│  │                                                 │  │
│  │  Every delivery step is recorded:               │  │
│  │  ├── Build artifacts with hashes                │  │
│  │  ├── Signing certificates                       │  │
│  │  ├── Test results                               │  │
│  │  ├── Deployment confirmation                    │  │
│  │  └── Rollback capability                        │  │
│  └─────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 7.3 Platform-Specific Delivery

| Platform | Format | Signing | Store |
|----------|--------|---------|-------|
| Windows | NSIS installer, MSIX | Code signing certificate | Microsoft Store |
| macOS | DMG, PKG | Apple Developer ID | Mac App Store |
| Linux | DEB, RPM, AppImage, Flatpak | GPG signing | Flathub, Snapcraft |
| Web | Static bundle, Docker | TLS certificate | Any hosting |
| Android | APK, AAB | Keystore | Google Play |
| iOS | IPA | Apple provisioning | App Store |

---

## 8. Plugin System

### 8.1 Purpose

Everything in ZylCode is a plugin. The plugin system is the composition
runtime that makes the system extensible.

### 8.2 Plugin Lifecycle

```
Define → Approve → Activate → Run → Update → Stop → Undefine
```

| State | Description |
|-------|-------------|
| Defined | Plugin source registered, not yet running |
| Awaiting Approval | Plugin needs user consent |
| Activating | Plugin loading in progress |
| Running | Plugin active and contributing |
| Updating | Switching to new version |
| Stopped | Plugin disabled but preserved |
| Removed | Plugin permanently deleted |

### 8.3 Plugin Contract

```json
{
  "id": "plugin-name",
  "version": "1.0.0",
  "description": "What this plugin does",
  "capabilities": {
    "services": ["serviceA", "serviceB"],
    "events": ["eventX"],
    "tools": ["tool1", "tool2"],
    "ui": { "slot": "panel.right" },
    "models": ["provider/model-id"]
  },
  "dependencies": {
    "required": ["other-plugin"],
    "optional": ["optional-plugin"]
  },
  "permissions": {
    "filesystem": ["read:src/**", "write:dist/**"],
    "network": ["api.example.com"],
    "shell": ["npm", "cargo"],
    "devices": ["browser:chrome"]
  },
  "sandbox": {
    "isolation": "process",
    "resourceLimits": {
      "memory": "512MB",
      "cpu": "2 cores",
      "timeout": "300s"
    }
  }
}
```

### 8.4 Plugin Types

| Type | Provides | Registration |
|------|----------|-------------|
| Model Provider | LLM inference | `models.register(id, provider)` |
| Tool Provider | Discrete actions | `tools.register(id, handler)` |
| MCP Server | Tools + resources | `mcp.connect(endpoint)` |
| Skill | Knowledge + patterns | `skills.register(id, skill)` |
| Agent | AI entity | `agents.register(id, config)` |
| Workflow | Orchestration | `workflows.register(id, workflow)` |
| Context Provider | Structured context | `context.register(id, provider)` |
| Design Tool | Visual manipulation | `design.register(id, tool)` |
| Device Tool | Runtime control | `device.register(id, tool)` |
| Verification Provider | Evidence production | `verification.register(id, provider)` |
| Deployment Provider | Release target | `deployment.register(id, provider)` |
| UI Plugin | Workspace extension | `ui.register(slot, component)` |

---

## 9. Model Democracy

### 9.1 Architecture

```
┌──────────────────────────────────────────────────────┐
│               MODEL ROUTER                            │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              PROVIDER REGISTRY                   │  │
│  │                                                 │  │
│  │  OpenAI ──────────┐                             │  │
│  │  Anthropic ───────┤                             │  │
│  │  DeepSeek ────────┤                             │  │
│  │  Gemini ──────────┤                             │  │
│  │  GLM / Z.AI ──────┼──► PROVIDER POOL            │  │
│  │  OpenRouter ──────┤                             │  │
│  │  Ollama ──────────┤                             │  │
│  │  Local models ────┘                             │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              TASK CLASSIFIER                     │  │
│  │                                                 │  │
│  │  Classifies tasks into categories:              │  │
│  │  ├── Code generation                            │  │
│  │  ├── Code review                                │  │
│  │  ├── Planning / architecture                    │  │
│  │  ├── Visual reasoning                           │  │
│  │  ├── Repository summarization                   │  │
│  │  ├── Test writing                               │  │
│  │  ├── Debugging                                  │  │
│  │  ├── Documentation                              │  │
│  │  └── General conversation                       │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              MODEL SELECTOR                      │  │
│  │                                                 │  │
│  │  Selects model based on:                        │  │
│  │  ├── Task category                              │  │
│  │  ├── Measured performance (benchmark history)   │  │
│  │  ├── Privacy requirements                       │  │
│  │  ├── Cost constraints                           │  │
│  │  ├── Latency requirements                       │  │
│  │  └── Availability                               │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              PERFORMANCE TRACKER                 │  │
│  │                                                 │  │
│  │  Records per-task-model outcomes:               │  │
│  │  ├── Test pass rate after code edit              │  │
│  │  ├── Visual QA pass rate after UI change        │  │
│  │  ├── Review accuracy vs human review            │  │
│  │  ├── Planning quality vs outcome                │  │
│  │  ├── Token usage and cost                       │  │
│  │  └── Response latency                           │  │
│  │                                                 │  │
│  │  Model profiles built over time:                │  │
│  │  "Model A: best at Rust refactoring"            │  │
│  │  "Model B: best at visual design reasoning"     │  │
│  │  "Model C: cheapest for summarization"          │  │
│  └─────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 9.2 Routing Algorithm

```
function selectModel(task, context):
    category = classifyTask(task)
    candidates = getModelsForCategory(category)
    
    // Filter by hard constraints
    candidates = filterByPrivacy(candidates, context.privacyRequirements)
    candidates = filterByAvailability(candidates)
    
    // Score by soft criteria
    for model in candidates:
        model.score = 
            performanceWeight * getPerformance(model, category) +
            costWeight * getCostEfficiency(model, task) +
            latencyWeight * getLatency(model)
    
    // Select best, with fallback
    return selectBest(candidates) || getLocalFallback()
```

### 9.3 Sovereign Mode

When cloud is unavailable:

| Capability | Cloud Mode | Sovereign Mode |
|-----------|-----------|---------------|
| Code generation | Full | Reduced quality, fully functional |
| Visual reasoning | Full | Basic (no vision models) |
| Repository intelligence | Full | Full (local index) |
| Build/test verification | Full | Full (local execution) |
| Design system | Full | Reduced (simpler suggestions) |
| Model routing | Full selection | Local models only |

The system never fails due to cloud unavailability. It degrades
gracefully and informs the user of reduced capability.

---

## 10. Autonomous Teams

### 10.1 Architecture

```
┌──────────────────────────────────────────────────────┐
│               ORCHESTRATOR                            │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              MISSION MANAGER                     │  │
│  │                                                 │  │
│  │  Receives: "Build invoicing SaaS for Nigeria"   │  │
│  │  Decomposes: Architecture → Design → Code →     │  │
│  │              Test → Verify → Ship                │  │
│  │  Assigns: Agent roles and responsibilities      │  │
│  │  Monitors: Progress, blockers, quality          │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              AGENT ROLES                         │  │
│  │                                                 │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │  │
│  │  │ Product  │ │Architect │ │ Designer │        │  │
│  │  │ Agent    │ │ Agent    │ │ Agent    │        │  │
│  │  │          │ │          │ │          │        │  │
│  │  │Requirements│ │System  │ │ UI/UX    │        │  │
│  │  │User stories│ │Design  │ │ Tokens   │        │  │
│  │  │Priority  │ │APIs      │ │Components│        │  │
│  │  └──────────┘ └──────────┘ └──────────┘        │  │
│  │                                                 │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │  │
│  │  │ Frontend │ │ Backend  │ │ Test     │        │  │
│  │  │ Agent    │ │ Agent    │ │ Agent    │        │  │
│  │  │          │ │          │ │          │        │  │
│  │  │React/Vite│ │Rust/Node │ │Unit      │        │  │
│  │  │Components│ │APIs      │ │Integration│        │  │
│  │  │Canvas    │ │Database  │ │Visual QA │        │  │
│  │  └──────────┘ └──────────┘ └──────────┘        │  │
│  │                                                 │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐        │  │
│  │  │ Security │ │ Reviewer │ │ Release  │        │  │
│  │  │ Agent    │ │ Agent    │ │ Agent    │        │  │
│  │  │          │ │          │ │          │        │  │
│  │  │Vulnerability│ │Code   │ │Build     │        │  │
│  │  │Audit     │ │review    │ │Package   │        │  │
│  │  │Dependency│ │Quality   │ │Sign      │        │  │
│  │  │scan      │ │gate      │ │Deploy    │        │  │
│  │  └──────────┘ └──────────┘ └──────────┘        │  │
│  └─────────────────────────────────────────────────┘  │
│                                                      │
│  ┌─────────────────────────────────────────────────┐  │
│  │              SHARED WORLD STATE                  │  │
│  │                                                 │  │
│  │  Repository Intelligence Graph (shared)         │  │
│  │  Task ownership map (file → agent)              │  │
│  │  Git branches (one per agent)                   │  │
│  │  Evidence Ledger (shared, append-only)          │  │
│  │  Design System (shared, read-only for most)     │  │
│  │  Build/Test results (shared)                    │  │
│  └─────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 10.2 Coordination Protocol

| Mechanism | Purpose |
|-----------|---------|
| Git branches | Each agent works in its own branch |
| Task ownership | File-level ownership prevents conflicts |
| Intelligence Graph | Agents see each other's structural changes |
| Evidence Ledger | All actions are recorded and auditable |
| Quality gates | No merge without passing verification |
| Handoff protocol | Structured message passing between agents |

### 10.3 Quality Gate

No agent's work merges to main until:

1. All unit tests pass
2. All integration tests pass
3. Visual QA passes (if UI changed)
4. Security scan passes
5. Code review (by Reviewer Agent) passes
6. Evidence trail is complete

---

## 11. Cross-Cutting Concerns

### 11.1 Persistence

| Data | Storage | Lifecycle |
|------|---------|-----------|
| Evidence Ledger | SQLite | Permanent, append-only |
| Intelligence Graph | SQLite | Reindexable, content-hashed |
| Session State | SQLite | Session-scoped, resumable |
| Model Performance | SQLite | Growing over time |
| Design System | JSON/YAML | Version-controlled |
| Plugin Registry | SQLite | Managed by plugin runtime |
| Marketplace Cache | SQLite/FS | Cacheable, refreshable |

### 11.2 Networking

| Connection | Method | Security |
|-----------|--------|----------|
| Model API | HTTPS | TLS 1.3, API key in secure storage |
| MCP Server | stdio / WebSocket | Local or authenticated remote |
| Git Remote | HTTPS / SSH | TLS / SSH key |
| Marketplace | HTTPS | TLS, signed packages |
| Device Control | ADB / USB / SSH | Device-specific auth |
| Browser Control | CDP / WebSocket | Localhost only |

### 11.3 IPC

| Channel | Direction | Format |
|---------|-----------|--------|
| Workspace ↔ Host | Bidirectional | JSON over Tauri IPC |
| Agent ↔ Tools | Agent → Tool | Tool call protocol |
| Agent ↔ Model | Agent → Model → Agent | Chat completion API |
| Plugin ↔ Host | Bidirectional | Plugin API (JSON) |
| Plugin ↔ Plugin | Through Host | Service calls via registry |

### 11.4 Security Model

| Concern | Approach |
|---------|----------|
| Tool permissions | Risk-based approval (Section 4.5 of Constitution) |
| Plugin permissions | Declared in manifest, enforced by sandbox |
| File access | Path allowlists per sandbox |
| Network access | Domain allowlists per sandbox |
| Shell access | Command allowlists per sandbox |
| Secret handling | Never in logs, never in evidence, never in context |
| Model data | Configurable: local-only, encrypted, or cloud |
| Marketplace trust | Signed packages, permission declarations, audit status |

### 11.5 Observability

| Signal | Source | Destination |
|--------|--------|-------------|
| Agent decisions | Agent loop | Evidence Ledger |
| Tool executions | Tool executor | Evidence Ledger |
| Build/test results | Proof Engine | Evidence Ledger |
| Model performance | Model Router | Performance Tracker |
| Plugin lifecycle | Plugin Runtime | Event log |
| Error/failure | All engines | Evidence Ledger + alert |
| User feedback | Workspace | Session state |

---

## 12. Migration from Current State

### 12.1 What Exists Today

| Component | Stage | Status |
|-----------|-------|--------|
| Agent loop | 1 | ✅ Complete |
| Evidence Ledger | 1 | ✅ Complete |
| Crash recovery | 1 | ✅ Complete |
| Repository Intelligence | 2 | ✅ Complete |
| MCP server | 1 | ✅ Complete |
| CLI | 1 | ✅ Complete |
| Desktop shell (Tauri) | 1 | ✅ Complete |
| React frontend | 1 | ✅ Complete |

### 12.2 What Comes Next

Stage 3 (Intelligence) builds on Stage 2 by adding:
- Model routing (new crate/module)
- Context optimization (enhance context.rs)
- Model benchmarking (new test infrastructure)
- Import/reference graph (enhance intelligence/)
- Semantic index (new capability)

### 12.3 Incremental Migration

Each stage adds capability without breaking what came before. The
crate topology grows organically:

```
Stage 1-2: crates/zylcode-core (done)
Stage 3:   crates/zylcode-core/src/models/ (new module)
Stage 4:   packages/zylcode-plugin-runtime (new package)
Stage 5:   crates/zylcode-core/src/execution/ (new module)
Stage 6:   packages/zylcode-vision-canvas (new package)
Stage 7:   Vision + Execution integration
Stage 8:   packages/zylcode-device-lab (new package)
...
```

---

## 13. Benchmark and Measurement

### 13.1 Per-Stage Benchmarks

Each stage has measurable acceptance criteria:

| Stage | Benchmark | Metric | Threshold |
|-------|-----------|--------|-----------|
| 2 | Repository Intelligence | P@10, R@10 | 0.25, 0.30 |
| 3 | Model routing accuracy | Task→model match rate | ≥0.80 |
| 4 | Plugin load time | Time to activate | <500ms |
| 5 | Execution pipeline | Build→screenshot time | <30s |
| 6 | Canvas↔Code sync | Bidirectional fidelity | ≥0.95 |
| 7 | Visual QA accuracy | Defect detection rate | ≥0.80 |
| 8 | Android execution | Build→emulator time | <120s |
| 9 | iOS execution | Build→simulator time | <180s |
| 10 | Multi-agent coordination | Task completion rate | ≥0.70 |
| 11 | Delivery pipeline | Build→installer time | <300s |
| 12 | Marketplace install | Package install time | <10s |
| 13 | Autonomous benchmark | End-to-end task completion | ≥0.60 |

### 13.2 The Autonomous Benchmark

Stage 13 defines a public, reproducible benchmark:

```
AUTONOMOUS ENGINEERING BENCHMARK

Tasks (increasing difficulty):
1. "Fix this bug" (given failing test)
2. "Add this feature" (given specification)
3. "Refactor this module" (given quality goals)
4. "Build this component" (given design)
5. "Build this application" (given description)
6. "Build this application with mobile support" (given description)
7. "Build, test, and ship this application" (given description)

Metrics:
- Task completion rate
- Evidence completeness
- Visual QA pass rate
- Test coverage achieved
- Time to completion
- Human intervention required
```

---

## 14. Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 2.0 DRAFT | 2026-09-15 | Zylvex Tech | Initial architecture v2 |

---

*ZylCode Architecture v2.0*
*Companion to ZylCode Product Constitution v2.0*
*Status: DRAFT — pending review*
