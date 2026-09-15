# ZYLCODE FORENSIC AUDIT SUMMARY

## Date: 2026-09-15

## Auditor: DeepSeek Harness

## Repository: C:\Projects\zylcode

---

## EXECUTIVE SUMMARY

**ZylCode is NOT a functional autonomous software engineering agent.**

The application boots, displays a UI, and has infrastructure for provider routing and MCP tool registration. However, **the core agent loop is missing**, **tool execution is mocked**, and **there is no real filesystem/shell/git integration**.

The current application is a **provider configuration dashboard** with **simulated agent capabilities**.

---

## KEY FINDINGS

### ✅ WHAT WORKS (6 capabilities)

1. **Application Lifecycle**: Tauri v2 app launches successfully
2. **Provider Configuration**: UI shows providers, config persists
3. **MCP Tool Registration**: 30 tools loaded from YAML
4. **Real LLM Calls**: HTTP calls to Anthropic/Ollama/OpenRouter
5. **Streaming Infrastructure**: Events emit and frontend listens
6. **Artifact Display**: Artifacts parsed from LLM response

### ❌ WHAT IS MOCKED (14 capabilities)

1. **Agent Conversation**: Single LLM call, no iterative reasoning
2. **File Read/Write**: Tools mocked, no real FS operations
3. **Terminal Execution**: Commands mocked, no real execution
4. **Git Operations**: Git tools mocked
5. **Tool Execution**: All tools return mock success
6. **Context Gathering**: No repository analysis
7. **Planning**: No task decomposition
8. **Iterative Reasoning**: No observe→plan→act→verify loop
9. **Error Recovery**: No failure diagnosis/repair
10. **Session Persistence**: No durable session storage
11. **Diff/Changes**: No real diff generation
12. **Test Execution**: Tests mocked
13. **Approval System**: No permission boundaries
14. **Real Agentic Loop**: No context→plan→act→verify cycle

---

## CRITICAL EVIDENCE

### 1. Tool Execution is Mocked

**File**: `crates/zylcode-mcp/src/tool.rs:64-98`

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

**Conclusion**: All 30 MCP tools return mock success without executing anything.

### 2. No Real Agent Loop

**File**: `crates/zylcode-core/src/pipeline.rs:138-199`

The pipeline:
1. Builds execution plan
2. Calls LLM once
3. Parses artifacts
4. Runs verification callback
5. Returns result

**MISSING**: Tool execution, iterative reasoning, error recovery

### 3. Real LLM Calls Exist

**File**: `crates/zylcode-core/src/router.rs:775-863`

The `call_provider()` method makes real HTTP calls to:
- Anthropic API
- Ollama localhost:11434
- OpenRouter API

**Conclusion**: Provider routing works, but tool execution doesn't.

---

## CAPABILITY STATUS

| Category | Count | Percentage |
|----------|-------|------------|
| GREEN (Working) | 6 | 30% |
| YELLOW (Partial) | 0 | 0% |
| RED (Mocked/Missing) | 14 | 70% |
| **TOTAL** | **20** | **100%** |

---

## CONCLUSION

**The application is a PROVIDER CONFIGURATION DASHBOARD with mocked agent capabilities.**

The "30 MCP tools" are UI representations, not working implementations.

The agent loop is: User prompt → LLM call → Mock tool execution → Synthetic response.

There is **NO REAL CODING AGENT** behind the UI.

---

## RECOMMENDED NEXT STEPS

### Phase 1A: Real Tool Runtime Foundation

**Objective**: Replace mocked tool execution with real typed Tool Runtime.

**Scope**:
1. Create `ToolRuntime` trait with real execution
2. Implement `FileSystemTool` with real file operations
3. Implement `ShellTool` with real process execution
4. Implement `GitTool` with real git commands
5. Implement `SearchTool` with real repository search
6. Replace `DynamicTool::call()` with real execution
7. Add execution evidence logging
8. Test end-to-end workflow

**Success Criteria**:
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

---

## FILES CREATED

1. `docs/ZYLCODE_FORENSIC_BASELINE.md` - Detailed forensic audit
2. `docs/capability-registry.json` - Machine-readable capability registry
3. `docs/FORENSIC_AUDIT_SUMMARY.md` - This summary

---

**END OF FORENSIC AUDIT**