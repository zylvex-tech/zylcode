# ZYLCODE STUB / MOCK / GAP REGISTER — 2026-09-28

**Method:** forensic sweep for `TODO · FIXME · mock · stub · placeholder · fake ·
simulated · demo · sample · hardcoded · not implemented · unimplemented · coming soon`
across `crates/**` and `apps/zylcode-desktop/src/**`, with **execution-path
inspection of every hit**. Matches that survive inspection are defects or gaps;
matches that are honest labels or test fixtures are recorded as VERIFIED-HONEST
so they are not "fixed" into dishonesty later.

Severity: **HIGH** = core-loop or truth-surface impact · **MED** = real gap,
bounded blast radius · **LOW** = hygiene · **INFO** = verified-honest, do not touch.

---

## HIGH

### G-01 — On-disk vector cache can answer a "live" prompt with a persisted SYNTHETIC response

- **Where:** `crates/zylcode-core/src/router.rs` — `TokenRouter::new` loads
  `VectorCacheStore::with_default_path()`; `dispatch_prompt` does
  `find_similar(&emb, 0.88)` **before any HTTP egress**, with `mock_embed`
  (char + bigram hashing, `cache.rs:56`).
- **Observed (2026-09-28, runtime):** the commissioning probe's first run
  returned in **0.01s** a stored payload `{"evidence":["Synthetic offline response"],…}`
  — a cache hit satisfied a live-provider request. The router itself prints
  `vector cache hit — returning cached response without provider call`.
- **Why it matters:** a caller (or a future UI) can believe a provider answered
  when a disk artifact from an offline run answered. This is exactly the
  CLAIM-vs-EVIDENCE failure mode the product thesis forbids.
- **Existing mitigation:** `TokenRouter::without_vector_cache(config)` exists
  and is now used by the commissioning probe.
- **Fix direction:** never cache synthetic/degraded responses in the vector
  store (exact-hash only, or tag entries with provenance and refuse synthetic
  provenance on live dispatch); surface `cache_hit` provenance in metrics.
- **State:** gap open; probe hardened (see G-05 mitigation note).

### G-02 — Tauri project-workspace IPC file is an orphan (never compiled, never registered)

- **Where:** `apps/zylcode-desktop/src-tauri/src/commands/project.rs`
  (8 commands: `create_project`, `open_project`, `list_projects`, `read_file`,
  `write_file`, `list_directory`, `get_git_status`, `close_project`).
- **Evidence:** `main.rs` declares **no** `mod commands` (zero `mod` lines);
  `generate_handler![…]` does not list any `project_*` command; there is no
  `commands/mod.rs`. The file is dead weight — Rust never type-checks it.
- **Impact:** the "secure local project workspace" backend exists and is tested
  in `zylcode-core`, but the desktop IPC path to it **does not exist**. The UI
  reaches files only through the HTTP service instead.
- **Fix direction:** either wire it (`mod commands;` + register handlers +
  compile-fix) or delete it and document HTTP as the single path. Do not leave
  an uncompiled file that reads as implemented.
- **State:** gap open.

### G-03 — Editor save / apply-patch invoke commands with no backend (core-loop link)

- **Where:** `apps/zylcode-desktop/src/lib/useArtifactStream.ts` →
  `safeInvoke(ENVIRONMENT, "save_workspace_artifact", …)` and
  `safeInvoke(ENVIRONMENT, "apply_patch", …)`; registered commands:
  **neither exists** in `generate_handler`.
- **Evidence:** `grep -rn "save_workspace_artifact\|apply_patch" --include=*.rs`
  → only `patch_best_of_n`/`sandbox` internals (worktree-only), no command.
  At runtime `safeInvoke` catches the Tauri "command not found" error and the
  UI shows `Save failed`.
- **Impact:** **REAL FILE IS MODIFIED through the editor cannot happen.**
  This is the broken link in the core engineering loop (§7) for the GUI path.
- **Fix direction:** register two path-bounded commands (workspace root
  containment via `zylcode_core::project::ProjectFilesystem`) — chosen as the
  first implementation slice of this execution.
- **State:** **being fixed in this run (see Execution Report, "files changed").**

---

## MEDIUM

### G-04 — Telemetry/audit hooks invoke non-existent Tauri commands

- **Where:** `hooks/useTelemetryStream.ts` (`get_bridge_status`,
  `reset_circuit_breaker`), `hooks/useAuditStream.ts` (`get_recent_audit_logs`)
  — none registered in `generate_handler`.
- **Mitigating fact:** no component imports either hook today (only their own
  definitions + runtime.test) — **latent**, not live-broken.
- **Fix direction:** wire them to real commands or delete the hooks.

### G-05 — Commissioning test passes vacuously

- **Where:** `crates/zylcode-core/tests/commissioning_test.rs` — returns
  `Ok(())` when router construction fails (no Ollama ⇒ early exit ⇒ green).
- **Impact:** a green battery must never be read as "provider commissioned".
- **Mitigation now:** `tests/live_commissioning.rs` (ignored by default)
  performs real egress and classifies `configured/authenticated/HTTP/completion`
  per provider without printing secrets; run with
  `cargo test -p zylcode-core --test live_commissioning -- --ignored --nocapture`.
- **Fix direction:** mark the old test `#[ignore = "vacuous without Ollama"]`
  or make it assert its own preconditions.

### G-06 — "Streaming" is post-hoc line chunking

- **Where:** `router.rs::dispatch_stream` — awaits the **complete** response,
  then emits synthetic per-line `StreamEvent`s ("Emit synthetic stream events
  chunked by lines for UI progress").
- **Impact:** the UI can show token-by-token delivery that is not streaming.
  Honest in code comments; not honest at the UI boundary.
- **Fix direction:** label the surface "progressive display" or implement real
  SSE (`"stream": true` per provider).

### G-07 — Terminal panel shows a stale BLOCKED banner after the backend recovers

- **Observed (runtime, 2026-09-28):** with `serve-intel` down the panel showed
  `terminal service error: HTTP 500`; after starting the backend and a
  successful `POST /api/terminal/reset → 200`, the BLOCKED banner persisted
  until a full page reload.
- **Impact:** displays a state that is no longer true (§16 truthfulness).
- **Fix direction:** clear the error state on successful reset/poll.

### G-08 — Computer-use + voice surfaces return simulated data by design

- **Where:** `computer_use/gui_automation.rs:151` → `Ok("Simulated clipboard
content")`; `computer_use/mod.rs:183` → "In our simulated environment, this
  should succeed"; `ai_input/voice_processor.rs:117` → "Return simulated text
  based on audio characteristics".
- **Impact:** acceptable **only** while UI labels them simulated. Any surface
  presenting these as real clipboard/transcription is a false surface.
- **State:** code-honest; UI labeling **UNVERIFIED** (no live check of those
  panels this session).

### G-09 — Model-authored plan / patch paths are CLI-only and provider-blocked

- **Where:** `patch_best_of_n::ModelPatchSource` is used by
  `zylcode best-of-n` CLI only; the HTTP mission drain uses
  `WorkingTreeSource` (verifies the current tree, does not edit).
- **Impact:** the GUI mission flow performs no model-driven modification;
  "MODEL CREATES PLAN" and "agent edits files" are unreachable in the product
  shell until a provider completes (BLOCKED) or a local model is added.
- **State:** documented; blocked on provider commissioning (see baseline).

### G-10 — No prompt-injection boundary for repository content

- **Finding:** no module implements untrusted-content isolation for model
  context (repo files could carry instructions).
- **Contrast:** shell **argument** injection is defended (`real_tools.rs`).
- **State:** gap open; Wave C/security work.

### G-11 — Symlink traversal not explicitly checked

- **Where:** `project/filesystem.rs` uses canonicalize + prefix containment
  (strong), but no explicit symlink policy test found.
- **State:** UNVERIFIED rather than broken.

### G-12 — Repository-intelligence benchmark is non-deterministic

- **Fact:** precision threshold held at 0.20 with rationale
  (RECONCILIATION_FORENSIC_REPORT §8); depends on repository content.
- **Rule:** never cite it as a deterministic test; do not lower the threshold.

---

## LOW / HYGIENE

### G-13 — Untracked handoff files on the primary tree (ownership unresolved)

`tasks/plan.md`, `tasks/todo.md`, `docs/governance/RECONCILIATION_FORENSIC_REPORT.md`,
`COMPLETE_AUDIT_REPORT.html`, `.git-msg.txt`,
`crates/zylcode-core/src/project/filesystem.rs.backup`.
Left untouched (no-commit/no-delete mandate). `.backup` is a stale duplicate of
a tracked file → delete candidate once authorised.

### G-14 — Root documentation debt

20+ historical `PHASE*_COMPLETE/REPORT`, `FINAL_*`, `PROJECT_COMPLETE` files at
repo root make superseded claims (test counts, statuses) that predate today's
575/0. Not defects, but they contradict `ENGINEERING_TRUTH.md` for any reader.
→ consolidation pass needed.

### G-15 — HTTP API is read-only for files

No write route in `serve-intel` (GET-only file-content). Fine for the browser
preview threat model; just never advertise browser-side editing.

### G-16 — Stale `.wt/` worktrees from the Gate-0 divergence

`.wt/audit-34d29a9`, `.wt/remediation`, `.wt/verify` (detached HEADs) still
present; the recommended merge of `865c142` into `main` was **never executed**
(owner decision pending — recorded as blocker, not touched).

---

## VERIFIED-HONEST (matches that must NOT be "fixed")

| Item                                                                                                                    | Why it is honest                                                                           |
| ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `CapabilityStatus` COMING SOON / NOT INSTALLED badges + `StatusLockedAction` "Not implemented yet."                     | deliberate truth UI, tested (`CapabilityStatus.test.tsx`)                                  |
| Forge.tsx `"Concept — not implemented. Appears here as marketplace taxonomy metadata only."` + "Try Demo — COMING SOON" | explicit non-implementation label, no fake checkout                                        |
| MissionComposer `$ Skills — COMING SOON`                                                                                | lib exists, UI honestly unwired                                                            |
| Capability spotlight `Design Studio COMING SOON`, `Multi-agent board COMING SOON`                                       | matches NOT_IMPLEMENTED status                                                             |
| DeliveryPanel `COMING SOON` when pipeline idle                                                                          | means "not run yet", not "works"                                                           |
| `ModelProvider::SyntheticOffline` "explicit synthetic provider, honoured unconditionally"                               | explicit selection, cannot impersonate a real provider by construction                     |
| `proof_engine.rs::ProofSource::Simulated` + tests asserting simulated evidence can never be accepted                    | evidence honesty machinery                                                                 |
| `tool.rs` comments containing `unsupported_mock`                                                                        | explain the REMOVED behaviour; `DynamicTool` now fails closed                              |
| `intelligence/symbols.rs` `todo!()` (3 hits)                                                                            | inside raw-string **test fixtures** (sample Rust code being parsed) — not executable paths |
| `voice_processor` "simulated transcript" comment                                                                        | documents the simulation (subject to G-08 labeling)                                        |
| `real_tools.rs` `success: true` (3 hits)                                                                                | real results of real reads/commands, not fabricated success                                |
| Explorer "BLOCKED — start `zylcode serve-intel`"                                                                        | truthful dependency statement (went live when backend started)                             |
