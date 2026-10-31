# DataFactor Findings Matrix — zylvex-tech/zylcode

**Assessment under review:** DataFactor score 62.7 (threshold 70, ambition 90)
**Evidence base:** local checkout, branch `feat/home-mission-pipeline` @ `d576aad`, content-identical to
`origin/main` @ `ff8e4fa` (PR #43 merge; `git rev-list --left-right --count origin/main...HEAD` = `1 0`).
**Date:** 2026-10-30
**Status:** INVESTIGATION ONLY — no remediation started pending founder approval.

> Method: every DataFactor claim was re-derived against the current tree rather than the scored
> snapshot. Each row states the claim, the measured evidence, a validity verdict, and the smallest
> legitimate correction. Expected benefits are **estimates, not guarantees**.

---

## 0. Re-measured baseline vs. the DataFactor snapshot

| Measure | DataFactor snapshot | Measured at HEAD | Delta |
| --- | ---: | ---: | --- |
| Commits | 265 | 264 | −1 (counting differs) |
| Commit span | 42 days | 59 days (2026-08-31 → 2026-10-29) | **+17d** |
| Human authors | 1 | 1 human + dependabot + `verify@zylcode.local` | unchanged in substance |
| Semver tags | 6 | 6 (v0.1.0 … v0.2.3) | ✓ |
| Files > 1000 LOC | 15 | **17** | **+2 (worse)** |
| Files > 500 LOC | 50 | **57** | **+7 (worse)** |
| Largest file | `first_mission/mod.rs` 2354 | `first_mission/mod.rs` **2535** | **+181 (worse)** |
| README lines | 255 | 424 | +169 |
| Tauri commands | 36 | **37** | +1 |

**Headline:** the snapshot is stale in our favour on age and docs, and **against us on code
cleanliness** — the god-file problem has grown since scoring, not shrunk.

---

## 1. Findings matrix

### F1 — Code Cleanliness: oversized modules (DataFactor: up to ~9 pts)

| Field | Value |
| --- | --- |
| **Claim** | 15 files > 1000 LOC, 50 files > 500 LOC; `first_mission/mod.rs` 2354, `agent/mod.rs` 2107, `real_tools.rs` 2046, `queries.rs` 1972, `main.rs` 1971 |
| **Evidence** | Tracked files only: **17 > 1000 LOC, 57 > 500 LOC**, 102,402 rs+ts+tsx LOC. Ranked: `crates/zylcode-core/src/first_mission/mod.rs` 2535, `crates/zylcode-core/src/agent/mod.rs` 2312, `crates/zylcode-mcp/src/real_tools.rs` 2233, `crates/zylcode-core/src/router.rs` 2198, `crates/zylcode-cli/src/main.rs` 2130, `crates/zylcode-nav/src/queries.rs` 2085, `crates/zylcode-core/src/terminal.rs` 1840, `apps/zylcode-desktop/src-tauri/src/main.rs` 1692, `crates/zylcode-nav/src/parse.rs` 1629, `crates/zylcode-core/src/cli.rs` 1596 |
| **Valid?** | **VALID — and understated.** Both counts have grown since the snapshot. |
| **Proposed correction** | Behaviour-preserving module extraction, highest-LOC first, one PR per extraction: (a) `first_mission/mod.rs` → split spec parsing/plan building (partially separated already in `first_mission/spec.rs`); (b) `agent/mod.rs` → extract `AgentSession`/`AgentTask`/`ToolCall` types into `agent/types.rs`; (c) `real_tools.rs` → extract `ToolError`/`RiskLevel`/`ToolEvidence` into `real_tools/types.rs`; (d) `router.rs`, (e) `queries.rs`, (f) `cli/main.rs`. No logic changes, no API changes. |
| **Files affected** | 6+ Rust modules plus their `mod.rs` re-exports and tests |
| **Risk** | **HIGH** — largest change class here. Mitigate: one file per PR, `cargo test --workspace` + `clippy -D warnings` green before merge, no signature changes. |
| **Verification** | `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && cargo test --workspace --all-targets` |
| **Expected benefit** | Up to ~9 pts (DataFactor's own estimate). **Not guaranteed**; the rubric may weight the count of oversized files rather than the total, so partial splitting may yield partial credit. |
| **Dependencies** | None. Largest single lever. |

### F2 — Code Cleanliness: `has_lint_config=false`

| Field | Value |
| --- | --- |
| **Claim** | No discoverable lint/format config despite `package.json` lint scripts |
| **Evidence** | **Both exist and are tracked:** `apps/zylcode-desktop/eslint.config.js` (ESLint 9 flat config, `eslint-plugin-react` + `react-hooks` + `@typescript-eslint`, error-level rules documented and verified via `eslint --print-config`) and `apps/zylcode-desktop/.prettierrc.json` (printWidth 100, trailingComma all, eol lf). `pnpm -r lint` **exits 0** — 46 warnings, 0 errors. Root `package.json` has `lint: "pnpm -r lint"` and `typecheck: "pnpm -r typecheck"` but **no root `format:check`**. |
| **Valid?** | **INVALID as stated — scanner discoverability failure.** Configs are package-scoped; DataFactor looked for a root-level config. |
| **Proposed correction** | Add a root-level `.prettierrc.json` and root `eslint.config.js` (or a root config that re-exports the package one) so the tooling is discoverable at the repo root; add a root `format:check` script mirroring `lint`. No rule changes — this is placement, not new policy. |
| **Files affected** | `/.prettierrc.json` (new), `/eslint.config.js` (new), `/package.json` |
| **Risk** | **LOW** — additive. Risk of double-linting the package; scope the root config to paths not already covered. |
| **Verification** | `pnpm -r lint && pnpm -r format:check && pnpm -r typecheck` |
| **Expected benefit** | ~1–3 pts on Code Cleanliness, and removes the "configuration exists in name only" narrative. |
| **Dependencies** | Should land **before** F3 so CI and config agree. |

### F3 — CI/CD: `ci_runs_lint=false`, `ci_runs_typecheck=false`, `ci_runs_tests=false`

| Field | Value |
| --- | --- |
| **Claim** | CI does not run lint, typecheck, or tests |
| **Evidence** | `.github/workflows/ci.yml`, single job `check` (`check / test / frontend (...)`) on a 3-OS matrix (`ubuntu-22.04`, `windows-latest`, `macos-latest`), runs **in order**: `pnpm install --frozen-lockfile` → `cargo check --workspace --all-targets` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo fmt --all -- --check` → **`cargo test --workspace --all-targets`** → `check_retracted_claims.py` → `cargo audit` → `pnpm audit --prod` → `check_hardcoded_secrets.py` → **`pnpm --filter zylcode-desktop lint`** → **`pnpm --filter zylcode-desktop format:check`** → N-ATLAS `typecheck` → N-ATLAS smoke → **`pnpm --filter zylcode-desktop build`** (`tsc && vite build`). Recent runs on `main`: green. |
| **Valid?** | **INVALID — scanner failure on all three.** The steps exist and gate. |
| **Proposed correction** | Split the monolithic `check` job into named jobs (`lint`, `typecheck`, `test`, `rust`) **or** add a lightweight `quality-gates` job whose name and step names contain the literal tokens the scanner keys on. The underlying enforcement is already correct; this is a naming/structure change for machine-readability, not new gating. |
| **Files affected** | `.github/workflows/ci.yml` |
| **Risk** | **MEDIUM** — restructuring a working 3-OS matrix can change wall-clock cost and cache behaviour. Alternative lower-risk option: keep the job, add explicit `name:` values that state the gate. |
| **Verification** | Push to a branch, confirm all required checks still appear and pass; confirm a deliberately broken lint step fails the run. |
| **Expected benefit** | ~2–4 pts on CI/CD Maturity (65 → mid-70s plausible). |
| **Dependencies** | None. Pairs with F2. |

### F4 — CI/CD: `ci_has_deploy=false`, no separate deploy stage

| Field | Value |
| --- | --- |
| **Claim** | No deploy stage |
| **Evidence** | `.github/workflows/release.yml` **exists**: tag-triggered (`on.push.tags`), `permissions: contents: write`, 3-platform matrix, installs Tauri system deps, `Build desktop app assets` → `Build Tauri app`, publishes a GitHub Release. This *is* the distribution stage. |
| **Valid?** | **PARTIALLY VALID.** There is no deploy stage *in ci.yml* — which is correct for a desktop app that ships via tagged releases rather than a continuous deploy. The scanner read `ci.yml` only. |
| **Proposed correction** | No structural change required. Optionally reference `release.yml` from README/CONTRIBUTING so the release path is discoverable, and confirm the scanner reads workflow files other than `ci.yml`. |
| **Files affected** | README.md / CONTRIBUTING.md (docs only) |
| **Risk** | **LOW** |
| **Verification** | `gh workflow list`; confirm `Release` workflow runs on a tag. |
| **Expected benefit** | ~1–2 pts, mostly by removing a "missing" flag. |
| **Dependencies** | None |

### F5 — CI/CD: clippy `continue-on-error` off-Linux

| Field | Value |
| --- | --- |
| **Claim** | Clippy is `continue-on-error` on non-Linux |
| **Evidence** | `ci.yml`: `continue-on-error: ${{ runner.os != 'Linux' }}`. Clippy **is** a hard gate on Linux; advisory on Windows/macOS where Tauri system deps cause differing lints. `cargo fmt --check` is a hard failure on all three OS. |
| **Valid?** | **VALID as stated, LOW severity.** The gate genuinely runs and genuinely fails. |
| **Proposed correction** | Investigate the actual cross-OS clippy deltas; if they are few, make it hard-fail everywhere. If they are inherent, document *which* lints differ and where they are still enforced. Do not simply delete the flag — that would create a red build on Windows/macOS and get the step disabled. |
| **Files affected** | `.github/workflows/ci.yml`, possibly `clippy.toml` |
| **Risk** | **MEDIUM** — flipping it to hard-fail may immediately break 2 of 3 OS legs. |
| **Verification** | Run the matrix on a branch before merging. |
| **Expected benefit** | ~1 pt |
| **Dependencies** | Should follow a clippy cross-OS investigation, not precede it. |

### F6 — Test Coverage (62)

| Field | Value |
| --- | --- |
| **Claim** | `test_spec_files=46`, `test_source_ratio` 1:6, `coverage.yml` gates Rust ≥78% lines + vitest thresholds, but `ci_runs_tests=false` |
| **Evidence** | 46 TS/TSX test files ✓. **24** Rust integration test files under `crates/*/tests/`, plus **100** source files containing `#[cfg(test)]`. `coverage.yml` runs `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` (hard gate) **and** `pnpm --filter zylcode-desktop test:coverage` with vitest thresholds `lines 70 / branches 83 / functions 80 / statements 70`. `cargo test --workspace --all-targets` runs in `ci.yml`. |
| **Valid?** | `ci_runs_tests=false` → **INVALID (scanner miss)**. Ratio and file counts → **VALID**. |
| **Proposed correction** | (a) Fold test-running into a clearly named job/step (F3 does this). (b) Raise the frontend `lines` threshold from 70 toward the Rust 78 bar as coverage allows. (c) Consider making `coverage.yml` a required check. |
| **Files affected** | `apps/zylcode-desktop/vitest.config.ts`, `.github/workflows/coverage.yml`, `ci.yml` |
| **Risk** | **LOW–MEDIUM** — raising a threshold can turn CI red; must be raised only after measuring current actuals. |
| **Verification** | `pnpm --filter zylcode-desktop test:coverage` and `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` locally. |
| **Expected benefit** | ~3–5 pts |
| **Dependencies** | Measure current frontend line coverage **first** to pick a safe threshold. |

### F7 — Architecture & Robustness / Security: input validation across Tauri commands

| Field | Value |
| --- | --- |
| **Claim** | `input_validation_patterns` empty in stats, despite `iv::validate_prompt` existing; coverage across 36 commands undocumented |
| **Evidence** | **`crates/zylcode-core/src/input_validation.rs` EXISTS** — 14 `validate_*` functions (`prompt`, `task`, `query`, `model`, `correlation_id`, `path`, `tool_id`, `mode`, `artifact`, `patch`, `tool_args`, `ensure_max_chars`) and 10 `MAX_*` size constants (e.g. `MAX_PROMPT_CHARS` 32,768, `MAX_ARGS_BYTES` 64 KiB, `MAX_ARTIFACT_BYTES` 4 MiB). Imported as `use zylcode_core::input_validation as iv;`. **37** `#[tauri::command]` functions at HEAD. Measured coverage: |

| Category | Count | Commands |
| --- | ---: | --- |
| No input parameters (validation N/A) | 20 | `missions_list`, `mission_run_next`, `terminal_reset`, `app_version`, `git_status`, `repo_file_tree`, `evidence_ledger`, `token_metrics`, `verify_logic`, `list_mcp_bridges`, `list_tools`, `get_provider_configs`, `get_provider_scorecard`, `deploy_status`, `build_status`, `build_start`, `clear_vector_cache`, `get_cache_stats`, `init_telemetry_db`, `get_recent_audit_logs` |
| Takes input **with** `iv::` validation | 11 | `process_intent`, `process_intent_stream`, `preview_execution_plan`, `repo_context`, `nav_query`, `mission_enqueue`, `repo_search`, `file_content`, `save_workspace_artifact`, `apply_patch`, `execute_tool` |
| Takes input **without** `iv::` validation | **6** | **`terminal_exec`**, **`register_mcp_bridge`**, `marketplace_search`, `package_start`, `set_provider_config`, `reorder_provider_chain` |

| Field | Value |
| --- | --- |
| **Valid?** | "Empty patterns" → **INVALID (scanner miss)**. Underlying coverage gap → **VALID: 11 of 17 input-taking commands are validated (65%); 6 are not**, and there is no coverage table. |
| **Proposed correction** | (1) Add validators for the 6 — priority order: `terminal_exec` (deserialises a `TerminalRequest` and drives shell execution) and `register_mcp_bridge` (`endpoint` + `transport` are attacker-controlled strings that name a command/endpoint) first; then `marketplace_search` (`validate_query`), `package_start`, `set_provider_config` (endpoint/timeout bounds), `reorder_provider_chain` (bounds + membership). (2) Add the documented coverage table DataFactor asked for: one row per command, listing its validator — as a doc comment in `input_validation.rs` and mirrored in README. (3) Unit tests asserting each validator rejects oversized/malformed payloads. |
| **Files affected** | `crates/zylcode-core/src/input_validation.rs`, `apps/zylcode-desktop/src-tauri/src/main.rs`, tests |
| **Risk** | **MEDIUM** — adding validation to live commands can reject payloads that currently work. Mitigate: bounds set from observed legitimate values, plus a test per command. |
| **Verification** | `cargo test -p zylcode-core input_validation && cargo test --workspace --all-targets` and a manual smoke of the desktop IPC surface. |
| **Expected benefit** | ~4–6 pts (Architecture 72 + Security 58 both cite this) |
| **Dependencies** | None. Highest value **per unit of risk** after F2/F3. |

### F8 — Security: `hardcoded_secret_hits=3`

| Field | Value |
| --- | --- |
| **Claim** | 3 hardcoded secret hits — "test fixtures/detector script not real secrets" |
| **Evidence** | `python scripts/check_hardcoded_secrets.py` → **`secret scan: clean -- no credential-shaped literal in tracked files.` exit 0.** The script runs as a hard CI gate and is allowlist-free by design (high-confidence provider formats only; `--selftest` asserts detection). DataFactor's own email concedes the 3 hits are its own detector patterns and fixtures. |
| **Valid?** | **INVALID — scanner false positive.** No action on the repository warranted. |
| **Proposed correction** | None to source. Optionally document in README that the repo carries its own secret scanner, which explains the apparent conflict to a human reviewer. |
| **Files affected** | none / README.md |
| **Risk** | none |
| **Verification** | `python scripts/check_hardcoded_secrets.py; echo $?` → 0 |
| **Expected benefit** | ~1–2 pts if the scanner can be told; otherwise 0 (do **not** edit fixtures to hide patterns — that would be gaming). |

### F9 — Architecture: `logging_framework=null`, `error_tracking=null`

| Field | Value |
| --- | --- |
| **Claim** | Both null, though tracing/tracing-subscriber are configured |
| **Evidence** | `Cargo.toml`: `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }`. The workspace also carries `opentelemetry`, `opentelemetry_sdk`, `tracing-opentelemetry` (visible in Dependabot updates). No Sentry/Bugsnag/error-reporting service integration. |
| **Valid?** | `logging_framework=null` → **INVALID (scanner miss)** — likely keyed on `log`/`env_logger`/`slog`. `error_tracking=null` → **VALID**, but see next row. |
| **Proposed correction** | Document `tracing` as the logging framework in README's architecture section so the fact is discoverable. For error tracking: decide deliberately — ZylCode is a local desktop/CLI tool, so shipping stack traces to a third party raises a privacy question rather than being an obvious win. Either integrate a privacy-preserving tracker with explicit opt-in, or record the decision as an ADR stating why server-side error tracking is intentionally absent. |
| **Files affected** | `README.md`, possibly `docs/governance/` ADR |
| **Risk** | **LOW** (docs/ADR). Adding an SDK would be **MEDIUM** (privacy, dependency weight). |
| **Verification** | Documentation review; if integrating, a live event round-trip. |
| **Expected benefit** | ~1–3 pts on Architecture (72) |
| **Dependencies** | Requires a founder decision on opt-in telemetry policy. |

### F10 — Dependency Health (68): `direct_runtime_deps=0`, 599 transitive, 0.x pins

| Field | Value |
| --- | --- |
| **Claim** | 0 direct runtime deps (odd/incomplete manifest signal); many 0.x-pinned crates |
| **Evidence** | `direct_runtime_deps=0` is a **manifest-reading artifact**: root `package.json` declares only `typescript` under `devDependencies` and has no `dependencies` block, while `apps/zylcode-desktop/package.json` carries **8 real runtime dependencies** (`react`, `react-dom`, `@tauri-apps/api`, `@tauri-apps/plugin-shell`, `@monaco-editor/react`, `@xterm/xterm`, `@xterm/addon-fit`, `framer-motion`). Both lockfiles committed ✓; Dependabot configured for cargo + npm + github-actions ✓; `cargo audit` and `pnpm audit --prod` both run in CI ✓. **0.x pins: VALID — 35 of 59 numeric-versioned Cargo dependencies are 0.x** (`rusqlite 0.31`, `opentelemetry 0.28`, `axum 0.7`, `tracing-opentelemetry 0.28`, …). |
| **Valid?** | `direct_runtime_deps=0` → **INVALID (scanner read root manifest only)**. 0.x pins → **VALID**. |
| **Genuine issue found during verification** | **10 open Dependabot PRs, several failing CI, the oldest open since 2026-10-21 (8+ days):** `#14 rusqlite 0.31→0.41` (CI + Coverage **failing**), `#15 axum 0.7.9→0.8.9`, `#16 tracing-opentelemetry 0.28→0.34`, `#17 eslint-plugin-react-hooks 5.2.0→7.1.1` (CI **failing**), `#18 react + @types/react` (Coverage **failing**), plus `#37 fix/release-linux-xdg-utils` (a real fix, unmerged since 2026-10-27), `#39/#40/#41/#42`. This is the actual dependency-health signal — and it also feeds History & Maintenance, because unmerged work reads as stalled cadence. |
| **Proposed correction** | Triage the 10 open PRs: land the trivially green ones (#39–#42), fix the three failing ones properly (rusqlite major → API migration; eslint-plugin-react-hooks v7 → peer-range/config update; react bump → diagnose the Coverage failure). Land `#37`. This is ordinary maintenance, not score-chasing. |
| **Files affected** | `Cargo.lock`, `pnpm-lock.yaml`, `eslint.config.js`, affected source |
| **Risk** | **MEDIUM** for the three majors (breaking changes); **LOW** for the green ones. |
| **Verification** | Each PR: full CI + Coverage green before merge; `cargo audit && pnpm audit --prod`. |
| **Expected benefit** | ~3–5 pts (Dependency Health 68 → high-70s plausible), plus cadence signal for F11. |
| **Dependencies** | None; can start immediately. |

### F11 — History & Maintenance (38): 42 days, 1 author, 265 commits

| Field | Value |
| --- | --- |
| **Claim** | Single human author, 42-day span, thin maintenance signal |
| **Evidence** | Span has grown to **59 days** (2026-08-31 → 2026-10-29). 264 commits. Author identities: `zylforge@zylvex.tech` / `worldsilenceday@gmail.com` (one human), `49699333+dependabot[bot]@users.noreply.github.com`, `verify@zylcode.local` (local verification identity). 6 tags. **10 open PRs aged 1–9 days** — Dependabot activity exists but is not converging. |
| **Valid?** | **VALID in substance, stale in numbers.** One human author is a real, structural property of the repository. |
| **Proposed correction** | Only legitimate levers: (a) merge the backlog so cadence reads as sustained rather than bursty; (b) keep landing small single-purpose commits; (c) if a genuine second contributor exists, have them commit under their own identity — **fabricating one is explicitly forbidden and would be a material misrepresentation.** |
| **Files affected** | none (process) |
| **Risk** | **NONE** for process changes. **PROHIBITED** for any fabricated history. |
| **Verification** | `git shortlog -sn`, `git log --format='%ae' \| sort -u` over time. |
| **Expected benefit** | DataFactor's own estimate: up to ~5 pts, **but explicitly time-bound — "nothing cosmetic fixes this."** Cannot be reached by engineering alone. |
| **Dependencies** | Calendar time. This is the item that gates 90. |

### F12 — Docs & Onboarding (82): no reproducible fresh-clone path

| Field | Value |
| --- | --- |
| **Claim** | No evidence that one documented command sequence builds and tests cleanly from a clean machine |
| **Evidence** | README is **424 lines** (not 255) and covers install/run/test/env/contributing/architecture/usage/api. `.env.example` present (7.7 KB) with every referenced var ✓. `CONTRIBUTING.md` present (9.6 KB) ✓. Dockerfile + docker-compose + devcontainer present ✓. **Genuine gap: no "Fresh clone setup" section** — `grep -iE "fresh clone\|apt-get" README.md` returns **nothing**, while `ci.yml` carries the authoritative Linux dependency list (`libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`). The exact working command sequence is therefore not written down anywhere a newcomer can follow. |
| **Valid?** | **VALID.** |
| **Proposed correction** | Add a **"Fresh clone setup"** section to README: (a) the apt/brew system packages, mirrored from `ci.yml`; (b) the exact sequence `pnpm install --frozen-lockfile` → `cargo check --workspace --all-targets` → `pnpm --filter zylcode-desktop build` → `cargo test --workspace --all-targets` → `pnpm --filter zylcode-desktop test:coverage`; (c) actually run it end-to-end on a clean checkout first and record any system dep that was missing. |
| **Files affected** | `README.md` (and only if the dry-run reveals a missing dep) |
| **Risk** | **LOW** — documentation, but the dry-run is the point: it may surface a real reproducibility bug. |
| **Verification** | Execute the documented sequence in a pristine clone/CI job. |
| **Expected benefit** | ~1–3 pts on Docs (82 → high-80s) and removes a buyer-confidence flag. |
| **Dependencies** | Requires a clean-checkout dry run. |

---

## 2. Summary — valid vs. scanner artifacts

| # | Finding | Verdict |
| --- | --- | --- |
| F1 | Oversized modules | **VALID, and worse than scored** (17/57 vs 15/50) |
| F2 | `has_lint_config=false` | **Scanner miss** — configs exist, package-scoped |
| F3 | `ci_runs_lint/typecheck/tests=false` | **Scanner miss** — all three run on 3 OS |
| F4 | `ci_has_deploy=false` | **Partial** — `release.yml` is the deploy stage |
| F5 | clippy continue-on-error off-Linux | **VALID**, low severity (gates on Linux) |
| F6 | `ci_runs_tests=false` | **Scanner miss**; ratios **VALID** |
| F7 | `input_validation_patterns` empty | **Scanner miss on the module**; **coverage gap VALID (6 of 17 input commands unvalidated)** |
| F8 | `hardcoded_secret_hits=3` | **Scanner false positive** — own scanner reports clean |
| F9 | `logging_framework=null` | **Scanner miss**; `error_tracking=null` **VALID** |
| F10 | `direct_runtime_deps=0` | **Scanner miss**; **0.x pins VALID**; **10 stale/failing PRs genuinely VALID** |
| F11 | History 38 | **VALID**, numbers stale (59d not 42d); **not engineering-fixable** |
| F12 | Fresh-clone reproducibility | **VALID** — no documented sequence |

**Scanner artifacts: F2, F3, F6 (part), F8, F9 (part), F10 (part).** Six of the twelve findings are at
least partly a discoverability problem rather than an engineering one.
**Genuinely engineering-fixable: F1, F7, F10 (PR backlog), F12, plus F3/F2 discoverability.**
**Not fixable by engineering: F11.**

---

## 3. Proposed phased plan — NOT STARTED, awaiting approval

**Phase 0 — Unstick the maintenance signal (est. +2–4 pts, LOW risk, ~half a day)**
Merge the green Dependabot PRs (#39–#42), land `#37`, triage the three failing ones.
*Verify:* all merged PRs show CI + Coverage green; `cargo audit && pnpm audit --prod` clean.
*Why first:* it is ordinary maintenance, needs no approval of design, and immediately improves two
categories while removing the "stalled backlog" reading.

**Phase 1 — Discoverability (est. +3–7 pts, LOW risk, ~half a day)**
F2 root lint/format config + root `format:check`; F3 explicitly named CI gate steps (optionally split
jobs); F4/F9 documentation of the release path and tracing; F12 README "Fresh clone setup" (after a
real clean-checkout dry run).
*Verify:* `pnpm -r lint && pnpm -r format:check && pnpm -r typecheck`; CI on a branch shows the named
checks; the documented fresh-clone sequence executed verbatim.
*Note:* this phase does not change product behaviour at all — it makes existing, real engineering
visible to a machine reader. **No source edits to satisfy a scanner.**

**Phase 2 — Genuine security/robustness gaps (est. +4–6 pts, MEDIUM risk, 1–2 days)**
F7: validate the 6 uncovered input-taking commands (`terminal_exec` and `register_mcp_bridge` first),
add the per-command coverage table, add unit tests. F6: measure current frontend coverage, then raise
the vitest `lines` threshold toward 78.
*Verify:* `cargo test -p zylcode-core input_validation`, full `cargo test --workspace --all-targets`,
`pnpm --filter zylcode-desktop test:coverage`, plus a manual desktop IPC smoke.
*Note:* thresholds are raised only after measuring actuals — never set a gate to a value we have not
observed passing.

**Phase 3 — God-file refactors (est. up to ~9 pts, HIGH risk, several PRs over days)**
F1, strictly in LOC order, **one extraction per PR**, behaviour-preserving, no API changes:
`first_mission/mod.rs` → `agent/mod.rs` → `real_tools.rs` → `router.rs` → `queries.rs` → `cli/main.rs`.
*Verify per PR:* `cargo check --workspace --all-targets && cargo clippy --workspace --all-targets --
-D warnings && cargo fmt --all -- --check && cargo test --workspace --all-targets`, all green before
merge; re-measure the >1000/>500 counts after each.
*Constraint:* split along existing seams only. **No rewriting to hit a line target.**

**Phase 4 — Sustained history (time-bound, no shortcut)**
F11: keep merging real, small, tested changes on a steady cadence. No fabricated contributors, no
cosmetic commits, no rewritten history.

### Sequencing rationale
Phase 0 → 1 → 2 are all low-to-medium risk, independently verifiable, and address four separate
DataFactor categories; together they plausibly reach and clear 70. Phase 3 is where the remaining
large block lives but carries the most regression risk, so it is deliberately last and deliberately
incremental. Phase 4 is the only path to 90 and cannot be compressed.

**Benefit figures are DataFactor's own estimates plus my reading of which flag each fix clears.
None are guaranteed, and the scorer's exact weighting is unknown.**

---

## 4. Open decisions requiring founder input

1. **F3** — split the CI job into named jobs (better discoverability, higher regression risk) vs. add
   explicit step `name:` values only (lower risk, weaker signal)?
2. **F9** — integrate an opt-in error tracker, or record an ADR stating that server-side telemetry is
   intentionally absent from a local-first tool?
3. **F10** — take the three major-version Dependabot bumps (rusqlite 0.41, axum 0.8,
   eslint-plugin-react-hooks 7) now, or defer them behind Phases 1–2?
4. **F5** — investigate and harden cross-OS clippy, or leave it gating on Linux and document why?
5. **Pre-existing uncommitted change:** during the earlier (separately requested) benchmark-margin
   task I edited `crates/zylcode-core/src/intelligence/context.rs` and
   `crates/zylcode-core/tests/repo_intelligence_benchmark.rs`. That work is **verified but
   uncommitted** and is covered in the chat report. Confirm keep or revert.
