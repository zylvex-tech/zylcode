# ZYLCODE MASTER EXECUTION REPORT — 2026-09-28

**Scope:** Forensic Resume + Master Execution Baseline, Wave A (Truth and Stability),
executed on `C:\Projects\zylcode` (`main`). No git history was modified; nothing
was committed (mandate §1). Companion documents:

- `docs/audit/ZYLCODE_MASTER_BASELINE_2026-09-28.md` — full capability map
- `docs/audit/ZYLCODE_STUB_MOCK_GAP_REGISTER_2026-09-28.md` — gap register

---

## 1. Starting Git state

```
root:     C:\Projects\zylcode   (confirmed: pwd = /c/Projects/zylcode, .git present)
branch:   main
HEAD:     58d7ce7 feat(evidence): formal claim model + evidence graph + engineering truth
tracked:  M crates/zylcode-cli/Cargo.toml        (mission subcommand dev-dep: tempfile)
          M crates/zylcode-cli/src/main.rs       (+101: `zylcode mission` command + handler)
          M crates/zylcode-core/src/lib.rs       (+2: module exports)
untracked: crates/zylcode-cli/tests/, crates/zylcode-core/{failure.rs, first_mission.rs,
          tests/first_mission_e2e.rs, tests/live_commissioning.rs, tests/fixtures/},
          docs/audit/ (this report), plus pre-existing handoff files
          (tasks/, RECONCILIATION_FORENSIC_REPORT.md, COMPLETE_AUDIT_REPORT.html,
           .git-msg.txt, project/filesystem.rs.backup) — untouched
stash:    (empty)
diff:     106 insertions, 0 deletions across 3 tracked files
```

The First-Mission work (`failure.rs`, `first_mission.rs`, fixtures, e2e tests) was
already on disk at session start, uncommitted, per the earlier T3 session. This
session **did not commit** — mandate §1 forbids it until authorised.

---

## 2. Current architecture (as found, not as designed on paper)

```
apps/zylcode-desktop        Tauri 2 + React 18 + Vite (port 1420 strict)
  src-tauri/src/main.rs     31 registered IPC commands (invoke_handler)
  src/components/shell/     ActivityRail, AgentDock, BottomPanel, ContextSidebar,
                            MenuBar, RightWorkspace, SurfaceHost
  src/components/*          40+ panels (Evidence, Proof, Telemetry, Terminal,
                            SourceControl, LiveSearch, RepoIntel, Forge, …)
crates/zylcode-core         engine: agent.rs (kernel), router.rs (providers),
                            intelligence/* (15 modules), missions.rs, ledger.rs
                            (sqlite/memory), proof_engine.rs, claim.rs,
                            evidence_graph.rs, first_mission.rs, delivery.rs,
                            project/filesystem.rs, patch_best_of_n.rs, …
crates/zylcode-mcp          tool runtime: real_tools.rs (12 executors),
                            permission.rs, evidence.rs, actor.rs, tool_catalogue.rs,
                            bridges/transports, telemetry, skills, marketplace
crates/zylcode-cli          binary `zylcode`: subcommands + axum HTTP API
                            (serve-intel, /api/* → frontend backend), port 17630
scripts/                    check_retracted_claims.py (+baseline)
.github/workflows           ci.yml (3-OS matrix), release.yml (tag-only)
```

Two backends, one frontend: Tauri IPC (31 commands) and the `zylcode serve-intel`
HTTP service (`/api/*`, proxied by Vite). Most panels talk HTTP; a few talk IPC.

---

## 3. Capability matrix

Full 50-row map with evidence codes: `docs/audit/ZYLCODE_MASTER_BASELINE_2026-09-28.md`.
Summary of highest-interest rows:

| Area                                                                       | Verdict                                                                                      |
| -------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Shell / themes / dock / panel                                              | IMPLEMENTED · UI_WIRED · RUNTIME_VERIFIED (all 8 themes live)                                |
| Repository indexing + intelligence                                         | IMPLEMENTED · TESTED (46 tests) · RUNTIME_VERIFIED (444→447 files, 2111 symbols, 6 packages) |
| Tool runtime (12 executors, binding, default-deny gate)                    | IMPLEMENTED · TESTED (Gate-0 invariants all confirmed)                                       |
| Evidence stack (ledger, proofs, tool JSONL, claims, graph, record/package) | IMPLEMENTED · TESTED · EVIDENCE_RECORDED                                                     |
| Session persistence + crash recovery                                       | IMPLEMENTED · TESTED · RUNTIME_VERIFIED (real abort + resume)                                |
| First Mission §36 end-to-end                                               | IMPLEMENTED · TESTED · RUNTIME_VERIFIED                                                      |
| Live LLM completion                                                        | **BLOCKED** (egress proven, no completion)                                                   |
| True streaming / provider-native tool calling                              | NOT_IMPLEMENTED                                                                              |
| Editor save/apply-patch backend                                            | **NOT_IMPLEMENTED (G-03)** — first implementation slice                                      |
| Tauri project IPC                                                          | orphan file, NOT reachable (G-02)                                                            |
| Multi-agent, UI builder, updater, crash reporting, entitlements            | NOT_IMPLEMENTED (all honestly labelled in UI)                                                |
| CI                                                                         | workflow files implemented, execution BLOCKED (billing)                                      |

---

## 4. Test totals (exact, this session)

| Command                                                                       | Discovered         | Passed                   | Failed | Ignored | Duration                                                        |
| ----------------------------------------------------------------------------- | ------------------ | ------------------------ | ------ | ------- | --------------------------------------------------------------- |
| `cargo test --workspace --all-targets --no-fail-fast`                         | 575                | **575**                  | **0**  | 0       | ≈84s in-test (17 binaries; longest: core lib 382 tests / 33.2s) |
| `cargo clippy --workspace --all-targets -- -D warnings`                       | —                  | 0 warnings               | 0      | —       | pass                                                            |
| `python scripts/check_retracted_claims.py`                                    | 11 baselined lines | 0 new claims             | —      | —       | pass                                                            |
| `pnpm test` (desktop, vitest)                                                 | 54                 | **54**                   | **0**  | 0       | 50.3s                                                           |
| `pnpm typecheck` (`tsc --noEmit`)                                             | —                  | 0 errors                 | 0      | —       | pass                                                            |
| `pnpm build` (`tsc && vite build`)                                            | —                  | OK                       | —      | —       | 8.80s                                                           |
| `cargo test -p zylcode-core --test live_commissioning -- --ignored` (network) | 1                  | 1 (classification probe) | 0      | 0       | 2.0s + real provider round-trips                                |
| Remote CI (`.github/workflows/ci.yml`)                                        | —                  | **BLOCKED** (billing)    | —      | —       | not run                                                         |
| Installed-app run (`tauri build` + launch)                                    | —                  | **UNVERIFIED**           | —      | —       | not run                                                         |

Focused verification ran first (claim/evidence/first-mission suites during
development), then the broadest safe worktree verification above.

---

## 5. Stub / mock findings

Full register with execution-path inspection: `docs/audit/ZYLCODE_STUB_MOCK_GAP_REGISTER_2026-09-28.md`.
Top findings:

1. **G-01 (HIGH)** — the router's on-disk vector cache answered a _live provider
   probe in 0.01s with a persisted "Synthetic offline response"_ (char/bigram
   `mock_embed`, 0.88 threshold, checked before egress). Observed at runtime
   today; `without_vector_cache` is the escape hatch and is now used by the probe.
2. **G-02 (HIGH)** — `src-tauri/commands/project.rs` (8 commands) is never
   `mod`-declared: not compiled, not registered. Dead file reading as implemented.
3. **G-03 (HIGH)** — `save_workspace_artifact` / `apply_patch` UI invocations
   have **no backend command** → editor writes fail at runtime. Chosen as the
   first implementation slice (§13).
4. **G-05 (MED)** — `commissioning_test` passes vacuously without a provider;
   replaced in evidentiary value by the new `live_commissioning` probe.
5. **G-06 (MED)** — "streaming" is post-hoc line chunking of a completed response.
6. 12 further entries (stale terminal banner, simulated computer-use/voice
   surfaces, prompt-injection boundary missing, symlink policy UNVERIFIED,
   benchmark non-determinism, untracked handoff files, root docs debt).
7. **Verified-honest set**: every `COMING SOON` / `Concept — not implemented` /
   `unsupported_mock`-comment hit was inspected; none is lying to the user —
   they are recorded so they are not "fixed" into dishonesty.

---

## 6. Core engineering loop — exactly how far the product gets (§7)

17 stages, each marked **PROVEN** (runtime or test evidence this session),
**PARTIAL**, **BLOCKED**, or **NOT WIRED**.

| #   | Stage                         | Verdict                                                      | Evidence                                                                                                                                                                                                                                            |
| --- | ----------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Open real repository          | **PROVEN (GUI)**                                             | serve-intel opened `C:\Projects\zylcode`; Explorer rendered 447 files                                                                                                                                                                               |
| 2   | Index / understand repository | **PROVEN (GUI)**                                             | live index: 444→447 files, 2111 symbols, 6 packages; 46 module tests                                                                                                                                                                                |
| 3   | User creates mission          | **PROVEN (GUI surface)**                                     | MissionComposer + `POST /api/missions` (route live, 200s); UI tests 6                                                                                                                                                                               |
| 4   | Agent receives mission        | **PARTIAL**                                                  | queue drain exists (`/api/missions/run-next`), but the drain runs **Best-of-N verification of the working tree**, not an agent that reads the mission as instructions. `AgentLoop` exists as a library, tested — **not wired to the mission queue** |
| 5   | Model creates plan            | **BLOCKED** (GUI: not wired; library: needs live completion) | Plan-mode uses deterministic `build_plan`; model planning needs a provider completion (BLOCKED §7 provider)                                                                                                                                         |
| 6   | Required approval occurs      | **PARTIAL**                                                  | permission gate default-deny + property tests PROVEN at library level; MissionComposer autonomy selector **not observed gating the drain** (UNVERIFIED)                                                                                             |
| 7   | Tool is called                | **PROVEN (library/runtime)**                                 | 12 executors + dispatch through gate; integration tests; terminal 200                                                                                                                                                                               |
| 8   | Real file is read             | **PROVEN (GUI)**                                             | `/api/files` live tree; EditorPane serves `/api/file-content`                                                                                                                                                                                       |
| 9   | Real file is modified         | **PROVEN (CLI/library)** — **NOT WIRED (GUI editor)**        | First Mission rewrote `statslib/core.py` with evidence (e2e); **G-03**: editor save has no backend                                                                                                                                                  |
| 10  | Diff is produced              | **PROVEN (partial)**                                         | SourceControl diffstat from real git (+/−); unified diff viewer depth UNVERIFIED                                                                                                                                                                    |
| 11  | Test/build executed           | **PROVEN**                                                   | drain runs `cargo test` for real (1800s budget, evidence in ledger); First Mission ran `python -m unittest`                                                                                                                                         |
| 12  | Result inspected              | **PROVEN**                                                   | exit codes + signature sets recorded per run (First Mission payloads)                                                                                                                                                                               |
| 13  | Failure reasoned about        | **PROVEN (library)**                                         | diagnosis parsed from captured output (`AssertionError…`), structured Failure object                                                                                                                                                                |
| 14  | Correction applied            | **PROVEN (library)**                                         | specification-directed replace → artifact hash                                                                                                                                                                                                      |
| 15  | Verification runs again       | **PROVEN**                                                   | same command re-run, exit 0, failure promoted to RECOVERED fail-closed                                                                                                                                                                              |
| 16  | Evidence recorded             | **PROVEN**                                                   | ledger prev-hash chain + evidence graph + claims chain, all verified; `verify_ledger_chain` over exported `ledger.jsonl`                                                                                                                            |
| 17  | Session/mission reopened      | **PROVEN**                                                   | real `abort()` mid-step + resume reconciled and completed (CLI e2e, this session)                                                                                                                                                                   |

**Verdict:** the complete chain is PROVEN end-to-end **through the library/CLI
path** (First Mission, 1/17 steps dependent on nothing external). Through the
**GUI product path**, the chain is PROVEN for stages 1–3, 7–8, 10–12, 16 and
breaks at **4–6 (model participation not wired to missions + providers BLOCKED)**
and **9 (editor write backend missing)**. Those three breaks are now precisely
identified, not guessed.

---

## 7. Repository-intelligence state

- 15 modules (`scanner, symbols, dependency, manifest, entry_points, git, query,
context, api, canvas, persisted, store, classifier, architecture, types`) —
  **46 unit tests** counted across modules, all green in the battery.
- Live runtime: index warm at server start; Explorer/Search/RepoIntel panels
  consume it; benchmark `repo_intelligence_benchmark.rs` (2 tests) with
  documented 0.20 precision threshold — **non-deterministic by content, never
  to be claimed deterministic** (G-12).
- Deterministic invalidation: content hashes in `persisted.rs` (mtimes cannot
  fake freshness).
- Not established: progressive/non-blocking indexing (**UNVERIFIED**),
  cross-commit "what changed" answering (module exists, no test observed),
  provenance of what was supplied to the model (**UNVERIFIED**).

## 8. Provider state (§11 — never collapsed into "provider works")

| Provider                            | configured                                    | authenticated        | request sent | HTTP response                                           | completion | classification                      |
| ----------------------------------- | --------------------------------------------- | -------------------- | ------------ | ------------------------------------------------------- | ---------- | ----------------------------------- |
| DeepSeek `deepseek-chat`            | YES (key in env, value not shown)             | YES (key recognized) | YES          | **402 Insufficient Balance** (1.08s, real `request_id`) | NO         | **BLOCKED — account balance**       |
| OpenRouter free tier                | YES (23-char value, **not `sk-or-…` format**) | NO                   | YES          | **401 Missing Authentication header** (0.27s)           | NO         | **BLOCKED — invalid credential**    |
| Anthropic `claude-3-5-haiku-latest` | YES (51-char `sk-…`)                          | NO                   | YES          | **401 API key is invalid** (0.69s)                      | NO         | **BLOCKED — invalid credential**    |
| Ollama local                        | —                                             | —                    | —            | not running (`/api/tags` unreachable)                   | NO         | **BLOCKED — not installed/running** |

- Egress pipeline itself is **RUNTIME_VERIFIED**: three distinguishable real
  provider answers prove env-key → auth header → HTTPS → provider works.
- Streaming: NOT_IMPLEMENTED (post-hoc chunking, G-06). Provider-native tool
  calling: NOT_IMPLEMENTED (no `tools` in bodies). Multimodal: not probed.
- First probe attempt returned a **cached synthetic** (G-01) — corrected with
  `without_vector_cache` before any conclusion was drawn.
- No secrets printed, logged, or persisted anywhere in this session (presence
  and length only, lengths of prefixes excluded from the record).

## 9. Agent-kernel state

`agent.rs`: `run()` loop with `max_iterations` termination (default 10, hard
stop), plan-level and tool-level approval gates (`require_approval`,
risk-based `requires_approval`), actor attribution, ledger integration.
Tests: `agent_loop_e2e.rs` (4: end-to-end, tool execution, observation loop,
repair loop), approval enforcement unit test, property tests on the permission
decision core. **TESTED, IMPLEMENTED** — with live-model execution BLOCKED and
the kernel **not wired into the mission queue** (G-09 context).

## 10. Memory state

| Kind                                                 | State                                                                  |
| ---------------------------------------------------- | ---------------------------------------------------------------------- |
| Execution ledger (hash-chained, sqlite)              | IMPLEMENTED · TESTED · EVIDENCE_RECORDED                               |
| Mission history                                      | IMPLEMENTED (persisted queue, survives restart)                        |
| Project facts / derived intel                        | IMPLEMENTED · TESTED (content-hash invalidation)                       |
| Settings/preferences                                 | IMPLEMENTED · TESTED · UI_WIRED                                        |
| Chat history                                         | IMPLEMENTED within sessions (frontend state); cross-session UNVERIFIED |
| Architectural decisions                              | partial via Claim/Proof stores (new, TESTED)                           |
| Explicit provenance + staleness for ALL memory kinds | UNVERIFIED (intel only)                                                |

## 11. MCP state

12 executable tools / 21 proposed (metadata-only, pinned by test), bridge
registers **only** tools with real executors, unknown tool ids fail closed,
`git.commit` cannot push, default gate refuses unapproved writes — all TESTED
(11 integration + 3 fault-injection + 23 telemetry tests). Evidence JSONL sink
records every gated call. UI: McpInspector + register/list commands wired.

## 12. UX state

Four-zone shell, Agent Dock, Bottom Panel, command palette, settings, 8 themes —
**all rendered live today** via the registered preview (http://localhost:1420).
Honesty labels verified in the running UI (COMING SOON badges, BLOCKED explorer
message that turned AVAILABLE when the backend started). Defects: stale terminal
banner (G-07), read-only editor with broken save (G-03). No cosmetic rewrite
performed (mandate §16).

## 13. Security findings

- **Implemented+tested:** workspace path containment (canonicalize + prefix +
  `..`), shell argument-injection defense, tool-operation binding, default-deny
  approval gate with property tests, actor-attributed evidence, env-only
  secrets (never printed — verified in this session's handling).
- **UNVERIFIED:** symlink traversal policy, prompt-injection boundary for repo
  content (G-10), MCP external-content trust beyond fail-closed tool dispatch.

## 14. Runtime evidence captured this session

1. **Preview live:** Vite dev server pid 29352 on `http://localhost:1420`
   (port pinned by config), `zylcode serve-intel` pid 4132 on 17630,
   `/healthz {"status":"ok"}`, Explorer showed the real 447-file tree, network
   log `/api/evidence|missions|terminal/reset → 200`. Run doc
   `.freebuff/run.md` written.
2. **First Mission e2e** (part of 575/0): full run + blocked-on-mismatch +
   **kill (real `abort`) → resume → reconcile → complete**.
3. **Live provider probe:** 3 real provider HTTP responses (§8), captured with
   request ids, no secrets.
4. **Frontend:** 54/54 vitest, tsc 0, build 8.80s.
5. Battery: 575/0, clippy 0, claims guard OK.

## 15. Files changed in this session

Modified (tracked):

- `crates/zylcode-cli/Cargo.toml` (+3: dev-dep tempfile)
- `crates/zylcode-cli/src/main.rs` (+101: `mission` subcommand, `MissionArgs`,
  `handle_mission` — completes the earlier T3 wiring)
- `crates/zylcode-core/src/lib.rs` (+2: module exports)

New (untracked):

- `crates/zylcode-core/src/failure.rs`, `first_mission.rs`,
  `tests/first_mission_e2e.rs`, `tests/fixtures/*` (carried from T3, verified)
- `crates/zylcode-cli/tests/first_mission_kill_resume.rs`
- `crates/zylcode-core/tests/live_commissioning.rs` (new this session)
- `docs/audit/ZYLCODE_MASTER_BASELINE_2026-09-28.md`
- `docs/audit/ZYLCODE_STUB_MOCK_GAP_REGISTER_2026-09-28.md`
- `docs/audit/ZYLCODE_MASTER_EXECUTION_REPORT_2026-09-28.md` (this file)
- `.freebuff/run.md` (preview run doc)

Small source fixes: `first_mission.rs` clippy (`rfind`, `mismatch_reason.clone()`,
2 `build_plan(&…)` call sites), test path fixes. Pre-existing handoff files
untouched; **nothing committed** (§1).

## 16. Blockers

| Blocker                                          | Class                              | Effect                                                             |
| ------------------------------------------------ | ---------------------------------- | ------------------------------------------------------------------ |
| GitHub Actions billing lock                      | BLOCKED_EXTERNAL                   | remote CI/release evidence unavailable; local battery is authority |
| DeepSeek account balance (402)                   | BLOCKED_EXTERNAL                   | no live completion                                                 |
| OpenRouter + Anthropic credentials invalid (401) | BLOCKED_EXTERNAL (fixable by user) | no live completion                                                 |
| Ollama not running                               | BLOCKED_EXTERNAL                   | local-model path unavailable                                       |
| Gate-0 `865c142` merge never executed            | OWNER_DECISION                     | diverged history persists                                          |
| No commit authorisation for this mandate         | PROCESS                            | all work left uncommitted                                          |
| Preview screenshot compositing unavailable       | TOOLING                            | visual verification done via a11y snapshots/DOM/network instead    |

## 17. Regressions

**None found.** 575/0, 54/54, tsc 0, build OK, clippy 0, guard OK — the
worktree is green before and after this session's changes. The stale terminal
banner (G-07) is a pre-existing UX defect, not a regression.

## 18. Next executable work (dependency order)

1. **G-03 (in progress):** register `save_workspace_artifact` + `apply_patch`
   Tauri commands with workspace-root containment, wire them, test — closes the
   GUI core-loop break at stage 9.
2. **G-01:** stop synthetic responses from entering/being served by the
   vector cache on live dispatch (truth-surface hazard).
3. **G-02:** compile-and-register or remove `commands/project.rs`.
4. **G-04/G-07:** dead hooks + stale banner cleanup.
5. Then Wave B (repository intelligence: progressive indexing, context
   provenance) and the Wave C model-wiring question — both partly blocked on
   provider commissioning; independent of that, security boundary work (G-10)
   and Gate-0 merge decision (owner).
