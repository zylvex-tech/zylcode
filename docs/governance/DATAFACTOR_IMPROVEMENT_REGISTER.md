# ZylCode — DataFactor Improvement Register (live)

**Directive:** minimum verified score **80**, ambition **90** · **Baseline: 62.7 (2026-10-29)**
**Opened:** 2026-10-30 · **Owner:** NEXUS
**Status vocabulary (§13):** NOT STARTED · INVESTIGATING · READY FOR IMPLEMENTATION · IN PROGRESS ·
BLOCKED · IMPLEMENTED · TESTING · VERIFIED · MERGED · REASSED

> **Evidence policy:** a row is `VERIFIED` only when the command and its output are recorded in the
> Tests/Results column. Code having been written is not verification.

---

## Register

| # | DF finding | Est. impact | Actual underlying problem | Proposed correction | Status | Files changed | PR | Tests performed | Results | Regression risk | DF re-score | Remaining work |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| R-01 | Code Cleanliness — `has_lint_config=false` | ~1–3 | ESLint 9 flat config + `.prettierrc.json` exist but are package-scoped; repo root declares no config, and root `package.json` has `lint` but no `format:check` | Add root-level ESLint/Prettier config that delegates to the package config (placement, **not** new rules); add root `format:check` | **NOT STARTED** | `/eslint.config.js` (new), `/.prettierrc.json` (new), `/package.json` | — | `pnpm -r lint && pnpm -r typecheck && pnpm -r format:check` | — | LOW | — | Must not duplicate or weaken existing rules |
| R-02 | CI/CD — `ci_runs_lint/typecheck/tests=false` | ~2–4 | All three genuinely run in `ci.yml`, but step/job names are not machine-readable | Explicit `name:` values naming each gate; **do not** restructure the 3-OS matrix without demonstrated benefit | **NOT STARTED** | `.github/workflows/ci.yml` | — | branch CI run, all required checks present and green | — | LOW | — | Confirm scanner reads new names |
| R-03 | CI/CD — `frontend-coverage` not required | ~1–2 | `coverage` (Rust) is a required check; `frontend-coverage` is not. A PR can merge with FE coverage failing | Add `frontend-coverage` to required status checks | **BLOCKED** | branch protection (API) | — | — | — | LOW | — | ⛔ **Must land AFTER R-04**, or every open PR turns red |
| R-04 | Test Coverage — 0.16pt FE margin | ~3–5 | FE coverage 70.16% lines vs 70% floor. Directly causes PR #18 to fail; any dep bump perturbations trip the gate | **Add tests** to widen margin (do **not** raise the threshold first); then raise thresholds only to measured values | **IN PROGRESS** | `apps/zylcode-desktop/src/**` tests | — | `pnpm --filter zylcode-desktop test:coverage` | baseline: **70.16 / 83.25 / 80.61 / 70.16** (lines/branch/funcs/stmts) vs floor 70/83/80/70 | LOW | — | Weakest: `repoIntel.ts` 46%, `version.ts` 70%, `terminal.ts` 76%, `toolRunner.ts` 76%, `gitStatus.ts` 76% |
| R-05 | Security — unvalidated Tauri commands | ~4–6 | **Corrected from 6 → 4.** `set_provider_config` and `reorder_provider_chain` already validate inline. Genuine gaps: `terminal_exec` (drives shell), `register_mcp_bridge` (endpoint/transport), `marketplace_search`, `package_start` | Add `iv::` validation with realistic bounds derived from each command's purpose; positive **and** negative tests; per-command coverage table | **NOT STARTED** | `crates/zylcode-core/src/input_validation.rs`, `apps/zylcode-desktop/src-tauri/src/main.rs` | — | `cargo test -p zylcode-core input_validation`, full workspace, desktop IPC smoke | — | **MEDIUM** | — | ⚠ Do not break existing workflows; `register_mcp_bridge_rejects_invalid_transport` already exists |
| R-06 | Architecture — no command→validator table | ~2 | Mapping is undocumented, so coverage cannot be reviewed or asserted | Doc table in `input_validation.rs` covering all 37 commands, mirrored in README | **NOT STARTED** | `input_validation.rs`, `README.md` | — | doc review + CI retracted-claim guard | — | LOW | — | Depends on R-05 landing |
| R-07 | Docs — no fresh-clone procedure | ~1–3 | README has no system-dep list or verified command sequence; `ci.yml` holds the authoritative apt list | Execute full sequence on a **pristine checkout** first, then document "Fresh Clone Setup" | **NOT STARTED** | `README.md` | — | execute documented commands verbatim on clean clone | — | LOW | — | ⚠ Do not document unverified commands |
| R-08 | Dependency Health — 11 green PRs stale | ~2–4 | #42/#41/#40/#39 are CLEAN and unmerged; #16/#15/#13/#11/#8 green but BEHIND; #9 has 0 reviews; #37 conflicts | Merge CLEAN ones now; update-behind then merge; review #9; resolve #37 | **IN PROGRESS** | — | #42 #41 #40 #39 #16 #15 #13 #11 #8 #9 #37 | each PR's CI + both coverage checks green before merge | 11/15 confirmed green | LOW | — | ⛔ #16/#15/#11 are **major** bumps → founder approval per §12 |
| R-09 | Dependency Health — 4 red PRs | ~2–3 | Diagnosed: #14 rusqlite `u64: FromSql` removed at `main.rs:1159`; #12 toml 1.x API change; #17 eslint-plugin-react-hooks v7 breaks flat config; #18 = R-04 | Per-PR migration plan, then fix | **INVESTIGATING** | — | #14 #12 #17 #18 | full CI + coverage | #18 unblocked by R-04 | MEDIUM | — | ⛔ Major migrations need founder approval |
| R-10 | Code Cleanliness — god files (up to ~9) | **~9 (largest)** | **17 files >1000 LOC, 57 >500** — *worse* than DataFactor's 15/50 snapshot | Extract types/responsibilities along existing seams; one extraction per PR; LOC order | **BLOCKED** | `first_mission/mod.rs`, `agent/mod.rs`, `real_tools.rs`, `router.rs`, `queries.rs`, `cli/main.rs` | — | the 4 cargo gates + re-measure LOC after each | baseline sizes recorded (2535/2312/2233/2198/2085/2130) | **HIGH** | — | ⛔ **FOUNDER APPROVAL REQUIRED**. `router.rs` additionally blocked until sponsored Respan work is committed |
| R-11 | Test Coverage — weakest FE modules | ~2–4 | `repoIntel.ts` 46% line coverage is the worst in the tree | Tests for terminal exec, tool runner, repo intel, git status, version | **NOT STARTED** | `apps/zylcode-desktop/src/**` | — | `pnpm --filter zylcode-desktop test:coverage` | — | LOW | — | Feeds R-04 |
| R-15 | Test Coverage — IPC boundary least tested | ~3–5 | **Baseline finding:** `src-tauri/main.rs` (all 37 commands) is **17.05%** covered — 1,085 of 1,308 lines uncovered; `zylcode-cli/main.rs` **7.56%**. Those two files hold 33% of every uncovered line in the workspace | Add tests at the IPC boundary in the same PRs as R-05 validation, so each new validator lands with positive **and** negative coverage | **NOT STARTED** | `apps/zylcode-desktop/src-tauri/`, `crates/zylcode-cli/` | — | `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` | baseline: **80.61% lines**, 0 failures | LOW | — | Converges with R-05 — highest-leverage pairing in the programme |
| R-12 | Architecture — `error_tracking=null` | ~1–3 | No Sentry/error SDK. For a local-first tool this is a **product/privacy** decision, not a gap | **Recommend ADR only** recording deliberate absence; do not add a third-party tracker without a founder privacy decision | **BLOCKED** | `docs/governance/` ADR | — | doc review | — | LOW | — | ⛔ Privacy/telemetry policy = founder decision (§12) |
| R-13 | History & Maintenance — 38 | ~5 (time-bound) | 59-day span, 1 human author. Cannot be engineering-fixed | Sustained cadence, focused commits, resolve stale work. **Fabrication prohibited** | **IN PROGRESS** | — | — | `git shortlog -sn` over time | baseline: 264 commits, 4 identities (1 human + dependabot + local verify) | none | — | Calendar time; gates 90 |
| R-14 | Intelligence benchmark margin | — (internal gate) | ZylCode's own repo-intelligence ranking sat at 0.61 vs its 0.60 floor — 3 hits of headroom, so any added file could tip CI | Ranking quality fix: multi-word file-anchor bonus, anchor extended to file channel, path weight 0.3→0.5. **No assertion/threshold altered** | **IMPLEMENTED — awaiting founder keep/revert** | `crates/zylcode-core/src/intelligence/context.rs`, `tests/repo_intelligence_benchmark.rs` | — | clippy, fmt, 108 intelligence tests, benchmark test | **0.61 → 0.64** precision, recall 0.96, all green | LOW | — | ⛔ **FOUNDER DECISION: keep or revert** |

---

## Structural metric re-measurement (updated after each phase)

| Metric | DataFactor snapshot | Baseline 2026-10-30 | After Phase | After Phase |
| --- | ---: | ---: | ---: | ---: |
| Files > 1000 LOC | 15 | **17** | | |
| Files > 500 LOC | 50 | **57** | | |
| Largest file | 2354 | **2535** | | |
| Tauri commands | 36 | **37** | | |
| Input commands validated | — | **11 of 15** (4 gaps) | | |
| FE line coverage | — | **70.16%** (floor 70) | | |
| Rust line coverage | — | **80.61%** (floor 78, margin 2.61pt) | | |
| Worst-covered file | — | `zylcode-cli/main.rs` **7.56%** · `src-tauri/main.rs` **17.05%** | | |
| Uncovered lines in those 2 files | — | **2,657 of 8,160 (33% of all)** | | |
| Open PRs | — | **15** (11 green, 4 red, 1 conflicted) | | |
| Commit span | 42 d | **59 d** | | |

---

## Score progression

| Milestone | Target | **Measured** | Date | Evidence |
| --- | ---: | ---: | --- | --- |
| Starting baseline | 62.7 | **62.7** | 2026-10-29 | DataFactor report |
| First milestone | 70+ | — | — | pending reassessment |
| Second milestone | 75+ | — | — | pending reassessment |
| Main objective | **80+** | — | — | pending reassessment |
| Long-term ambition | 90 | — | — | pending reassessment |

*An estimated score is never recorded as an achieved score. A score is achieved only when supported
by an actual DataFactor assessment.*
