# AGENT_KERNEL.md — ZylCode Agent Kernel

**Status: PARTIAL — implemented at R3 for Phases 1A–1D**
**Rung: R3 (Phases 1A, 1B) · R3-with-conditions (1C, 1D)**
**Phases: 1A, 1B, 1C, 1D (done) · extended by 3B, 4, 14**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §5 · `ZYLCODE_ARCHITECTURE_V2.md` §3.2

---

## 1. Responsibility

Reasoning, planning, tool invocation, workflows, model selection, permissions and memory.

The Agent Kernel is the component that **decides**. It does not own the truth about the
repository, the project, or the proof — it consults the systems that do.

---

## 2. What Exists Today (honest inventory)

| Component | Status | Rung | Evidence |
|---|---|---|---|
| Real tool runtime (`FileSystemTool`, `ShellTool`, `GitTool`, `SearchTool`) | FUNCTIONAL | R3 | `crates/zylcode-mcp/src/real_tools.rs` |
| Iterative reasoning loop with tool execution | FUNCTIONAL | R3 | `crates/zylcode-core/src/agent.rs` |
| Observation feedback (tool results → next decision) | FUNCTIONAL | R3 | `test_observation_loop` |
| Repair loop on failure | FUNCTIONAL | R3 | `test_repair_loop` |
| `AgentDecision` protocol | FUNCTIONAL | R3 | `crates/zylcode-core/src/agent_protocol.rs` |
| Approval enforcement on risk | FUNCTIONAL | R3 | `requires_approval()`, `AwaitingApproval` |
| Completion verification requiring evidence | FUNCTIONAL | R3 | `verify_completion_evidence()` |
| Bounded execution (steps, timeouts, cancellation) | FUNCTIONAL | R3 | step limits |
| Durable memory + checkpoint recovery | FUNCTIONAL | R3 | `SessionCheckpoint`, crash-window tests |
| Evidence ledger | FUNCTIONAL | R3 | `crates/zylcode-core/src/ledger.rs` |
| Model client abstraction (`RealModelClient`, `TestModelClient`) | FUNCTIONAL | R3 | `router.rs` |
| **Live multi-provider commissioning** | **PARTIAL** | R2 | providers configured; end-to-end live run outstanding |
| **Generic verification execution** | **PARTIAL** | R2 | Phase 1D condition |
| **Repository-graph context assembly** | **ABSENT** | R0 | still uses pre-existing `ContextBuilder` |
| **Shared world state** | **ABSENT** | R0 | requires Phase 3A |

> **Note the last two rows.** They are the reason Phase 2B and Phase 3A matter: today the Agent
> Kernel assembles context from a file-tree walker rather than from the repository graph, and it
> has no shared world state. Both are recorded defects, not design choices.

---

## 3. The Decision Protocol

```
                 ┌──────────┐
                 │  THINK   │
                 └────┬─────┘
                      ▼
                 ┌──────────┐
                 │   PLAN   │
                 └────┬─────┘
                      ▼
              ┌───────────────┐
              │  TOOL CALL?   │──no──┐
              └───────┬───────┘      │
                      │ yes          │
                      ▼              │
            ┌──────────────────┐     │
            │ NEEDS APPROVAL?  │     │
            └────┬────────┬────┘     │
              no │        │ yes      │
                 │        ▼          │
                 │  ┌───────────────┐│
                 │  │ REQUEST       ││
                 │  │ APPROVAL      ││
                 │  └───────┬───────┘│
                 │          │        │
                 ▼          ▼        ▼
            ┌─────────────────────────────┐
            │        EXECUTE              │
            └──────────────┬──────────────┘
                           ▼
                     ┌───────────┐
                     │ OBSERVE   │
                     └─────┬─────┘
                           ▼
                     ┌───────────┐
                     │  VERIFY   │
                     └─────┬─────┘
                           ▼
                  ┌────────────────┐
                  │ COMPLETE?      │
                  └───┬────────┬───┘
                   yes│        │no
                      ▼        ▼
                 ┌────────┐ ┌────────┐
                 │COMPLETE│ │REPAIR /│
                 │(needs  │ │RETRY   │
                 │evidence│ │        │
                 └────────┘ └────────┘
```

**Hard rule:** `COMPLETE` requires evidence. `verify_completion_evidence()` refuses a completion
claim with no tool execution and no evidence. This is enforced, not advisory.

---

## 4. Context Assembly (the current defect)

### 4.1 Today

```
AgentLoop::assemble_context()
   └─► context_builder::ContextBuilder   (file tree walk, git status, file reads)
```

`crates/zylcode-core/src/agent.rs:14, 223, 315, 666–687`

### 4.2 Required

```
AgentLoop::assemble_context()
   └─► IntelligenceGraph.query(RepoQuery)
         ├─ packages · symbols · dependencies
         ├─ entry points · architecture · change graph
         └─ relevant_context(task) → ranked results WITH REASON
```

### 4.3 Why this is not cosmetic

| Current | Required |
|---|---|
| Returns files | Returns **ranked** files with **reasons** |
| No dependency awareness | Transitive dependency awareness |
| No symbol awareness | Symbol-level targeting |
| No test relationships | Test↔code relationships |
| Recomputes every time | Incremental, cached, hash-validated |

**This is the Phase 2B deliverable, and it is why Phase 2A had to be correct.**

Until this is done, the Agent Kernel's context is *plausible* rather than *informed*. A
plausible context produces plausible code. An informed context produces correct code.

---

## 5. Tool Contract

```rust
trait RealTool {
    fn id(&self) -> &str;
    fn schema(&self) -> ToolSchema;      // name, params, description
    fn risk(&self) -> RiskLevel;         // → drives approval
    async fn invoke(&self, params: Value, ctx: &ToolContext) -> ToolResult;
}
```

**Requirements**

1. Every tool declares a **risk level**. Risk drives approval. Undeclared risk is treated as
   maximum risk (fail closed).
2. Every invocation produces a **ledger entry** with params, result, duration and outcome.
3. Tools are **discoverable** — `ToolRegistry::list_schemas()` is what the model sees.
4. A tool that cannot describe itself cannot be registered.

---

## 6. Memory

Three layers, deliberately distinct:

| Layer | Scope | Lifetime | Owner |
|---|---|---|---|
| **Working context** | current step | request | Agent Kernel |
| **Session memory** | current mission | mission | Project System (Phase 3A) |
| **Engineering memory** | project | project | Project System + Ledger |

> Memory is not a chat log. Memory is structured state that outlives the conversation that
> produced it. This is the difference between an assistant and a system.

---

## 7. Permissions

- Fail-closed: absence of a grant is denial.
- Every decision — allow **and** deny — is recorded with the rule that produced it.
- Approval is a **state** (`AwaitingApproval`), not a prompt. It survives restart.
- Permission grants are Project decisions, inspectable later.

---

## 8. Bounded Execution

Every run has hard limits:

- maximum steps
- wall-clock timeout
- token/cost budget
- cancellation

Exceeding a limit terminates the run with an explicit status. It never silently continues.
"Agent cannot run indefinitely" is a product requirement, not an optimisation.

---

## 9. Entry Points (required for R3)

```
zylcode run "<task>"              # bounded agent run
zylcode agent status              # current state
zylcode agent approve <id>        # grant a pending approval
zylcode agent cancel <id>
```

Plus the UI agent surface (run, observe, approve, cancel).

---

## 10. Remaining Work

| # | Item | Phase | Blocks |
|---|---|---|---|
| 1 | Live multi-provider commissioning | 1C closure | Model Platform claims |
| 2 | Generic verification execution | 1D closure | Proof Engine |
| 3 | Repository-graph context assembly | 2B | informed context |
| 4 | Shared world state | 3A | multi-agent |
| 5 | Mission-scoped execution | 3B | resumable autonomy |
| 6 | Capability-based model routing | 4 | Model Democracy |
| 7 | Multi-agent roles on shared state | 14 | ecosystem |

---

## 11. Anti-Requirements

The Agent Kernel must **not**:

- maintain its own copy of repository structure
- store project state
- decide whether an action is permitted (that is Permissions)
- interpret verification results (that is the Proof Engine)
- pass state to other agents via transcripts (that is what the Project Graph is for)
