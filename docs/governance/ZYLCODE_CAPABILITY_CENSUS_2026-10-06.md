# ZYLCODE CAPABILITY CENSUS — 2026-10-06

**Companion (machine-readable):** `docs/capability-census-2026-10-06.json`
**Method:** evidence-first. Status is derived from source inspection, committed tests and executed
commands in this working tree on 2026-10-06 — **never from a file name, a module's existence, or a
historical claim.**
**Baseline:** `main` @ `aae6b4e4ffd939cd09abb6d7345ae1a762f7ba03`, `origin/main` identical,
ahead/behind `0/0`, 14 dirty entries, 0 stashes, 475 tracked files.

## Status vocabulary

| Status | Means |
|---|---|
| `NOT_IMPLEMENTED` | No source, no surface, no behaviour. Includes planned-only. |
| `SCAFFOLDED` | Types or a module skeleton exist; no reachable behaviour. |
| `UI_ONLY` | A surface renders; no real backend path is reachable from it. |
| `WIRED` | A real backend path exists and is reachable from a named surface, but no executed test in this session proves it. |
| `TESTED` | Exercised by a committed test that executed in this session's battery. |
| `RUNTIME_VERIFIED` | Exercised against a real process/network/filesystem, outcome captured. |
| `BLOCKED` | Cannot be reached for a stated reason. **Never** a synonym for "not done". |
| `DEPRECATED` | Superseded; retained as provenance. |

**Two honesty rules applied throughout.** (1) A frontend vitest suite was **not executed** in this
session (the battery was Rust-only); entries resting on it say so. (2) `r3_verified_count == 0` in
the tool catalogue is pinned by a test; no entry claims a higher rung than the evidence supports.

---

## Census summary

| | |
|---|---|
| Categories | **22** (A–V) |
| Capabilities recorded | **86** |
| `RUNTIME_VERIFIED` | 6 |
| `TESTED` | 47 |
| `WIRED` | 5 |
| `SCAFFOLDED` | 4 |
| `UI_ONLY` | 2 |
| `NOT_IMPLEMENTED` | 19 |
| `BLOCKED` | 3 |
| `DEPRECATED` | 0 |

**The shape of the truth:** ZylCode's *evidence, tooling, repository-intelligence and deterministic
execution* spine is real and tested. Its *model-dependent* spine is blocked. Its *factory,
research, institutional and security* surfaces are designed, not built.

---

## A. IDE / EDITOR

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| A-01 | Desktop shell (four-zone IDE layout) | `TESTED` | A-02, T-01 | `components/shell/*` + `SurfaceHost.test.tsx`; 54 vitest tests reported 2026-09-28 | Frontend suite not executed this session | TESTED |
| A-02 | Editor pane (view + tabbed edit) | `UI_ONLY` | B-02 | Read path real via `/api/file-content`; save routes to `save_workspace_artifact`, a command absent from `generate_handler` (gap G-03) | **GUI write does not work.** Read does. | RUNTIME_VERIFIED |
| A-03 | Command palette | `UI_ONLY` | — | `CommandPalette.tsx`; no backend dispatcher bound | Navigational only | WIRED |
| A-04 | Source control panel | `RUNTIME_VERIFIED` | J-01 | `gitops::git_status_payload` reads real git; 5 committed tests | Read-only; no stage/commit from the panel | RUNTIME_VERIFIED |

## B. PROJECT & WORKSPACE

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| B-01 | Secure local project filesystem | `TESTED` | — | `project/filesystem.rs` (10 tests); canonicalise containment | Desktop IPC path is an orphan file (G-02) | RUNTIME_VERIFIED |
| B-02 | Workspace file read/tree service | `RUNTIME_VERIFIED` | B-01 | `/api/files`, `/api/file-content` served live | No HTTP write route (by design today) | RUNTIME_VERIFIED |
| B-03 | Project store (persistent identity) | `TESTED` | — | `project_store.rs` (634 LOC), commit `56cf3e5` | No product surface enumerates projects | WIRED |
| B-04 | Git worktree isolation for candidates | `TESTED` | J-02 | `sandbox.rs` + `patch_best_of_n.rs`; mount-policy tests | Container path needs a runtime; unused by the factory today | RUNTIME_VERIFIED |

## C. REPOSITORY INTELLIGENCE

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| C-01 | Repository scanner + persisted index | `RUNTIME_VERIFIED` | — | `intelligence/{scanner,persisted,store}.rs`; live "444 files, 2111 symbols, 6 packages"; Windows exclusion defect fixed + trap-file asserted | Per-invocation indexing ~10-12s | RUNTIME_VERIFIED |
| C-02 | Symbol index (Rust + TS) | `TESTED` | C-01 | `symbols.rs`; 1552 symbols on HEAD | Regex-based; `DEFINITION_INDEXED`, not semantic | RUNTIME_VERIFIED |
| C-03 | Repository query API (14 queries) | `TESTED` | C-01, C-02 | `query.rs`; `zylcode repo-context` CLI + committed transcript | R3 is **builder-claimed**; Phase 2A remains RE-OPENED | RUNTIME_VERIFIED |
| C-04 | Context retrieval (ranked, deterministic) | `TESTED` | C-02, C-03 | 20-query benchmark P@10=0.48 R@10=1.00; 10 identical cross-process runs | Precision 0.48 is honest and modest | RUNTIME_VERIFIED |
| C-05 | AST navigation engine (tree-sitter) | `TESTED` | — | `crates/zylcode-nav` (8207 LOC, 9 modules); `tests/navigation.rs` | Not yet wired into the factory graph | RUNTIME_VERIFIED |
| C-06 | Impact analysis ("what will this affect?") | `TESTED` | C-03, C-05 | `dependency.rs` + nav queries; Q6 P@10=1.00 | No first-class impact *artifact* | WIRED |

## D. AI PROVIDERS

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| D-01 | Provider registry + capability model | `TESTED` | — | `router.rs`, `model_capabilities.rs`, `provider_scorecard.rs` | Declarative; proves no provider answers | RUNTIME_VERIFIED |
| D-02 | Token router with real HTTP dispatch | `TESTED` | D-01 | `router.rs`; `live_commissioning` recorded 3 real HTTP outcomes | Real HTTP, no successful completion | RUNTIME_VERIFIED |
| D-03 | **Usable approved cloud provider** | `BLOCKED` | D-02 | OpenRouter/Anthropic **401**, DeepSeek **402**, OpenRouter key is a literal placeholder | **AGENT-01 stays BLOCKED.** No model-dependent gate may pass. | RUNTIME_VERIFIED |
| D-04 | Synthetic offline provider | `TESTED` | D-02 | `SyntheticOffline` used by tests/benches | Gap G-01 open: an on-disk vector cache can answer a live prompt with a SYNTHETIC response | TESTED |

## E. AGENT RUNTIME

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| E-01 | Agent kernel loop | `TESTED` | H-01, D-02, G-01 | `agent.rs`; `agent_loop_e2e.rs` (4 tests) | Live-model path **BLOCKED** | RUNTIME_VERIFIED |
| E-02 | Agent decision protocol | `TESTED` | — | `agent_protocol.rs`; `decision_proptest.rs` | Decision *quality* unmeasurable while D-03 blocked | RUNTIME_VERIFIED |
| E-03 | Builder/verifier separation | `TESTED` | K-01 | `best_of_n.rs`, `patch_best_of_n.rs` | Candidate generation is model-dependent | RUNTIME_VERIFIED |
| E-04 | Context builder for the agent | `TESTED` | C-04 | `context_builder.rs`; discriminating ranking test | Per-invocation re-index ~10s | RUNTIME_VERIFIED |

## F. AGENT ORCHESTRATION

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| F-01 | Mission queue (durable) | `TESTED` | G-01 | `missions.rs` (10 tests); `/api/missions` 200 | **Linear queue, not a graph** | RUNTIME_VERIFIED |
| F-02 | Deterministic plan mode | `TESTED` | C-04 | `missions::build_plan`; test asserts real symbols | Model-authored plan **BLOCKED** | RUNTIME_VERIFIED |
| F-03 | **Dependency-aware task graph** | **`TESTED`** | F-01, G-01 | **Created this wave:** `factory/graph.rs` — DAG, validation, readiness, orphan propagation, 9 unit tests + integration coverage | Executes sequentially; parallelism is DESIGNED | TESTED |
| F-04 | **Factory job lifecycle (durable)** | **`TESTED`** | F-03 | **Created this wave:** `factory/job.rs` — 15-stage forward-only projection, 7 unit tests | Stage automation beyond the slice is DESIGNED | TESTED |
| F-05 | Parallel agents over isolated worktrees | `SCAFFOLDED` | B-04 | Worktree primitives exist; no scheduler, leases or merge queue | Orchestration absent | WIRED |

## G. MEMORY

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| G-01 | Execution ledger (hash-chained, durable) | `TESTED` | — | `ledger.rs`, `memory_ledger.rs`, `sqlite_ledger.rs`; `crash_recovery.rs`; live `ledger.db` | Execution-scoped, not project/decision memory | RUNTIME_VERIFIED |
| G-02 | Crash recovery / reconciliation | `TESTED` | G-01 | `agent.rs` recovery; real `abort()`+resume in `first_mission_kill_resume.rs` | No general job-level recovery service | RUNTIME_VERIFIED |
| G-03 | Decision memory (ADR-style) | `NOT_IMPLEMENTED` | G-01 | No decision store exists | Design-only | TESTED |
| G-04 | Memory scope separation | `NOT_IMPLEMENTED` | G-01 | Only session-scoped memory exists | Design-only | TESTED |

## H. TOOLS

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| H-01 | Executable tool catalogue | `TESTED` | — | `tool_catalogue.rs` (881 LOC); 5 metrics; `r3_verified_count == 0` pinned | 38 definitions vs 12 executors — never conflate | RUNTIME_VERIFIED |
| H-02 | Real tool executors | `TESTED` | H-01 | `real_tools.rs`; `integration_test.rs` (11 tests); `git.commit` cannot push | 12 of 38 definitions executable | RUNTIME_VERIFIED |
| H-03 | Repository-intelligence tools (`nav.*`) | `TESTED` | C-05 | `nav_tools.rs`; `tests/nav_tools.rs` | Read-only by construction | RUNTIME_VERIFIED |
| H-04 | Permission gate + dispatch | `TESTED` | — | `permission.rs`, `dispatch()`; `decision_proptest.rs` | Policy is per-runtime; a job chooses its ceiling | RUNTIME_VERIFIED |
| H-05 | Tool execution evidence (JSONL) | `TESTED` | H-04 | `evidence.rs`; `executable_builtins_evidence.rs` | Until this wave, not linked into a graph by the tool path | RUNTIME_VERIFIED |

## I. MCP

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| I-01 | MCP tool registry + dynamic tools | `TESTED` | H-01 | `registry.rs`, `tool.rs`; `mcp.tools.yaml` tracked | TOOL-01 closed at `1a0c79d` | RUNTIME_VERIFIED |
| I-02 | MCP bridge (external transport) | `SCAFFOLDED` | I-01 | `register_mcp_bridge` validates + stores descriptors; **no outbound client demonstrated** | **Do not describe as a working MCP client** | WIRED |
| I-03 | Config hot-reload | `TESTED` | I-01 | `hot_reload.rs`, `install_global_watcher` | `clear()`+re-register; invariant re-established per run | TESTED |

## J. TERMINAL / PROCESS EXECUTION

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| J-01 | Terminal hub (real shell) | `TESTED` | H-02 | `terminal.rs`; commit `aa76f71`; `/api/terminal/reset` 200 | No PTY-interaction guarantees documented | RUNTIME_VERIFIED |
| J-02 | Process execution with captured outcome | `TESTED` | — | `run_cmd`/`CmdOutcome`; `first_mission_e2e.rs` | No resource limits beyond timeout on the plain path | RUNTIME_VERIFIED |

## K. TESTING

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| K-01 | Workspace test battery | `RUNTIME_VERIFIED` | — | 575/0/0 recorded 2026-09-28; **re-run 2026-10-06: 730/0/1 across 25 groups, exit 0** (the 1 ignored is the live provider probe) | Remote CI **BLOCKED** (billing lock) | RUNTIME_VERIFIED |
| K-02 | Property-based tests | `TESTED` | — | `decision_proptest.rs`, `pipeline_proptest.rs` | Narrow coverage (2 subsystems) | RUNTIME_VERIFIED |
| K-03 | Fault-injection suite | `TESTED` | H-04 | `fault_injection.rs` | Adversarial *evidence* tests are PARTIAL | RUNTIME_VERIFIED |
| K-04 | Agent behaviour evaluation framework | `NOT_IMPLEMENTED` | E-01 | No harness; First Mission is one scenario, not a scored framework | Design-only | TESTED |

## L. DEBUGGING

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| L-01 | Failure diagnosis from captured output | `TESTED` | L-02, G-01 | `diagnose_from_output()`, `implicated_file()` | Signature-based; model diagnosis **BLOCKED** | RUNTIME_VERIFIED |
| L-02 | Structured failure taxonomy | `TESTED` | — | `failure.rs` (5 tests); recovery is fail-closed | Not yet a first-class UI object | WIRED |
| L-03 | Interactive debugger (DAP) | `NOT_IMPLEMENTED` | — | Grep for `debugger|breakpoint|DAP` returns nothing | Not in the 16-phase roadmap | NOT_IMPLEMENTED |

## M. SECURITY

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| M-01 | Permission/risk gate (tool execution) | `TESTED` | H-04 | `permission.rs`; `decision_proptest.rs` | Covers tool execution only | RUNTIME_VERIFIED |
| M-02 | Workspace containment (traversal defence) | `TESTED` | — | `resolve_within` (TOOL-01); `project/filesystem.rs` | A future non-tool write path must reuse it | RUNTIME_VERIFIED |
| M-03 | Secret detection / SAST / dep scanning | `NOT_IMPLEMENTED` | — | No scanner, no SAST, no dependency audit in `crates/` | Design-only | SCAFFOLDED |
| M-04 | Security finding object | `NOT_IMPLEMENTED` | M-03 | No `SecurityFinding` type | Design-only | SCAFFOLDED |

## N. DOCUMENTATION

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| N-01 | Governance corpus | `WIRED` | — | `docs/governance/` indexed by its README | 48 tracked root `.md`; legacy reports quarantined, not archived | WIRED |
| N-02 | Retracted-claim guard | `RUNTIME_VERIFIED` | — | `check_retracted_claims.py` + baseline + CI step; falsification-proven | Runs locally only while CI is locked | RUNTIME_VERIFIED |
| N-03 | Automated documentation generation | `NOT_IMPLEMENTED` | — | No doc-generation pipeline | Design-only | SCAFFOLDED |

## O. RESEARCH

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| O-01 | Research mode workspace | `NOT_IMPLEMENTED` | B-03 | `grep -ri research crates/**/*.rs` → no module | Design-only | SCAFFOLDED |
| O-02 | Reproducible computational research capture | `NOT_IMPLEMENTED` | P-02 | First Mission captures env/hashes; no dataset/seed/result model | The *pattern* exists, the *data model* does not | SCAFFOLDED |
| O-03 | Epistemic typing of research statements | `NOT_IMPLEMENTED` | P-01 | `claim.rs` types engineering claims; no research typing or citations | Design-only | SCAFFOLDED |

## P. SOFTWARE FACTORY

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| P-01 | Claim model with epistemic status | `TESTED` | — | `claim.rs` (13 tests) incl. tamper detection | Two R-ladders share a prefix — name your ladder | RUNTIME_VERIFIED |
| P-02 | Evidence graph (typed, hash-chained) | `TESTED` | P-01 | `evidence_graph.rs` (7 tests) incl. ancestry + tamper | Tool path wrote no graph nodes before this wave | RUNTIME_VERIFIED |
| P-03 | Proof records (deterministic) | `TESTED` | — | `proof_engine.rs`; `.zylcode/proofs.json` | Honest negatives: `Blocked`, `RuntimeNotReached`, `EvidenceMissing` | RUNTIME_VERIFIED |
| P-04 | End-to-end mission runner (First Mission) | `RUNTIME_VERIFIED` | P-01…P-02, G-01, H-02 | `first_mission.rs` (2689 LOC); e2e + real kill/resume | Repair is **specification-directed**, recorded as a HYPOTHESIS | RUNTIME_VERIFIED |
| P-05 | Artifact bus (lifecycle/lineage/retention) | `TESTED` | — | `artifact_bus.rs` (831 LOC) | Time-based retention; no policy engine | RUNTIME_VERIFIED |
| P-06 | **Factory orchestrator** | **`TESTED`** | F-03, F-04, P-02 | **Created this wave:** `factory/runner.rs` executes a dependency-aware graph through the real gated tool runtime, records evidence, resumes, blocks honestly. Was `SCAFFOLDED` before this wave | Role *specialisation* is DESIGNED; the runner executes task kinds | TESTED |

## Q. UI/UX DESIGN

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| Q-01 | Design canvas | `SCAFFOLDED` | — | `intelligence/canvas.rs`; no surface renders it | Vision Studio is **PROPOSED** | WIRED |
| Q-02 | Design tokens / information architecture | `NOT_IMPLEMENTED` | Q-01 | No token model; brand governance is prose | Design-only | SCAFFOLDED |
| Q-03 | Embedded preview surface | `TESTED` | P-05 | Commit `96aa8a4`; RuntimeLab/ArtifactViewer | Previews a static artifact, not a live app runtime | RUNTIME_VERIFIED |
| Q-04 | Accessibility + visual regression | `NOT_IMPLEMENTED` | Q-01 | No a11y test, no visual-regression tool | Design-only | SCAFFOLDED |

## R. COLLABORATION

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| R-01 | Human-in-the-loop approval gates | `TESTED` | F-01 | `missions.rs` approval test; `permission.rs`; **factory per-task gate added this wave** | No risk-tier policy object | RUNTIME_VERIFIED |
| R-02 | Multi-user / team collaboration | `NOT_IMPLEMENTED` | — | No auth, no user model, no server-side tenancy | Single-user desktop today | SCAFFOLDED |
| R-03 | Actor attribution on actions | `TESTED` | H-05 | `actor.rs`; `ToolEvidence::actor`; `process_intent` sets the actor | Task-local; a server needs request identity | RUNTIME_VERIFIED |

## S. GOVERNANCE

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| S-01 | Proof graph R0–R5 ladder | `WIRED` | — | `ZYLCODE_PROOF_GRAPH.md` GOVERNING | **Two live R-ladders** — ambiguity is itself a defect | WIRED |
| S-02 | Capability registry with dispute history | `WIRED` | — | `capability-registry.json`: 37 entries, 12 DISPUTED | Registry is **not accepted as accurate** | WIRED |
| S-03 | Agent operating protocol | `WIRED` | — | `ZYLCODE_AGENT_OPERATING_PROTOCOL.md` | Convention, not machinery | WIRED |
| S-04 | Institutional policy objects | `NOT_IMPLEMENTED` | D-01, H-04 | No policy engine; `PermissionPolicy` is tool-scoped only | Design-only | SCAFFOLDED |

## T. BUILD / RELEASE / DEPLOYMENT

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| T-01 | Build pipeline | `TESTED` | J-02 | `delivery::build_workspace`; commit `bf7d66e`; `/api/build` | Local works; CI locked | RUNTIME_VERIFIED |
| T-02 | Release packaging (installers) | `BLOCKED` | T-01 | `release.yml` configured; Actions jobs execute **zero steps** (billing lock) | **Cannot be claimed as working** | RUNTIME_VERIFIED |
| T-03 | Deployment / DevOps agent | `NOT_IMPLEMENTED` | T-01 | `deploy_targets()` enumerates only | Enumeration is not deployment | SCAFFOLDED |
| T-04 | Remote CI | `BLOCKED` | — | `ci.yml` exists; jobs fail with zero steps | Local battery is the only authority | RUNTIME_VERIFIED |

## U. INSTITUTIONAL / EDUCATION

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| U-01 | Institutional mode | `NOT_IMPLEMENTED` | R-02, S-04 | `grep -ri institution|education|classroom` → nothing | Design-only | SCAFFOLDED |
| U-02 | Education mode | `NOT_IMPLEMENTED` | U-01 | No assistance-level concept | Design-only | SCAFFOLDED |
| U-03 | Research-to-product pipeline | `NOT_IMPLEMENTED` | O-01, P-02 | No pipeline artifact | Design-only | SCAFFOLDED |

## V. LOCAL / PRIVATE / SOVEREIGN

| ID | Capability | Status | Dependencies | Evidence | Known limitations | Target |
|---|---|---|---|---|---|---|
| V-01 | Local model provider (Ollama) | `WIRED` | D-02 | `router.rs` declares it (`requires_api_key: false`) | **MUST NOT** be used to declare AGENT-01 passed | WIRED |
| V-02 | Air-gapped / offline mode | `SCAFFOLDED` | V-01 | `docs/AIR_GAPPED.md`; `SyntheticOffline` | No egress firewall, no network policy layer | WIRED |
| V-03 | Provider policy (residency/retention) | `NOT_IMPLEMENTED` | D-01, S-04 | No provider policy object | Design-only | SCAFFOLDED |
| V-04 | Container-sandboxed execution | `TESTED` | B-04 | `sandbox.rs` mount-policy tests | Needs a container runtime; worktree-only fallback | RUNTIME_VERIFIED |

---

## Open gates (carried forward, unresolved)

| ID | State | Reason |
|---|---|---|
| **AGENT-01** | `BLOCKED` | No usable approved cloud provider (401/402/placeholder). |
| **PHASE-2A** | `RE-OPENED` | Not accepted; 12 registry entries DISPUTED pending independent re-acceptance. |
| **CI-BILLING** | `BLOCKED_EXTERNAL` | GitHub Actions account lock; jobs execute zero steps. |
| **GATE0-HISTORY** | `OPEN — OWNER DECISION` | Diverged history; forensic report untracked; merge not executed. |
| **LADDER-COLLISION** | `OPEN` | Claim ladder R0–R7 vs capability rung R0–R5 share a prefix. |
| **COMPUTER-USE** | `SIMULATION FACADE` | Sleep-based simulation with simulated-success comments. **Replace, do not extend.** |

---

## What this census changes

Three entries moved off `NOT_IMPLEMENTED` in this wave, and only three:

* **F-03 Dependency-aware task graph** — now `TESTED`.
* **F-04 Factory job lifecycle** — now `TESTED`.
* **P-06 Factory orchestrator** — now `TESTED` (executes task kinds; role specialisation remains DESIGNED).

**No entry was promoted by documentation.** Each of the three is backed by code on disk and by tests
that executed in this session. No other status changed, and in particular **no blocked or disputed
entry was softened.**
