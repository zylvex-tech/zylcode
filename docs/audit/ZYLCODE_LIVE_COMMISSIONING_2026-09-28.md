# ZYLCODE LIVE COMMISSIONING — 2026-09-28

**Scope:** Live Runtime Commissioning and Core-Loop Proof (mandate of 28 Sep 2026),
executed against the running dev environment on `C:\Projects\zylcode` (`main`).
Nothing committed; no git history touched. Companion documents:
`docs/audit/ZYLCODE_MASTER_BASELINE_2026-09-28.md`,
`docs/audit/ZYLCODE_STUB_MOCK_GAP_REGISTER_2026-09-28.md`,
`docs/audit/ZYLCODE_MASTER_EXECUTION_REPORT_2026-09-28.md`.

---

## 1. Live baseline (verified this session, not assumed)

| Item                               | Value                                                                                                                                                                       | Verification                                     |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| Frontend URL                       | `http://localhost:1420/`                                                                                                                                                    | HTTP 200 curl + DOM snapshots                    |
| Frontend process                   | `node` **PID 29352** (vite, started 09-28 02:37)                                                                                                                            | `Get-Process -Id` + `netstat :1420 LISTEN`       |
| Backend URL                        | `http://127.0.0.1:17630`                                                                                                                                                    | `netstat :17630 LISTEN` + `/healthz` 200         |
| Backend process                    | `zylcode` **PID 4132 → 30972 → 29852** (started 02:41, then restarted twice _by the commissioning tests below_)                                                             | process identity checked by name+PID, not memory |
| Health                             | `{"service":"zylcode-repo-intel","status":"ok"}`                                                                                                                            | curl                                             |
| Indexed files                      | 447 → 450 → **454** (grows with the working tree; index is content-hash validated)                                                                                          | `/api/repo-intel`, `/api/files`                  |
| Symbols / packages                 | 2111 → 2120 symbols · 6 packages                                                                                                                                            | `/api/repo-intel`                                |
| Endpoints exercised (200)          | `/api/evidence`, `/api/missions`, `/api/terminal/reset`, `/api/terminal/exec`, `/api/search`, `/api/files`, `/api/file-content`, `/api/repo-intel`, `/healthz`              | curl + preview network log                       |
| New endpoints shipped this session | `POST /api/artifact/save`, `POST /api/artifact/patch` (transport parity for the editor write path)                                                                          | runtime probe incl. fail-closed `../` refusal    |
| Browser verification method        | `preview_snapshot` accessibility trees, `preview_evaluate` DOM probes, `preview_logs` network capture                                                                       | —                                                |
| Screenshot limitation              | `preview_screenshot` produced **no frames** (compositor unavailable in this environment) — **no visual screenshot verification is claimed**; all UI evidence is DOM/network | —                                                |
| Console errors                     | none observed after backend recovery (proxy 500s only while backend was deliberately down)                                                                                  | `preview_logs`                                   |
| BLOCKED surfaces (honest)          | Design Studio, Multi-agent board (COMING SOON), Runtime panel (LIMITED)                                                                                                     | DOM                                              |

---

## 2. Stale backend-recovery defect — reproduced, root-caused, fixed, proven live

**Observed defect (reproduced):** with the backend stopped, a _reload_ showed
Terminal `BLOCKED` / "terminal service error: HTTP 500" (honest), and a page
that had mounted _before_ the backend died kept showing `AVAILABLE` — a stale
false surface. Earlier evidence (previous session): after the backend came up,
surfaces stayed in their stale state until reload.

**Concrete cause (source-inspected):** one-shot mount-time probes with no
health re-check:

- `TerminalPanel` probed once in `useEffect([])`; any failure latched
  `BLOCKED` forever, any early success latched `AVAILABLE` forever; transport
  failure was detected by **string-prefix sniffing** (`startsWith("terminal ")`).
- `ExplorerTree` fetched `/api/files` once; `unavailable` latched.
- `BottomPanel.EvidenceView` fetched `/api/evidence` once; latched.
  (`FilesPanel` and `useTestRuns` already polled — they self-healed, proving
  the fix pattern was already established in-repo.)

**Fix (three surfaces, one contract):**

- `lib/terminal.ts`: explicit `transportError` flag distinguishes _transport_
  failure (backend gone) from a _command_ failing (health unchanged) — no
  string sniffing.
- `TerminalPanel.tsx`: mount probe → BLOCKED truthfully; re-probe every
  `RECOVERY_INTERVAL_MS` (2s) while down; recover to AVAILABLE on a real
  healthy probe (announced in-terminal); a mid-session transport failure
  flips the badge back to BLOCKED and re-arms the loop; a user command that
  succeeds while BLOCKED flips health immediately (evidence first); failed
  `terminalReset` now reports failure instead of printing "session reset".
- `ExplorerTree.tsx` (`TREE_RETRY_MS` 15s) and `BottomPanel.EvidenceView`
  (6s): poll **only while unavailable**; healthy states never re-fetch.

**Regression tests (10 new, all green):** `TerminalPanel.test.tsx` (6),
`ExplorerTree.test.tsx` (3), `terminal.test.ts` (4; transport classification).
Frontend suite: **67 passed / 0 failed**, `tsc --noEmit` clean.

**Live proof (no reload involved in the recovery leg):**

1. Backend killed → reload → Terminal `BLOCKED` "terminal service error: HTTP
   500" (honest state, no fabrication).
2. New backend binary started on the same port (without touching the page).
3. Next snapshot, **no reload**: Terminal `AVAILABLE` — "real shell · cwd
   persists per session". Recovery chain works end to end.

**During the same backend restart window the new save backend was deployed and
runtime-proven fail-closed:** `POST /api/artifact/save` with
`{"label":"../escape.txt"}` → `{"error":"file path must not contain '..'"}`
(refused before any write; target file untouched).

---

## 3. Repository intelligence vs deterministic source truth

Method: for symbols from four different modules, ground truth was established
with `grep -rn` (deterministic), then the same query went to the live
`/api/search`. `elapsed_ms` per query: 1.9–2.6 s (451 files).

| Query               | Ground truth                              | Intelligence answer                                                                 | Verdict   |
| ------------------- | ----------------------------------------- | ----------------------------------------------------------------------------------- | --------- |
| `TokenRouter`       | `crates/zylcode-core/src/router.rs:500`   | top-1 `crates::zylcode-core::src::router::TokenRouter` (rel 3.56), owner file top-2 | **match** |
| `save_file_payload` | `crates/zylcode-core/src/surfaces.rs:253` | top-1 `…::surfaces::save_file_payload` (rel 3.72), owner file top-2                 | **match** |
| `TerminalHub`       | `crates/zylcode-core/src/terminal.rs:117` | top-1 exact symbol (rel 3.88)                                                       | **match** |
| `ClaimStore`        | `crates/zylcode-core/src/claim.rs:357`    | top-1 exact symbol (rel 3.41)                                                       | **match** |

False positives / false negatives in definition queries: **none** in the four
probed. Case variant (`tokenrouter`) still resolves the symbol (substring
match, lower rank) — acceptable, honestly reasoned.

**Unsupported queries (honestly NOT_IMPLEMENTED, no fabricated graph):**
"references of X", "who calls X", "imports in X" return keyword-ranked
results whose reasons do not claim reference/ownership relations. Deterministic
index provides: definitions, file/language/package/entry-point discovery,
architecture facts, recent git changes. References/imports/dependency-graph
queries are the next intelligence wave (Wave B), not a current capability.

## 4. Mission persistence — proven across a real restart

1. Enqueue over live HTTP: mission `5b668eb4…` "commissioning probe: verify
   mission persistence across restart" (mode `plan`, state `queued`) → 201
   payload echoed; persisted to `.zylcode/missions.json` (on-disk verified).
2. Backend process **killed** (old PID gone; listener gone — verified).
3. New backend process started; `/healthz` ok (new PID).
4. `GET /api/missions` after restart: `5b668eb4 … queued` present, plus three
   historical missions (`done`, `failed`, `done`) — **VERDICT: PERSISTED**.
   Durable states observed in the wild: `queued`, `done`, `failed`.
   (Launcher timeouts for detached starts are recorded separately per §13;
   process/listener/health verification is the proof of liveness.)

## 5. Live provider commissioning (no secrets; evidence = fresh HTTP round-trips)

Explicit `#[ignore]`d probe (`crates/zylcode-core/tests/live_commissioning.rs`,
run with `--ignored`), re-executed today:

| Provider   | configured                       | request sent | HTTP response received                    | completion | classification                                                                  |
| ---------- | -------------------------------- | ------------ | ----------------------------------------- | ---------- | ------------------------------------------------------------------------------- |
| DeepSeek   | YES (env key, value never shown) | YES — 1.13 s | YES — **402 Insufficient Balance**        | NO         | **BLOCKED** (external: billing)                                                 |
| OpenRouter | YES                              | YES — 0.46 s | YES — 401 "Missing Authentication header" | NO         | **BLOCKED** (credential invalid/not OpenRouter format; 23-char key, no hyphens) |
| Anthropic  | YES                              | YES — 0.69 s | YES — 401 "API key is invalid"            | NO         | **BLOCKED** (credential)                                                        |

Transport truth established earlier by differential curl: the header _does_
transit (DeepSeek's 402 proves auth parsing); OpenRouter's stored key is
almost certainly not an OpenRouter credential. Streaming, tool-calling,
multimodal: **BLOCKED** (they require a completable channel). No key material
printed or persisted anywhere. Per mandate §5, this does not stop the pass.

## 6. Core-loop proof status (chain → where it breaks)

| Chain step                              | State                                                | Evidence                                                                                                                |
| --------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Open real repository                    | RUNTIME_VERIFIED                                     | live UI + `/api/files` 454 files                                                                                        |
| Index / understand repository           | RUNTIME_VERIFIED                                     | `/api/repo-intel` 454/2120/6; ground-truth-matched queries (§3)                                                         |
| User creates mission                    | RUNTIME_VERIFIED                                     | HTTP enqueue persisted (§4)                                                                                             |
| Model/agent planning                    | **BLOCKED (external: provider credentials/billing)** | §5                                                                                                                      |
| Tool calls (real)                       | RUNTIME_VERIFIED (deterministic path)                | `/api/terminal/exec` live; Phase T3 First Mission e2e drives real python/git                                            |
| Real file change + diff                 | TESTED (backend) + runtime save probe                | `save_file_payload`/`apply_patch_payload` (9 unit tests; live fail-closed probe); Tauri + HTTP transports wired         |
| Test/build execution                    | TESTED + RUNTIME_VERIFIED                            | `/api/build` status machinery; First Mission e2e runs real `python -m unittest`                                         |
| Failure inspection → repair → re-verify | TESTED                                               | First Mission e2e: real failing test → diagnosis → repair → green; kill/`abort()` mid-step → resume reconciles honestly |
| Evidence recorded                       | RUNTIME_VERIFIED                                     | UI chain **intact**; ledger hash-chain survives restarts                                                                |
| Durable mission/session                 | RUNTIME_VERIFIED                                     | §4 restart proof                                                                                                        |

The only missing link in the live model-driven loop is **model/agent planning**,
blocked externally by credentials/billing — not by repository code.

## 7. Regressions and gates this session (§13)

- Frontend: `pnpm test` → **11 files / 67 tests passed / 0 failed** (+10
  regression tests); `tsc --noEmit` clean.
- Rust: `cargo check -p zylcode-cli --all-targets` clean; `cargo check -p
zylcode-desktop` clean; `cargo test -p zylcode-core --lib surfaces` → **9
  editor-write tests green** (containment, symlink escape, git-apply
  fail-closed, byte-faithful line endings).
- Detached-service launcher timeouts (PowerShell `Start-Process`) occurred
  3× and are recorded as launcher quirks; liveness was proven by listener +
  health checks, per §13.

## 8. Final classification (§14, independent axes)

| Capability                                     | State                                                                                                                                                                                                  |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Repository indexing                            | IMPLEMENTED · TESTED · RUNTIME_VERIFIED (454 files / 2120 symbols, content-hash validated)                                                                                                             |
| Repository intelligence                        | IMPLEMENTED · TESTED · RUNTIME_VERIFIED for definitions/files/packages/entry points/recent changes; **references/imports/dependency graph NOT_IMPLEMENTED** (queries honestly refused as keyword-only) |
| Mission persistence                            | IMPLEMENTED · TESTED · RUNTIME_VERIFIED (restart-proof §4)                                                                                                                                             |
| Provider authentication                        | BLOCKED (external: 402 billing / 401 credentials — requests genuinely reach providers)                                                                                                                 |
| Provider streaming / tool calling / multimodal | BLOCKED (requires completable provider)                                                                                                                                                                |
| Agent planning (LLM)                           | BLOCKED (external)                                                                                                                                                                                     |
| Tool execution                                 | IMPLEMENTED · TESTED · RUNTIME_VERIFIED (terminal exec live; deterministic mission runner e2e)                                                                                                         |
| Approval enforcement                           | TESTED (workspace boundary: editor save/patch refuse escapes, `.git/*`, symlinks; terminal commands execute inside workspace root)                                                                     |
| File editing                                   | IMPLEMENTED · TESTED · UI_WIRED · runtime-probed (desktop command + HTTP parity deployed)                                                                                                              |
| Diff generation / patch apply                  | TESTED (git apply fail-closed, byte-faithful EOL contract)                                                                                                                                             |
| Test execution                                 | TESTED · RUNTIME_VERIFIED (deterministic mission path)                                                                                                                                                 |
| Failure recovery                               | TESTED (failure object taxonomy; First Mission repair + kill/resume)                                                                                                                                   |
| Evidence persistence                           | IMPLEMENTED · TESTED · RUNTIME_VERIFIED (hash chain intact in UI across restarts)                                                                                                                      |
| Session recovery                               | RUNTIME_VERIFIED (backend restarts 3× this session; missions and ledger survived each)                                                                                                                 |
| Dev web runtime                                | **DEV_WEB_RUNTIME_VERIFIED** (this document)                                                                                                                                                           |
| Packaged desktop runtime                       | UNVERIFIED (not attempted this pass; Tauri crate compiles — `cargo check` clean)                                                                                                                       |

## 9. Next executable work

1. Provider credential remediation is user-owned (fund DeepSeek or supply a
   valid OpenRouter/Anthropic key); the probe re-runs in seconds.
2. Wave B references/imports/dependency graph (deterministic, no LLM).
3. Attempt packaged Tauri desktop build/launch (INSTALLED_DESKTOP_RUNTIME).

---

## 10. Git checkpoint (recorded after push)

- **Branch:** `main` → `origin/main` (`https://github.com/zylvex-tech/zylcode.git`)
- **Checkpoint commits:**
  - `93eb3d2` — feat(mission): First Mission runner — failure taxonomy, real-tool e2e, kill/resume
  - `6b50a8b` — feat(runtime): auto-recovery for backend-dependent surfaces + editor write backends
  - (earlier same-day: `f9ba68b` branding, `58d7ce7` evidence foundation — pushed in the same checkpoint)
- **Push verification:** `git fetch origin` after push; local HEAD == `origin/main` == `6b50a8b`; ahead 0 / behind 0.
- **Verification date:** 2026-09-28
- **Gates at checkpoint (exact worktree):** cargo test **584 passed / 0 failed** (18 binaries), `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean, `cargo fmt --check` clean; frontend `pnpm test` **67/67**, `tsc --noEmit` clean. Secret gate on all staged patches: clean (no credentials, tokens, or headers committed).
- Note: this section was appended as a trailing docs commit because the checkpoint commits were already pushed; the docs commit itself necessarily does not contain its own SHA.
