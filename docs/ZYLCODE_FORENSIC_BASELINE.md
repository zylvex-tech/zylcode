# ZYLCODE FORENSIC TRUTH BASELINE

## Executive Summary

**Date**: 2026-09-15  
**Auditor**: DeepSeek Harness  
**Repository**: C:\Projects\zylcode  
**Version**: 0.2.0  

### Overall Assessment

**ZylCode is NOT a functional autonomous software engineering agent.**

The application boots, displays a UI, and has infrastructure for provider routing and MCP tool registration. However, **the core agent loop is missing**, **tool execution is mocked**, and **there is no real filesystem/shell/git integration**.

The current application is a **provider configuration dashboard** with **simulated agent capabilities**.

---

## PHASE 0 FORENSIC FINDINGS

### 1. Repository Structure

```
C:\Projects\zylcode\
├── apps/
│   └── zylcode-desktop/          # Tauri 2 + React frontend
│       ├── src-tauri/            # Rust backend
│       └── src/                  # TypeScript frontend
├── crates/
│   ├── zylcode-core/             # Core engine library
│   ├── zylcode-mcp/              # MCP tool registry & execution
│   └── zylcode-cli/              # CLI interface
├── Cargo.toml                    # Workspace manifest
└── package.json                  # Frontend dependencies
```

### 2. Framework & Entry Points

| Component | Framework | Entry Point |
|-----------|-----------|-------------|
| Desktop App | Tauri v2 | `apps/zylcode-desktop/src-tauri/src/main.rs` |
| Frontend | React + TypeScript | `apps/zylcode-desktop/src/main.tsx` |
| Core Engine | Rust | `crates/zylcode-core/src/lib.rs` |
| MCP Registry | Rust | `crates/zylcode-mcp/src/lib.rs` |
| CLI | Rust | `crates/zylcode-cli/src/main.rs` |

### 3. State Management

| Layer | Technology | Location |
|-------|------------|----------|
| Backend State | `EngineState` struct | `main.rs:13-17` |
| Frontend State | React hooks | `src/App.tsx` |
| IPC | Tauri `invoke` commands | `main.rs:34-758` |
| Tool Registry | `Arc<ToolRegistry>` | `lib.rs:161` |
| Provider Config | `Arc<RwLock<Vec<ProviderConfig>>>` | `main.rs:711` |

### 4. Agent Loop Analysis

**STATUS: ❌ NOT IMPLEMENTED**

The current "agent loop" is:

1. User enters prompt in UI
2. Frontend calls `process_intent` or `process_intent_stream` via Tauri IPC
3. Backend calls `ZylCodeEngine::process_intent()`
4. Engine calls `pipeline.execute_pipeline()`
5. Pipeline calls `router.dispatch_prompt()` → **REAL LLM CALL**
6. Pipeline parses artifacts from LLM response
7. Pipeline runs verification callback
8. Returns `IntentResult` to frontend

**MISSING**: 
- No tool execution between LLM calls
- No iterative reasoning loop
- No context gathering from repository
- No file reading/writing
- No shell command execution
- No git operations
- No test execution
- No error recovery/repair

### 5. Model/Provider System

**STATUS: ✅ IMPLEMENTED (Real HTTP calls)**

| Provider | Status | Evidence |
|----------|--------|----------|
| Anthropic | ✅ Real | `router.rs:799-803` - Makes HTTP POST to Anthropic API |
| Ollama | ✅ Real | `router.rs:805-812` - Makes HTTP POST to localhost:11434 |
| OpenRouter | ✅ Real | `router.rs:813-822` - Makes HTTP POST to OpenRouter API |
| Synthetic Offline | ✅ Real | `router.rs:637-651` - Returns deterministic response when no API keys |

**Evidence**: `router.rs:775-863` contains `call_provider()` which makes real HTTP requests.

### 6. MCP Tool System

**STATUS: ❌ MOCKED**

#### Tool Registration
- **WORKING**: `register_from_default_location()` loads `mcp.tools.yaml`
- **WORKING**: 30 tools are registered in the tool registry
- **WORKING**: Tools are displayed in the UI

#### Tool Execution
- **MOCKED**: `DynamicTool::call()` returns echo of parameters
- **MOCKED**: No real `tokio::process::Command` execution
- **MOCKED**: No real filesystem operations
- **MOCKED**: No real shell commands

**Evidence**: `tool.rs:64-98` shows `DynamicTool::call()` returns:
```rust
Ok(serde_json::json!({
    "tool": self.config.id,
    "transport": "stdio",
    "echo": params,
    "command": self.config.command,
}))
```

The comment says: "In this phase we provide a deterministic local simulation that remains testable without external processes."

### 7. File System Operations

**STATUS: ❌ MOCKED**

No real filesystem integration exists. Tools like `fs.read`, `fs.write`, `fs.list` are defined in `mcp.tools.yaml` but execution is mocked.

### 8. Terminal/Shell Execution

**STATUS: ❌ MOCKED**

No real process execution. Tools like `npm.run`, `cargo test`, `git.status` are defined but execution is mocked.

### 9. Git Operations

**STATUS: ❌ MOCKED**

No real git integration. Tools like `git.commit`, `git.push`, `git.pull` are defined but execution is mocked.

### 10. Streaming Responses

**STATUS: ✅ IMPLEMENTED (Infrastructure)**

- Backend emits `intent:chunk` events via `process_intent_stream`
- Frontend listens for chunks via `useStreamSubscription`
- Streaming infrastructure exists but content is from LLM, not tool execution

### 11. Artifacts

**STATUS: ✅ IMPLEMENTED (From LLM only)**

- Pipeline parses artifacts from LLM response
- Frontend displays artifacts in `ArtifactViewer`
- Artifacts are generated by LLM, not by tool execution

### 12. Session/Task Persistence

**STATUS: ❌ NOT IMPLEMENTED**

- No durable session storage
- No task state persistence
- No conversation history
- No execution evidence ledger

### 13. Diff/Change Tracking

**STATUS: ❌ NOT IMPLEMENTED**

- No real file changes
- No diff generation
- No change tracking
- No before/after evidence

### 14. Approval/Permission System

**STATUS: ❌ NOT IMPLEMENTED**

- No approval boundaries
- No permission checks
- No risk classification
- No dangerous action gates

---

## CAPABILITY MATRIX

| Capability | Status | Evidence |
|------------|--------|----------|
| Application boots | ✅ GREEN | Process running, window visible |
| Provider configuration UI | ✅ GREEN | UI shows providers, config persists |
| MCP tool registration | ✅ GREEN | 30 tools loaded from YAML |
| Real LLM provider calls | ✅ GREEN | HTTP calls to Anthropic/Ollama/OpenRouter |
| Streaming infrastructure | ✅ GREEN | Events emit and frontend listens |
| Artifact display | ✅ GREEN | Artifacts parsed from LLM response |
| Agent conversation | 🔴 RED | No real agent loop, single LLM call |
| File read/write | 🔴 RED | Tools mocked, no real FS operations |
| Terminal execution | 🔴 RED | Commands mocked, no real execution |
| Git operations | 🔴 RED | Git tools mocked |
| Tool execution | 🔴 RED | All tools return mock success |
| Context gathering | 🔴 RED | No repository analysis |
| Planning | 🔴 RED | No task decomposition |
| Iterative reasoning | 🔴 RED | No observe→plan→act→verify loop |
| Error recovery | 🔴 RED | No failure diagnosis/repair |
| Session persistence | 🔴 RED | No durable session storage |
| Diff/changes | 🔴 RED | No real diff generation |
| Test execution | 🔴 RED | Tests mocked |
| Approval system | 🔴 RED | No permission boundaries |
| Real agentic loop | 🔴 RED | No context→plan→act→verify cycle |

---

## WHAT ACTUALLY WORKS

1. ✅ Tauri application launches and displays UI
2. ✅ Provider configuration UI works
3. ✅ MCP tool definitions are loaded from YAML
4. ✅ Real HTTP calls to AI providers (when API keys configured)
5. ✅ Streaming event infrastructure
6. ✅ Artifact parsing from LLM response
7. ✅ Basic verification callback system

## WHAT IS MISSING/BROKEN

1. ❌ Real tool execution (file, terminal, git)
2. ❌ Real agent loop (plan→execute→verify)
3. ❌ Real file system operations
4. ❌ Real terminal output
5. ❌ Real git integration
6. ❌ Real test execution
7. ❌ Real diff generation
8. ❌ Session persistence
9. ❌ Task state management
10. ❌ Approval/permission system
11. ❌ Context gathering from repository
12. ❌ Iterative reasoning loop
13. ❌ Error recovery/repair

---

## MOCKED COMPONENTS

### 1. Tool Execution (tool.rs:64-98)
```rust
async fn call(&self, params: Value) -> Result<Value> {
    // Simulate stdio tool echo — real impl would use tokio::process::Command
    Ok(serde_json::json!({
        "tool": self.config.id,
        "transport": "stdio",
        "echo": params,
        "command": self.config.command,
    }))
}
```

### 2. MCP Tool Definitions (mcp.tools.yaml)
All 30 tools are defined but execution is mocked:
- `git.commit`, `git.push`, `git.pull`, `git.status`, `git.diff`
- `code.analyze`, `code.format`, `code.lint`
- `test.run`, `test.single`
- `build.debug`, `build.release`
- `npm.install`, `npm.run`
- `fs.read`, `fs.write`, `fs.list`
- `search.grep`, `search.find`
- `docker.build`, `docker.run`
- `k8s.get`, `k8s.apply`
- `db.query`
- `ai.train`, `ai.evaluate`
- `slack.send`
- `security.scan`
- `perf.bench`
- `docs.generate`

### 3. Agent Loop
The current pipeline is:
1. User prompt → LLM call → Parse artifacts → Return result

**MISSING**: Tool execution, iterative reasoning, error recovery

---

## EVIDENCE-BASED STATUS CATEGORIES

| Category | Definition |
|----------|------------|
| **GREEN** | Demonstrated end-to-end with actual execution evidence |
| **YELLOW** | Implemented but insufficiently verified |
| **RED** | Missing, broken, placeholder, mocked, or not demonstrated |
| **GRAY** | Architectural intention / future capability |

---

## CRITICAL FINDINGS

### 1. The "30 MCP tools" are PLACEHOLDERS
They are defined in YAML but execution is mocked. The UI shows them as available, but they don't actually do anything.

### 2. No real file system integration
Tools claim to read/write files but don't. No actual filesystem operations occur.

### 3. No real terminal
Commands are simulated. No actual process execution occurs.

### 4. No real git
Git operations are mocked. No actual git commands are executed.

### 5. No real agent loop
The pipeline calls the LLM once and returns. No iterative reasoning, no tool execution, no error recovery.

### 6. Provider settings dominate because there is no real coding agent to show
The UI shows provider configuration because that's the only thing that actually works.

---

## CONCLUSION

**The application is a PROVIDER CONFIGURATION DASHBOARD with mocked agent capabilities.**

The "30 MCP tools" are UI representations, not working implementations.

The agent loop is: User prompt → LLM call → Mock tool execution → Synthetic response.

There is **NO REAL CODING AGENT** behind the UI.

---

## NEXT STEPS

To build a real Codex/Claude/Devin-class product, we need to:

1. **Implement real tool execution** - Actually read/write files, run commands
2. **Build real agent loop** - Context gathering → Planning → Tool selection → Execution → Verification
3. **Integrate real filesystem** - Direct file operations, not mocked
4. **Build real terminal** - Stream actual command output
5. **Implement real git** - Actual git operations
6. **Create session persistence** - Save/restore work
7. **Build approval system** - User control over dangerous actions

---

## PHASE 1A IMPLEMENTATION PLAN

### Objective
Replace mocked tool execution with real typed Tool Runtime.

### Scope
Implement real primitives for:
- FILESYSTEM: read, write, create, list, search
- SHELL/PROCESS: spawn, stdout/stderr streaming, exit code
- GIT: status, diff, commit, branch
- SEARCH: fast repository search

### Success Criteria
The first production milestone must prove this real workflow:

```
USER TASK
→ AGENT READS REAL REPOSITORY FILE
→ AGENT IDENTIFIES REQUIRED CHANGE
→ AGENT PRODUCES PLAN
→ USER/POLICY APPROVES
→ AGENT MODIFIES REAL FILE
→ DIFF RECORDS REAL CHANGE
→ AGENT RUNS REAL TEST/BUILD COMMAND
→ REAL STDOUT/STDERR RETURNS
→ AGENT DETECTS FAILURE OR SUCCESS
→ AGENT REPAIRS IF NECESSARY
→ TEST PASSES
→ GIT DIFF CONFIRMS CHANGE
→ EVIDENCE LEDGER CONFIRMS EVERY ACTION
```

### Implementation Order
1. Create `ToolRuntime` trait with real execution
2. Implement `FileSystemTool` with real file operations
3. Implement `ShellTool` with real process execution
4. Implement `GitTool` with real git commands
5. Implement `SearchTool` with real repository search
6. Replace `DynamicTool::call()` with real execution
7. Add execution evidence logging
8. Test end-to-end workflow

---

**END OF FORENSIC BASELINE**