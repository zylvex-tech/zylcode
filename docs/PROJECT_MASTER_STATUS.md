# ZylCode — Project Master Status

> **Last updated:** 2026-10-23 · `main` @ `6469bae` · 125 commits on main
>
> This document is the single source of truth for where ZylCode is, what has been  
> built, what remains, and what to do next. It is updated after every merged PR.

---

## 1. Executive Summary

ZylCode is a **local-first, bring-your-own-key AI engineering environment** that  
transforms intent into verified, evidence-backed deliverables. It is not a  
chatbot wrapper — it is an operating system for software creation with a  
permission-gated tool runtime, an evidence ledger, and a structured agent loop.

**Current state:** The foundational kernel (Phases 1A–1D) is implemented and  
accepted. Phase 2A (Repository Intelligence) was rejected in independent audit  
and is awaiting remediation. Everything beyond 2A is blocked or not started.

**The gap is not "more code" — it is "wire existing libraries into product  
surfaces" and "get Phase 2A accepted."**

---

## 2. Phase Status Board (Authoritative)

ZylCode is built in **16 phases across 6 epochs**. A phase must be **ACCEPTED**  
(not merely implemented) before dependents may begin.

| Phase  | Title                                  | Status                         | Rung | Blocks                 |
| ------ | -------------------------------------- | ------------------------------ | ---- | ---------------------- |
| **1A** | Real Tool Runtime                      | ✅ **ACCEPTED**                 | R3   | 1B                     |
| **1B** | Agent Execution Loop                   | ✅ **ACCEPTED**                 | R3   | 1C                     |
| **1C** | Model-Driven Autonomous Agent          | ✅ **ACCEPTED WITH CONDITIONS** | R3   | 1D                     |
| **1D** | Durable Memory, Evidence & Recovery    | ✅ **ACCEPTED WITH CONDITIONS** | R3   | 2A                     |
| **2A** | Repository Intelligence Foundation     | ❌ **NOT ACCEPTED — RE-OPENED** | R2   | 2B                     |
| **2B** | Repository Reasoning & Impact Analysis | 🚫 **BLOCKED on 2A**           | —    | 3A                     |
| **3A** | ZylCode Project System                 | ⏸ **NOT STARTED**              | —    | 3B, 4, 5, 6A, 8A       |
| **3B** | Mission Engine                         | ⏸ **NOT STARTED**              | —    | 4, 14                  |
| **4**  | Model Platform / Model Democracy       | ⏸ **NOT STARTED**              | —    | 14                     |
| **5**  | Extension Platform                     | ⏸ **NOT STARTED**              | —    | 6A, 7A, 10, 11, 13, 15 |
| **6A** | Artifact System                        | ⏸ **NOT STARTED**              | —    | 6B, 7A, 9              |
| **6B** | Live Preview Runtime                   | ⏸ **NOT STARTED**              | —    | 7A                     |
| **7A** | Browser Runtime                        | ⏸ **NOT STARTED**              | —    | 7B, 9, 12              |
| **7B** | Computer Use Foundation                | ⏸ **NOT STARTED**              | —    | 7C, 7D                 |
| **7C** | Computer Use Reliability               | ⏸ **NOT STARTED**              | —    | 7D                     |
| **7D** | Application Adapters                   | ⏸ **NOT STARTED**              | —    | 9, 12                  |
| **8A** | Design Foundation                      | ⏸ **NOT STARTED**              | —    | 8B, 8C                 |
| **8B** | Professional Design Canvas             | ⏸ **NOT STARTED**              | —    | 9                      |
| **8C** | Design ↔ Code                          | ⏸ **NOT STARTED**              | —    | 9                      |
| **9**  | Visual Intelligence & Self-Repair      | ⏸ **NOT STARTED**              | —    | 12, 16                 |
| **10** | Android Device Lab                     | ⏸ **NOT STARTED**              | —    | 12, 13, 16             |
| **11** | macOS/iOS Worker                       | ⏸ **NOT STARTED**              | —    | 12, 13                 |
| **12** | Proof Engine v2                        | ⏸ **NOT STARTED**              | —    | 13, 16                 |
| **13** | Delivery Engine                        | ⏸ **NOT STARTED**              | —    | 16                     |
| **14** | Multi-Agent Engineering                | ⏸ **NOT STARTED**              | —    | 15, 16                 |
| **15** | Marketplace and Ecosystem              | ⏸ **NOT STARTED**              | —    | 16                     |
| **16** | Public Commissioning                   | ⏸ **NOT STARTED**              | —    | —                      |

**Rung legend:** R0 = simulation · R1 = scaffold · R2 = library/test · R3 =  
product surface + evidence · R4 = independent audit · R5 = computed from evidence

---

## 3. What Has Been Built (Completed Phases)

### Phase 1A — Real Tool Runtime ✅ ACCEPTED

Replaced mocked tool execution with real typed `ToolRuntime`.

- **Real executors:** `FileSystemTool`, `ShellTool`, `GitTool`, `SearchTool`
- **Evidence recording:** every execution produces a `ToolResult` with evidence
- **Risk classification:** tools classified by `RiskLevel` (Read → Destructive)
- **Tests:** tool invocation produces real filesystem/shell effects with ledger entries

**Files:** `crates/zylcode-mcp/src/real_tools.rs`, `tool.rs`  
**Commit:** `5f5a99e`

---

### Phase 1B — Agent Execution Loop ✅ ACCEPTED

Iterative reasoning loop with tool execution and observation feedback.

- **State machine:** `Created → Analyzing → Planning → Executing → Verifying → Completed`
- **Observation feedback:** tool results fed back to model for next decision
- **Bounded execution:** step limits, timeouts, cancellation
- **Tests:** `test_observation_loop`, `test_repair_loop` pass; loop terminates on budget

**Files:** `crates/zylcode-core/src/agent.rs`  
**Commit:** (series leading to agent loop)

---

### Phase 1C — Model-Driven Autonomous Agent ✅ ACCEPTED WITH CONDITIONS

Model-driven decisions via formal `AgentDecision` protocol.

- **Decision protocol:** `Think`, `Plan`, `ToolCall`, `RequestApproval`, `Verify`, `Complete`, `Fail`
- **Approval enforcement:** destructive actions require explicit approval; auto-approve removed
- **Completion verification:** cannot claim `Complete` without evidence
- **Conditions:** live multi-provider commissioning outstanding

**Files:** `crates/zylcode-core/src/agent_protocol.rs`

---

### Phase 1D — Durable Memory, Evidence & Recovery ✅ ACCEPTED WITH CONDITIONS

Durable memory with crash recovery, evidence ledger, and permission gate.

- **Evidence ledger:** append-first, idempotent `LedgerStore` with SQLite/memory impls
- **Crash recovery:** 3 crash windows tested; resume without duplicate side effects
- **Hash-chain integrity:** SHA-256 chain for ledger integrity
- **Fail-closed verification:** any error → `FAILED`, never `Verified`
- **Conditions:** generic verification execution remains partial

**Files:** `crates/zylcode-core/src/ledger.rs`, `sqlite_ledger.rs`, `crash_recovery.rs`  
**Commit:** `6f564ac`

---

### Phase 2A — Repository Intelligence Foundation ❌ RE-OPENED

A structured, queryable model of a repository. **Implemented but NOT ACCEPTED.**

**What exists:**

- 7,224-line intelligence library in `crates/zylcode-core/src/intelligence/`
- File scanner (286 files / 1,552 symbols indexed after fixes)
- Manifest parser (Cargo, pnpm, npm)
- Symbol extraction (Rust + TypeScript, regex-based)
- Dependency graph (package-level, bidirectional)
- Git change intelligence
- Query API (14 typed methods)
- Context retrieval with relevance ranking
- Benchmark: P@10=0.48, R@10=1.00 on 20 known-answer queries

**Why rejected:**

1. Headline metric initially counted build output (14,900 of 15,064 files under `target/`)
2. Benchmark timing failed on re-execution (276s vs claimed 12.7s)
3. Zero product integration — library only, no UI or CLI surface beyond debug
4. Windows path-normalization defect in exclusion logic

**Remediation status:** P0 fixes applied (path exclusion, metric rebaseline,  
determinism proof, reproduction transcript). P1 items (integration into AgentLoop,  
quality thresholds, failing test resolution) **NOT STARTED**.

**Blocking:** Phase 2B and everything downstream.

---

## 4. Recent Engineering (2026-10)

These are the most recently merged changes on `main`:

### PR #28 — `serde-saphyr` migration (merged 2026-10-23)

- Replaced `serde_yaml` 0.9.34 / `unsafe-libyaml` 0.2.11 with `serde-saphyr` 2.0.0
- `serde-saphyr` uses `granit-parser`, a pure-Rust YAML parser (`#![forbid(unsafe_code)]`)
- Preserved strict boolean semantics (`strict_booleans: true`)
- Preserved empty-document rejection
- Rewrote `manifest.rs` from untyped `serde_yaml::Value` to typed `PnpmWorkspace` struct
- Added 14 regression tests
- Bumped workspace MSRV 1.77 → 1.89
- Resolved RUSTSEC-2024-0449
- Commits: `ae012f6` → `e72db9c` → `10fa40b`

### PR #29 — `CODEOWNERS` (merged 2026-10-23)

- Added `CODEOWNERS` file for repository ownership governance

### PR #25 — Post-merge audit (merged earlier)

- Corrected advisory register counts
- Verified dependency states

### PR #23 — Router egress spy thread-local (merged)

- Scoped test-only egress spy per thread, not per process

### PR #24 — LRU 0.12 unsound advisories (merged)

- Moved `lru` off 0.12.5 (two unsound advisories)
- Moved `tokio` off a yanked release

---

## 5. Capability Registry (Honest Assessment)

The authoritative registry is `docs/capability-registry.json` (v1.4.1-audited).

| Status               | Count | What                                                                           |
| -------------------- | ----- | ------------------------------------------------------------------------------ |
| **GREEN**            | 22    | Product-reachable capabilities with evidence                                   |
| **DISPUTED**         | 12    | Phase 2A intelligence capabilities; libraries without product surfaces         |
| **PARTIAL**          | 2     | `release_installers` (CI blocked), `tool_system_executable` (R3 slice pending) |
| **EXTERNAL_BLOCKER** | 1     | `remote_ci` (GitHub billing)                                                   |

**Key GREEN capabilities:** application lifecycle, provider configuration, MCP tool  
registration, real LLM calls, streaming, agent protocol, tool runtime, model  
client abstraction, context builder, approval enforcement, completion verifier,  
observation loop, repair loop, bounded execution, risk classification, evidence  
ledger, session persistence, crash recovery.

**Key DISPUTED capabilities:** repository scanner, manifest intelligence, symbol  
indices, dependency graph, git change graph, architecture fingerprint, context  
retrieval — the heart of the product's differentiated value.

---

## 6. What Is Left To Do

### Immediate (unblocks everything)

**Phase 2A re-acceptance.** This is the critical path. Without it, 2B is blocked,  
and without 2B, Epoch II (Project & Agent OS) cannot begin.

P1 remediation items still open:

1. Integrate intelligence into `AgentLoop` context assembly + CLI entry point —  
   **or** honestly downgrade the 11 DISPUTED capabilities to PARTIAL/R2
2. Raise quality thresholds: `Precision@10 ≥ 0.60`, Q5/Q6/Q10 non-zero
3. Resolve failing `router::tests::synthetic_offline_dispatch_returns_parseable_payload`
4. Commit or discard 53 modified files from the dirty-tree incident

### Near-term (after 2A accepted)

| Phase | What                                                                                                            | Why it matters                                            |
| ----- | --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| 2B    | Repository reasoning — impact analysis, definition/reference traversal, test-to-code relationships, change risk | Turns the index into actionable intelligence              |
| 3A    | ZylCode Project System — persistent project identity, shared world state                                        | A Project is not a directory; this is the OS layer        |
| 3B    | Mission Engine — replace "Ask AI" with structured Mission                                                       | Missions are resumable, inspectable, evidence-backed      |
| 5     | Extension Platform — stable ABI                                                                                 | Everything that adds capability without core modification |

### Medium-term

| Phase | What                                                  |
| ----- | ----------------------------------------------------- |
| 6A–6B | Artifact System + Live Preview Runtime                |
| 7A–7D | Browser Runtime + Computer Use (real, not simulation) |
| 8A–8C | Design Foundation + Canvas + Design↔Code              |
| 9     | Visual Intelligence & Self-Repair                     |
| 12    | Proof Engine v2 — computed rung assignment            |
| 13    | Delivery Engine — installers, signing, deployment     |
| 14–16 | Multi-Agent, Marketplace, Public Commissioning        |


## 8. The Most Logical Next Step

> **Start Phase 2A P1 remediation.**

Specifically:

1. **Check out `main` at the latest tip (`6469bae`)**
2. **Verify a clean worktree compiles:** `cargo check --workspace --all-targets`
3. **Address the dirty-tree legacy:** review `performance.rs` and  
   `system_integration.rs` — commit if real, discard if not, but do not leave  
   them as unreviewed untracked files that a future clean checkout depends on
4. **Fix the failing router test:** `synthetic_offline_dispatch_returns_parseable_payload`
5. **Wire intelligence into AgentLoop:** when the agent receives a prompt,  
   automatically call `context_retrieval` and feed ranked context into the prompt
6. **Add a real CLI entry point:** `zylcode repo-context "<task>"` already exists;  
   ensure it is product-reachable and documented
7. **Raise benchmark thresholds** and ensure Q5/Q6/Q10 return non-zero results
8. **Request re-audit** of Phase 2A once P1 items are complete

**Why this is the right next step:**

- It unblocks Phase 2B, which unblocks Epoch II, which unblocks everything else
- It converts existing library code (7,224 lines) into product value
- It addresses the independent audit's core finding: "libraries without product  
  surfaces are R2, not R3"
- It does not require building new crates — it requires wiring what exists

**What NOT to do:**

- Do not start Phase 3A, 5, 6, or 7 before 2A is accepted (violates sequencing rules)
- Do not mark DISPUTED capabilities GREEN without product surfaces
- Do not build more crates before wiring existing ones
- Do not ship the Computer-Use simulation without labeling it as simulation

---

## 9. Verification Commands (current `main`)

```bash
# Rust workspace
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets

# Frontend
pnpm install --frozen-lockfile
pnpm --filter zylcode-desktop build
pnpm --filter zylcode-desktop test

# Security audit
cargo audit                # 2 warnings (accepted), 0 vulnerabilities
pnpm audit --prod          # clean

# Dependency verification (post-PR #28)
cargo tree -p serde_yaml        # should error: not found
cargo tree -p unsafe-libyaml    # should error: not found

# Capability registry check
python scripts/check_retracted_claims.py
```

**Latest verified run:** 2026-10-23, PR #28 merge, 4 CI checks passed.

---

## 10. Document Index

| Document                                             | Purpose                                      |
| ---------------------------------------------------- | -------------------------------------------- |
| `docs/roadmap/ZYLCODE_ROADMAP_V2.md`                 | 16-phase roadmap with gates and dependencies |
| `docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md`   | Phase program and execution rules            |
| `docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md` | Supreme spec                                 |
| `docs/capability-registry.json`                      | 37-entry capability ledger                   |
| `docs/governance/MAIN_STATUS_REPORT_2026-10-11.md`   | Independent audit of `main`                  |
| `docs/governance/PHASE2A_INDEPENDENT_AUDIT.md`       | Why Phase 2A was rejected                    |
| `docs/governance/PHASE2A_REMEDIATION_ORDER.md`       | Required fixes for 2A re-acceptance          |
| `docs/PHASE1A_COMPLETION_REPORT.md`                  | Phase 1A evidence                            |
| `docs/PHASE1B_COMPLETION_REPORT.md`                  | Phase 1B evidence                            |
| `docs/PHASE1C_COMPLETION_REPORT.md`                  | Phase 1C evidence                            |
| `docs/PHASE1D_COMPLETION_REPORT.md`                  | Phase 1D evidence                            |
| `docs/PHASE2A_COMPLETION_REPORT.md`                  | Phase 2A evidence (pre-audit)                |
| `docs/governance/ACCEPTED_ADVISORIES.md`             | Dependency advisory register                 |
| `README.md`                                          | Public-facing project overview               |

---

> **Built with evidence. Verified with honesty.**
>
> *ZylCode — Software Creation Operating System · Zylvex Technologies Ltd.*
