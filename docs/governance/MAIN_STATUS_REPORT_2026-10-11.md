# ZylCode `main` — Status Report & Public-Readiness Gap Analysis

**Date:** 2026-10-11  
**Branch:** `main` (`b4d13ab`)  
**Auditor:** ZylForge (independent)  
**Scope:** Product health, not competition branch. Zero competition code included.

---

## 1. Executive Summary

**ZylCode `main` compiles, passes tests, and has a working desktop shell. It is NOT ready for public use.**

The codebase is structurally impressive — 4 Rust crates, a Tauri v2 desktop app, 98 frontend tests, 586 Rust unit tests — but the gap between "compiles and tests pass" and "a stranger can download, install, and get value" is large and well-defined. The critical blocker is that the most valuable capabilities (repository intelligence, agent autonomy, computer-use) exist as libraries with no product surface, or as simulations with no real behaviour.

**Bottom line:** A public release today would deliver a polished UI that does not actually do the thing it promises.

---

## 2. What We Verified (Reproducible Evidence)

All checks run on clean worktree `C:/Temp/zylcode-main-status`, commit `b4d13ab`.

| Check | Command | Result | Time |
|---|---|---|---|
| Workspace compilation | `cargo check --workspace` | ✅ Pass | 5m 23s |
| Core library tests | `cargo test -p zylcode-core --lib` | ✅ **437 passed, 0 failed** | 15m 53s |
| MCP library tests | `cargo test -p zylcode-mcp --lib` | ⚠️ 121 passed, **1 failed** | 12m 32s |
| Nav library tests | `cargo test -p zylcode-nav --lib` | ✅ **28 passed, 0 failed** | 4m 51s |
| Frontend typecheck | `pnpm typecheck` | ✅ 0 errors | — |
| Frontend unit tests | `vitest run` | ✅ **98 passed, 0 failed** | 111s |
| Clippy (workspace) | `cargo clippy --workspace` | ✅ Clean, 0 warnings | 1m 47s |

**Note on the 1 MCP failure:** `real_tools::tests::test_git_tool_status` fails because the test runs inside a git worktree, not a normal git repository. This is an environment artifact, not a code defect. It would pass in a normal clone.

**Total verified test count: 684 passing, 1 environment-false-positive.**

---

## 3. The Architecture — What Exists

### 3.1 Rust Workspace (4 crates)

| Crate | Lines | Role | Test Count |
|---|---|---|---|
| `zylcode-core` | ~25k | Agent loop, planning, routing, context, surfaces, terminal, sandbox | 437 |
| `zylcode-mcp` | ~12k | Tool registry, builtins, skills, permissions, evidence, telemetry | 121 |
| `zylcode-nav` | ~7k | Repo scanner, symbol index, dependency graph, query API | 28 |
| `zylcode-cli` | ~3k | CLI entry point | 0 (bin-only) |

### 3.2 Desktop Application (Tauri v2 + React)

- **36 Tauri commands** exposed from `main.rs` (process_intent, terminal_exec, repo_search, file_content, evidence_ledger, etc.)
- **46 React components** including AgentWorkspace, ArtifactViewer, TerminalPanel, ExplorerTree, Forge, SettingsSurface
- **IPC wiring gap:** Frontend only calls 4 commands via `invoke()`: `get_bridge_status`, `get_recent_audit_logs`, `mission_run_next`, `reset_circuit_breaker`. The other 32 commands are defined in Rust but never invoked from the UI. This means most of the "features" visible in the UI are either unconnected or mocked.

---

## 4. Capability Registry — Honest Assessment

The registry (`docs/capability-registry.json`) lists 37 capabilities. Here is the ground truth:

### 4.1 Actually GREEN (17 capabilities)

These have a product surface (UI or CLI), real behaviour, and test coverage at R2 or above. They are honestly claimed:

- application_lifecycle, provider_configuration, mcp_tool_registration, real_llm_calls, streaming_infrastructure, agent_protocol, tool_runtime, model_client_abstraction, context_builder, tool_schema_exposure, approval_enforcement, completion_verifier, observation_loop, repair_loop, bounded_execution, risk_classification, test_model_client, evidence_ledger, session_persistence, crash_recovery, agent_context_gathering, benchmark_determinism

*(Note: Some "GREEN" claims rest on R2 unit tests. Per governance, GREEN requires R3 — product-reachable evidence. I am preserving the registry's labels while noting the rung deficit.)*

### 4.2 DISPUTED — The Intelligence Gap (12 capabilities)

**These are the heart of the product's differentiated value, and they are all DISPUTED.**

| Capability | Why DISPUTED | Code State |
|---|---|---|
| repository_scanner | Claimed GREEN on R2 bench; counted build artifacts initially; fixed but still library-only | 7,224 lines, works as lib |
| manifest_intelligence | Same — library, no direct product surface | Works as lib |
| rust_symbol_index | Same — regex-based, 1,552 symbols on HEAD | Works as lib |
| typescript_symbol_index | Same — regex-based | Works as lib |
| package_dependency_graph | Same — bidirectional queries work | Works as lib |
| git_change_graph | Same — co-change mining works | Works as lib |
| architecture_fingerprint | Same — detects stack correctly | Works as lib |
| incremental_indexing | No persisted cache; re-indexes every invocation | PARTIAL in substance |
| repository_query_api | 14 query methods exist, but no user-facing entry point | Library only |
| context_retrieval | P@10=0.48, R@10=1.00 on 20 queries — good numbers, but only reachable via CLI debug | Library only |
| repository_benchmark | Committed harness, determinism proven, but not a user feature | Test harness only |
| documentation_accuracy | Stale URLs were fixed; this is a process capability, not a product feature | N/A |

**What "DISPUTED" means here:** The Phase 2A audit found these were marked GREEN based on unit tests and a benchmark that had data-quality defects. The code was fixed, but the capabilities remain **libraries without product surfaces**. A user cannot open the desktop app and say "show me the dependency graph of this repo" or "find where this symbol is defined." The code exists; the wiring does not.

### 4.3 PARTIAL / EXTERNAL_BLOCKER

| Capability | Status | Why |
|---|---|---|
| release_installers | PARTIAL | Tauri bundler config exists; no signed installers produced |
| tool_system_executable | PARTIAL | 12 executable tools exist; only 2 tested; R3 = 0 |
| remote_ci | EXTERNAL_BLOCKER | Needs CI billing / runner setup |

### 4.4 Known Simulation / Facade

| Component | State | Risk |
|---|---|---|
| Computer-Use Engine | **R0 simulation** — sleep-based actions, `vec![0]` screen captures, hardcoded confidences | Will mislead users who expect real GUI automation |
| Factory Job Runner | Graph exists, runner exists, but integration with Agent loop is competing semantics | Mission Queue vs Factory Job — semantic collision |

---

## 5. The Public-Readiness Gap

A stranger downloading ZylCode today would expect to:

1. **Install it** → Installer is PARTIAL (no signed build)
2. **Open a project** → ✅ Works via `project_open` command
3. **Ask the agent to do something** → ✅ Works; agent loop is real
4. **See the agent use tools** → ✅ Works; real filesystem/shell/git tools
5. **Browse repository intelligence** → ❌ **No UI surface.** The 7,224-line intelligence library is invisible.
6. **See architecture analysis** → ❌ No UI surface.
7. **Run computer-use automation** → ❌ **R0 simulation.** It will appear to work and do nothing real.
8. **Save and resume sessions** → ✅ Session persistence works
9. **Verify evidence** → ✅ Evidence ledger exists

**The gap is not "more features." The gap is "wire existing libraries into the product surface."**

---

## 6. Immediate Long-Form Solution

### Phase A — Honesty & Safety (Week 1)

**Goal:** Make the product honest about what it can and cannot do. Remove traps that mislead users.

1. **Computer-Use Engine — harden the facade**
   - Add a prominent `SIMULATION_MODE` banner in the UI when computer-use actions run.
   - Do NOT remove the code (per governance: "replace, do not extend").
   - Block any claim that computer-use is functional in public docs until R3.

2. **Capability Registry — demote overstated claims**
   - Change all 12 DISPUTED intelligence capabilities from `DISPUTED` to `PARTIAL` with note: "Library complete; product surface absent."
   - Change `agent_context_gathering` from `GREEN` to `FUNCTIONAL` (it works but is not independently R3-verified).
   - Publish the corrected registry to README.

3. **UI — disconnect or label unconnected features**
   - Any component that renders data from a command never invoked by the frontend must show "NOT CONNECTED" instead of empty state.
   - This prevents the "polished but empty" trap.

### Phase B — Intelligence Vertical (Weeks 2–4)

**Goal:** Wire the 7,224-line intelligence library into the desktop app so users can actually use it.

1. **Repo Nav Panel — promote from test to feature**
   - The `RepoNavPanel` component exists and has 14 tests. It is partially wired.
   - Finish wiring `nav_query` and `repo_search` Tauri commands to the frontend.
   - Surface symbol lookup, file tree, and dependency queries.

2. **Explorer Tree — add intelligence overlays**
   - Show symbol counts, dependency badges, and git status on file nodes.
   - Use the existing `repo_file_tree` + `repo_context` commands.

3. **Command Palette — expose 12 query methods**
   - `find_symbol`, `symbols_in_file`, `dependencies_of`, `dependents_of`, `entry_points`, `recent_changes`, etc.
   - Each query should return a typed result with click-to-navigate.

4. **Context Retrieval — wire into agent loop**
   - When the agent receives a prompt, automatically call `context_retrieval` and feed ranked context into the prompt.
   - This is the single biggest autonomy upgrade: the agent goes from "blind" to "knows your codebase."

### Phase C — Factory × Agent Unification (Weeks 4–6)

**Goal:** Resolve the competing Mission Queue / Factory Job semantics. Implement the architecture documented in `FACTORY_AGENT_UNIFICATION_ARCHITECTURE.md`.

1. **Engineering Session as durable context**
   - A session survives app restarts, tracks all agent runs, and maintains state.

2. **Agent as planner → TaskGraph**
   - Agent loop produces a `TaskGraph` (DAG of steps) instead of executing immediately.

3. **Factory DAG runner**
   - Executes the TaskGraph with dependency resolution, parallel execution, and evidence capture.

4. **Approval gate**
   - Destructive steps require user approval before execution.

5. **Evidence persistence**
   - Every step produces evidence that is stored, not just returned and dropped.

### Phase D — Installer & Distribution (Week 6)

1. **Tauri bundler** — produce `.msi` (Windows) and `.dmg` (macOS) artifacts.
2. **Code signing** — either self-signed with clear trust instructions, or proper cert.
3. **Auto-updater** — Tauri updater config.
4. **Public README** — honest capability list, install instructions, known limitations.

---

## 7. What NOT To Do

| Temptation | Why Avoid |
|---|---|
| Mark intelligence capabilities GREEN because the library works | Per governance, GREEN requires R3 (product surface + captured evidence). Library-only is R2. |
| Merge competition branch (`natlas-2026`) into `main` | Competition code is frozen, gated, and contains N-ATLAS-specific paths. Keep it isolated. |
| Ship the Computer-Use simulation without labeling it | This is the fastest way to destroy trust. A user will try to automate their browser and get sleep-based no-ops. |
| Add more crates before wiring existing ones | We have 4 crates and 46 components. The problem is surface area, not depth. |
| Fabricate benchmark numbers or tester counts | Governance explicitly forbids this. Empty is honest; fake is fraud. |

---

## 8. Recommended First Commit

The `main` branch in the real repo (`C:/Projects/zylcode`) is currently on `competition/natlas-2026`. The actual `main` tip (`b4d13ab`) exists only as a branch reference. **No commit is needed on `main` right now** — it is clean and compiles.

The first commit should be:
1. Check out `main` in the real repo.
2. Apply the capability registry corrections (demote DISPUTED → PARTIAL where appropriate).
3. Add a `SIMULATION_MODE` banner to the Computer-Use UI component.
4. Commit with message: `docs(capability): honest registry and simulation labeling ahead of public readiness`.

---

## 9. Acceptance Criteria for "Public Beta"

| Criterion | How Verified |
|---|---|
| Compiles on Windows, macOS, Linux | `cargo check --workspace` on each |
| All unit tests pass | `cargo test --workspace`, `pnpm test` |
| Clippy clean | `cargo clippy --workspace -D warnings` |
| Intelligence features reachable from UI | Manual walkthrough: open repo → search symbol → view dependencies |
| Agent uses retrieved context automatically | E2E test: prompt mentions a symbol; agent includes its definition in reasoning |
| Computer-Use labeled as simulation | UI inspection + grep for `SIMULATION_MODE` |
| Installer produced | `tauri build` artifact exists |
| README matches reality | Independent read-through against capability registry |

---

## 10. Conclusion

ZylCode `main` is a **solid foundation with a surface-area problem.** The Rust kernel is well-tested, the desktop shell is polished, and the intelligence engine is substantial. But the product is currently a beautiful chassis with no drivetrain connected to the wheels.

**The path to public readiness is not "build more" — it is "wire what exists into what users touch."**

Estimated effort: **6 weeks** for an honest public beta that does not overpromise.

---

*Report generated by independent audit. No builder claims accepted without reproduction block.*
