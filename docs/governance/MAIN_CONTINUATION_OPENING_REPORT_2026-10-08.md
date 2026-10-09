# ZYLCODE MAIN CONTINUATION OPENING REPORT

**Date:** 2026-10-08  
**Branch:** `main` (via worktree `C:/Temp/zylcode-main-forensics`)  
**HEAD:** `b4d13ab5e279524867d91e8e8c07a30b42a8f884`  
**Origin/main:** `aae6b4e4ffd939cd09abb6d7345ae1a762f7ba03`  
**Ahead/behind main vs origin/main:** `0 ahead, 0 behind` (clean at `aae6b4e`)  
**Competition branch divergence:** `competition/natlas-2026` is 25 commits ahead of `main`; merge-base `b4d13ab`

---

## 1. Git State

| Item | Value |
|---|---|
| Current branch | `main` (worktree) |
| HEAD SHA | `b4d13ab` |
| Origin/main SHA | `aae6b4e` |
| Ahead/behind | 0/0 (main is clean relative to origin) |
| Dirty tree | 0 modified, 0 untracked (clean worktree from `main`) |
| Stash state | 0 entries |
| Worktrees | 4 total: original repo (`competition/natlas-2026`), `.wt/verify-build` (detached), `C:/Temp/zylcode-cleanroom` (detached `c8e473d`), `C:/Temp/zylcode-final` (detached `f0fcd01`) |
| Recent commits (main) | `b4d13ab` docs(gate): final owner gate report; `179a3d7` docs(census); `efab2ac` fix(census); `b09340e` feat(factory): evidence-producing factory runner; `6c383c5` feat(factory): durable dependency-aware task graph |

---

## 2. Canonical Test Baseline

| Crate | Test Count | Status |
|---|---|---|
| `zylcode-core` (lib + integration) | ~350 `#[test]` attributes | **Cannot execute** — workspace does not compile |
| `zylcode-mcp` (lib) | ~41 `#[test]` attributes | **Cannot execute** — workspace does not compile |
| `zylcode-nav` | Unknown | **Cannot execute** — workspace does not compile |
| Frontend (vitest) | 54 tests (reported 2026-09-28) | Not re-executed in this session |

**Critical finding:** The workspace compilation fails at the dependency resolution level, preventing ANY test execution from a clean checkout.

---

## 3. Build / Lint / Typecheck Status

| Check | Status | Details |
|---|---|---|
| `cargo check --workspace` | ❌ **FAIL** | `schemars 0.8.22` incompatible with `indexmap 1.9.3` — `IndexMap<K,V>` vs `IndexMap<K,V,S>` generic arity mismatch |
| `cargo build --workspace` | ❌ **FAIL** | Same root cause |
| `cargo test --workspace` | ❌ **FAIL** | Same root cause |
| `cargo clippy --workspace` | ❌ **FAIL** | Same root cause |
| `pnpm build` (frontend) | Not tested | Worktree is Rust-only forensics |

**Root cause:** `Cargo.lock` contains both `schemars 0.8.22` (which depends on `indexmap 1.9.3`) and `schemars 0.9.0` (which depends on `indexmap 2.14.2`). The `indexmap 1.9.3` crate changed its generic parameter count in a way that breaks `schemars 0.8.22`. This is a known Rust ecosystem breakage.

**Impact:** No compilation, no tests, no clippy, no new feature work possible without fixing this first.

---

## 4. Latest Completed Main-Product Milestone

**Phase 1D — Agent Kernel Foundation** (ACCEPTED with conditions)

Evidence: tool runtime, agent loop, model-driven agent, durable memory all have committed tests and product surfaces. The capability census records TESTED or RUNTIME_VERIFIED status for core trust foundation (permissions, evidence ledger, crash recovery).

---

## 5. Current Partially Completed Milestone(s)

| Phase | Status | What Exists | What is Blocked |
|---|---|---|---|
| **2A — Repository Intelligence Foundation** | 🔴 **NOT ACCEPTED — RE-OPENED** | Scanner, symbols, query API, context retrieval all implemented and tested. HTTP routes exist. | Audit rejected: metric counted `target/` (14,900 of 15,064 files); benchmark not reproducible; zero product integration. Remediation shipped; re-audit pending. |
| **2B — Repository Reasoning & Impact Analysis** | 🔴 **Blocked** on 2A acceptance | Architecture exists (`intelligence/` modules: architecture, dependency, canvas, manifest, nav_api, entry_points) | Cannot begin until 2A is accepted. |
| **Factory Foundation** (between 1D and 2A) | 🟡 **In Progress** | `factory/` module (graph, job, runner, store) committed. Evidence-producing runner with real tool dispatch. TaskGraph with dependency-aware DAG. Best-of-N verification. | Not yet accepted as a formal phase. Integration with agent loop incomplete. |
| **Project System** (UX-01C) | 🟢 **LIVE** | Workspace surfaces (explorer, editor, missions) in IDE. Persistent project store (`project_store.rs`) with schema versioning. | GUI write path broken (save command not registered in Tauri handler). |

---

## 6. Existing Agent-Kernel Capabilities

| Capability | State | Evidence |
|---|---|---|
| Tool runtime (filesystem, shell, git, search) | RUNTIME_VERIFIED | `zylcode_mcp::real_tools` — 10 committed tests |
| Permission gate (risk-classified, fail-closed) | RUNTIME_VERIFIED | `zylcode_mcp::permission` — 4 tests |
| Evidence ledger (hash-chained JSONL) | RUNTIME_VERIFIED | `zylcode_core::ledger` + `sqlite_ledger` — 6+ tests |
| Agent loop (model-driven decisions) | TESTED | `agent.rs` — model integration exists but AGENT-01 is BLOCKED |
| Crash recovery (resumable, reconciled against git) | TESTED | `memory_ledger.rs` — 3 tests |
| Provider routing (5 providers + SyntheticOffline) | WIRED | `router.rs` — routes configured but not measurement-based |
| Structured task output (`AgentDecision`) | TESTED | `router/decision.rs` + `pipeline.rs` |
| Best-of-N verification | TESTED | `best_of_n.rs` + `patch_best_of_n.rs` |
| Model democracy (scorecard) | WIRED | `provider_scorecard.rs` — foundation exists, not measurement-driven |

**AGENT-01 BLOCKER:** No usable approved cloud provider. `OPENROUTER_API_KEY` is a literal placeholder; Anthropic 401; DeepSeek 402. The `SyntheticOffline` provider works hermetically but produces deterministic placeholder responses. The agent loop is architecturally ready but cannot execute real model calls in production.

---

## 7. Existing Repository-Intelligence Capabilities

| Capability | Census Status | Notes |
|---|---|---|
| Scanner + persisted index | RUNTIME_VERIFIED | 444 files, 2111 symbols, 6 packages on HEAD |
| Symbol index (Rust + TS) | TESTED | 1552 symbols; regex-based, not semantic |
| Query API (14 typed queries) | TESTED | `repo-context` CLI; R3 claimed but audit pending |
| Context retrieval (ranked) | TESTED | P@10=0.48, R@10=1.00; deterministic |
| AST navigation (tree-sitter) | TESTED | `zylcode-nav` crate, 8207 LOC |
| Architecture detection | SCAFFOLDED | `intelligence/architecture.rs` exists |
| Dependency graph | SCAFFOLDED | `intelligence/dependency.rs` exists |
| Canvas/visual design | SCAFFOLDED | `intelligence/canvas.rs` exists |

---

## 8. Existing Software Factory Capabilities

| Capability | State | Evidence |
|---|---|---|
| TaskGraph (dependency-aware DAG) | TESTED | `factory/graph.rs` — structural validation, cycle detection |
| FactoryRunner (real tool dispatch) | TESTED | `factory/runner.rs` — dispatches through `zylcode_mcp::dispatch` |
| Job lifecycle (INTAKE → DELIVERY) | TESTED | `factory/job.rs` — 5 stages |
| JobStore (durable on-disk) | TESTED | `factory/store.rs` — atomic writes, recovery |
| Evidence integration | WIRED | Records to `EvidenceGraph`, `ClaimStore`, `Failure` taxonomy |
| Approval gating | TESTED | Structural dependency on `approve` node; order-independent |
| Best-of-N patch verification | TESTED | `patch_best_of_n.rs` — worktree isolation |

**Limitation:** The factory is proven in isolation but not yet integrated into the agent loop as the primary execution path. A mission (linear FIFO) and a factory job (DAG) coexist but are not unified.

---

## 9. Existing AI Engineer / Session Capabilities

| Capability | State | Notes |
|---|---|---|
| AI Input System (text/file/voice/image) | SCAFFOLDED | `ai_input/` module exists; voice processor is simulated; image/clipboard not implemented |
| Session persistence | TESTED | `memory_ledger.rs` — durable conversation state |
| Task planning | WIRED | `planner.rs` exists; not product-reachable |
| Context builder | TESTED | `context_builder.rs` — assembles repo context for prompts |
| Pipeline (artifact parsing) | TESTED | `pipeline.rs` — parses `<zylcode-response>` XML envelope |
| Artifact Bus | TESTED | `artifact_bus.rs` — versioned, content-hashed, forward-only lifecycle |

---

## 10. Existing Provider Architecture

| Aspect | State |
|---|---|
| Provider enum | 5 variants: OpenRouter, DeepSeek, Anthropic, LocalOllama, SyntheticOffline |
| Streaming | Configured but not independently verified |
| Tool calling | Not implemented |
| Multimodal | Not implemented |
| Context limits | Not enforced |
| Error handling | Honest (timeout, auth, quota, unavailable states) |
| Retry logic | Basic retry with exponential backoff |
| Cancellation | Token-based cancellation exists |
| Provider health | `NatlasResilienceState` pattern (on competition branch only) |

**Critical:** The main branch lacks the `NatlasResilienceState` pattern developed on the competition branch. Provider failure on main falls through synthetic-degradation paths in `router.rs`.

---

## 11. Existing MCP / Tool Capabilities

| Capability | State |
|---|---|
| Filesystem (read/write/list) | RUNTIME_VERIFIED |
| Shell execution | RUNTIME_VERIFIED |
| Git operations | RUNTIME_VERIFIED |
| Search | RUNTIME_VERIFIED |
| Built-in plugins | 38 defined, 12 executable, 2 tested, 11 product-reachable, 0 R3-verified |
| Skills system | SCAFFOLDED — framework exists, no active skills |
| Plugin marketplace | SCAFFOLDED — local module exists, no hosted registry |
| Hot reload | SCAFFOLDED |
| Actor identity | TESTED — field exists but all production sites pass `None` |

---

## 12. Existing Permission / Governance Architecture

| Level | Implementation |
|---|---|
| READ | Implicit (file read, directory list) |
| PROPOSE | Agent generates plan; human reviews |
| WRITE | `fs.write` governed by workspace containment + path traversal checks |
| EXECUTE | Shell commands gated by `PermissionPolicy` |
| DESTRUCTIVE | Explicit risk classification (High = requires approval) |
| EXTERNAL/PUBLISH | Not implemented (no deploy path exists yet) |

**Evidence:** `zylcode_mcp::permission` (4 tests), `factory::runner` approval gating (14 tests in bridge suite on competition branch).

---

## 13. Current UI/UX Capability

| Component | State | Notes |
|---|---|---|
| Desktop shell (4-zone layout) | TESTED | Menu, activity rail, main editor, bottom panel |
| Editor pane | UI_ONLY | Read works; write path STUB (G-03) |
| File explorer | RUNTIME_VERIFIED | `/api/files` serves real tree |
| Command palette | UI_ONLY | No backend dispatcher bound |
| Source control panel | RUNTIME_VERIFIED | Read-only git status |
| Terminal panel | UI_ONLY | Placeholder UI, no PTY |
| Problems panel | UI_ONLY | Placeholder |
| Tests panel | UI_ONLY | Placeholder |
| Evidence panel | UI_ONLY | Placeholder |
| Mission composer | WIRED | UI exists, backend queue exists |
| Settings | UI_ONLY | Static panels |
| Themes | TESTED | 8 premium themes |
| Delivery panel | WIRED | Real pipeline UI; build/release works |
| Repo Intel panel | WIRED | Real data, UI renders |

**Frontend test status:** 54 vitest tests reported passing (2026-09-28); not re-executed in this session.

---

## 14. Persistence Architecture

| Store | Format | State |
|---|---|---|
| Project store | JSON (`projects.json`) | TESTED — schema v1, migration, recovery |
| Mission queue | JSON (`missions.json`) | TESTED |
| Evidence ledger | JSONL (`ledger.jsonl`) | RUNTIME_VERIFIED — hash-chained |
| SQLite vector cache | SQLite (`vector_cache.db`) | WIRED — exists, used by router |
| Provider scorecard | JSON | WIRED |
| Factory job store | JSON (per-job) | TESTED — atomic writes |
| Artifact Bus | Content-addressed files | TESTED — SHA-256 pinned |

---

## 15. Evidence / Audit Architecture

| Component | State |
|---|---|
| Evidence graph (traversable provenance) | TESTED |
| Claim store (epistemic claims with promotion) | TESTED |
| Failure taxonomy (typed failures + recovery) | TESTED |
| Audit trail (tool invocations) | RUNTIME_VERIFIED |
| Redaction (secrets scrubbed before storage) | RUNTIME_VERIFIED |
| Chain hash (tamper detection) | TESTED — `chain_hash` computed post-serialization |

---

## 16. Known Mocks / Simulations / Stubs

| Location | What | Severity |
|---|---|---|
| `computer_use/mod.rs` | Every perception returns fabricated data; every action is a timer | **R0 — non-functional skeleton** |
| `ai_input/voice_processor.rs` | "Simulated transcript"; never delegates to real ASR | SCAFFOLDED |
| `cache.rs::mock_embed` | Deterministic mock embedding for hermetic tests | Test-only, correctly isolated |
| `router.rs` | `SyntheticOffline` returns placeholder responses | **First-class provider by design**, not a degradation path |
| `builtin_plugins.rs` | Many plugins defined but not executable | 38 defined, 12 executable |
| `enhanced_plugin_marketplace.rs` | Print-only stubs ("queued for installation" without installing) | STUB |
| `cli.rs::MarketplaceCommands` | Install/Publish print success without action | STUB |
| `agent.rs` (line 2034) | `chain_hash: String::from("TODO")` | **Known defect** — literal "TODO" in production path |
| `telemetry.rs` | HTTP/SSE context propagation is empty placeholder | STUB |

---

## 17. Known Blockers

| Blocker | Impact | Resolution Path |
|---|---|---|
| **Workspace compilation fails** | **CRITICAL** — prevents all development | Fix `schemars`/`indexmap` version conflict in `Cargo.lock` or `Cargo.toml` |
| Phase 2A NOT ACCEPTED | Blocks 2B and downstream intelligence work | Re-audit required; remediation code shipped |
| AGENT-01 (no usable provider) | Agent loop cannot execute real model calls | Requires real API credentials or local Ollama setup |
| GUI write path broken | Editor cannot save files through UI | Tauri command `save_workspace_artifact` not registered |
| Computer-Use Engine R0 | Non-functional skeleton | Replace, do not extend (per Constitution v2.1) |

---

## 18. Architectural Debt

| Debt | Impact |
|---|---|
| `schemars` version conflict | **Blocks all compilation** |
| Mission queue (linear FIFO) vs Factory job (DAG) are not unified | Two execution paths with different semantics |
| Router has silent synthetic-degradation paths | Provider failures can silently downgrade to another provider |
| `agent.rs` contains literal `"TODO"` chain hash | Production code carries known placeholder |
| Frontend write path is an orphan | Editor reads via HTTP but saves via non-existent Tauri command |
| Actor identity field exists but always `None` | No production site sets actor identity |
| Evidence returned and dropped, not persisted | `ToolEvidence` produced but not stored (competition branch fixed this) |

---

## 19. Highest-Value Next Vertical

**Fix the workspace compilation failure.**

This is not a feature. It is a prerequisite for every other piece of work. Without a compiling workspace:
- No tests can run
- No new code can be validated
- No clippy feedback
- No CI can pass
- No developer can contribute

The fix is likely a one-line `Cargo.toml` or `Cargo.lock` adjustment (pinning `schemars` to `0.9.0` or updating `indexmap` resolution).

---

## 20. Why That Vertical is Next

The dependency resolution failure is a **complete blocker**. It is not a missing feature or a quality issue — it is a build-breaker that prevents any engineering work from being validated. All other verticals (factory integration, GUI write path, provider health, Computer-Use Engine replacement) require a compiling workspace.

---

## 21. Exact Proposed Scope

1. **Diagnose** the exact `schemars`/`indexmap` conflict in `Cargo.lock`
2. **Fix** by either:
   - Updating `schemars` dependency to `0.9.0` (compatible with `indexmap 2.x`)
   - Or pinning `indexmap` to a version compatible with `schemars 0.8.22`
   - Or removing the `schemars 0.8.22` dependency path
3. **Verify** `cargo check --workspace` passes
4. **Verify** `cargo test --workspace` passes
5. **Verify** `cargo clippy --workspace --all-targets` passes
6. **Record** the exact fix and dependency versions

---

## 22. Explicit Exclusions

- No new features
- No UI changes
- No provider changes
- No N-ATLAS competition code (that branch is frozen)
- No destructive git operations
- No Cargo.lock wholesale regeneration (targeted fix only)

---

## 23. Proposed Implementation Sequence

| Step | Action | Verification |
|---|---|---|
| 1 | Inspect `Cargo.lock` for schemars/indexmap dependency graph | `grep -A10 "schemars" Cargo.lock` |
| 2 | Identify which crate pulls in `schemars 0.8.22` | `cargo tree -i schemars:0.8.22` |
| 3 | Apply targeted fix (Cargo.toml pin or lockfile edit) | `cargo check --workspace` passes |
| 4 | Run full test suite | `cargo test --workspace` passes |
| 5 | Run clippy | `cargo clippy --workspace --all-targets` passes |
| 6 | Commit with reproduction block | Record commands, env, output, SHA |

---

## 24. Proposed Test / Acceptance Matrix

| Test | Expected | After Fix |
|---|---|---|
| `cargo check --workspace` | Pass | Must pass |
| `cargo test --workspace` | All tests pass | Must pass |
| `cargo clippy --workspace --all-targets` | Zero errors | Must pass |
| `cargo test -p zylcode-core --lib` | Core lib tests pass | Must pass |
| `cargo test -p zylcode-mcp --lib` | MCP tests pass | Must pass |
| Frontend `pnpm test` | Not in scope for this fix | N/A |

---

## Classification

**MAIN_CONTINUATION_BLOCKED**

The workspace compilation failure (`schemars 0.8.22` vs `indexmap 1.9.3`) is a complete blocker. No test execution, no feature work, no validation is possible from a clean checkout until this is resolved.

**Smallest genuine owner decision required:** Authorize a targeted dependency fix in `Cargo.toml`/`Cargo.lock`. No architecture change. No feature work. Just restore compilability.

---

## Competition Branch Protection

| Check | Result |
|---|---|
| `competition/natlas-2026` frozen | ✅ RC1 at `1f80322` — not modified |
| No merge/cherry-pick/rebase attempted | ✅ |
| Main work via separate worktree | ✅ `C:/Temp/zylcode-main-forensics` |
| Original dirty tree preserved | ✅ All 14 foreign entries untouched |
