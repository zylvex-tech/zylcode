# ZylCode — DataFactor Improvement Programme: Execution Plan

**Founder directive:** minimum verified score **80**, ambition **90**.
**Prepared:** 2026-10-30 · **Prepared by:** NEXUS
**Status:** AWAITING FOUNDER REVIEW of this plan, then execution proceeds under the authority
granted in §12 of the directive.

> Rule applied throughout: *the score is a measurement instrument, not the objective.* Every item
> below names the engineering defect it repairs. Items whose only effect would be to make a scanner
> report a nicer number are explicitly excluded and listed in §3.4.

---

## 1. Current repository state

| Item | Value | Evidence |
| --- | --- | --- |
| Branch | `feat/home-mission-pipeline` | `git branch --show-current` |
| HEAD | `d576aad` (2026-10-29 12:08:30 +0100) | `git log -1` |
| `origin/main` | `ff8e4fa` (merge of PR #43) | `git fetch --prune` |
| Divergence | `1 0` — main is 1 commit ahead, HEAD has 0 unique | `git rev-list --left-right --count` |
| **Tree parity** | **IDENTICAL** — `git diff origin/main HEAD` is empty | verified 2026-10-30 |
| Local commits | 264 · span 2026-08-31 → 2026-10-29 (59 days) | `git log` |
| Tags | 6 (v0.1.0, v0.2.0, v0.2.0-dev, v0.2.1, v0.2.2, v0.2.3) | `git tag` |
| Remote | 15 open PRs, all Dependabot/fix branches | `gh pr list` |

### 1.1 Uncommitted work — all preserved, nothing discarded

Seven paths are dirty. **None were reverted, stashed or reset.** They fall into two groups:

**A. Pre-existing sponsored work (Respan hosted gateway) — NOT MINE, do not disturb**

| File | Change |
| --- | --- |
| `crates/zylcode-core/src/router.rs` | +249 lines: `ModelProvider::Respan`, `RESPAN_GATEWAY_BASE_URL`, `RESPAN_DEFAULT_MODEL`, `env_nonempty`, `from_env`/`base_url` wiring |
| `crates/zylcode-core/tests/respan_gateway.rs` | **new**, 272 lines — loopback `StubGateway` test double, request-shape assertions, explicit "not evidence a Respan trace exists" disclaimer |
| `crates/zylcode-core/tests/live_commissioning.rs` | +9 — probes `ModelProvider::Respan` first when `RESPAN_API_KEY` present |
| `.env.example` | +24 — `RESPAN_API_KEY`, `RESPAN_BASE_URL`, `RESPAN_MODEL` with per-variable source line references |

**B. My earlier benchmark-margin work — reported, uncommitted, decision pending**

| File | Change |
| --- | --- |
| `crates/zylcode-core/src/intelligence/context.rs` | +72/−12 — retrieval ranking: multi-word file-anchor bonus (capped 1.4), anchor bonus extended to the file channel itself, symbol path-token weight 0.3 → 0.5 |
| `crates/zylcode-core/tests/repo_intelligence_benchmark.rs` | +13/−5 — **diagnostic dump only**: `take(10)` → `take(50)` with rank + HIT/MISS markers |

**Verification of (B):** no scoring threshold, assertion or metric was altered. The gate remains
`avg_precision >= 0.60` and `avg_recall >= 0.60`. Measured effect: Precision@10 **0.61 → 0.64**,
Recall@10 0.96 unchanged. Verified green: `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --all -- --check`, 108/108 `intelligence` lib tests, benchmark test itself.

**Decision required:** keep (it is a real ranking-quality fix to the product feature) or revert.
See §7.

### 1.2 Fresh baseline — measured 2026-10-30

| Gate | Command | Result |
| --- | --- | --- |
| Format | `cargo fmt --all -- --check` | **PASS** (exit 0) |
| Compile | `cargo check --workspace --all-targets` | **PASS** (exit 0, 3m11s) |
| Lint (Rust) | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (exit 0) |
| **Tests (Rust)** | `cargo test --workspace --all-targets` | **PASS — 932 passed, 0 failed**, 31 binaries |
| Lint (FE) | `pnpm --filter zylcode-desktop lint` | **PASS — 0 errors, 46 warnings** |
| Typecheck (FE) | `pnpm --filter zylcode-desktop typecheck` | **PASS** (exit 0) |
| Format (FE) | `pnpm --filter zylcode-desktop format:check` | **PASS** |
| **Tests (FE)** | `pnpm --filter zylcode-desktop test` | **PASS — 46 files, 453 tests** |
| Secrets | `python scripts/check_hardcoded_secrets.py` | **PASS — clean** |
| Retracted claims | `python scripts/check_retracted_claims.py` | **PASS** (11 baselined) |
| Rust advisories | `cargo audit` | **PASS** (exit 0; 2 *allowed* warnings) |
| npm advisories | `pnpm audit --prod` | **PASS — no known vulnerabilities** |
| **FE coverage** | `pnpm --filter zylcode-desktop test:coverage` | **PASS but 70.16% lines vs 70% floor — 0.16pt margin** |
| **Rust coverage** | `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` | **PASS — 80.61% lines** (42,082 lines, 8,160 missed) vs 78% floor = **2.61pt margin**; 74.85% functions; 80.57% regions; **0 test failures** |

**`cargo audit` allowed advisories (documented, not suppressed):** RUSTSEC-2024-0370
(`proc-macro-error`, unmaintained), RUSTSEC-2024-0429 (`glib`, unsound). 599 crates scanned.

**The single most important baseline number is the frontend coverage margin: 0.16 points.** That is
the direct cause of PR #18 failing, and it means *any* dependency bump perturbing coverage will fail
CI. This is a real fragility, not a scanner artifact — see F3 below.

**The single most important *engineering* number is where the Rust coverage is missing.** 8,160 of
those uncovered lines sit in just two files — both of them entry points:

| File | Lines | Uncovered | Line coverage |
| --- | ---: | ---: | ---: |
| `crates/zylcode-cli/src/main.rs` | 1,376 | 1,272 | **7.56%** |
| `apps/zylcode-desktop/src-tauri/src/main.rs` | 1,308 | 1,085 | **17.05%** |
| `crates/zylcode-core/src/computer_use/mod.rs` | 124 | 82 | 33.87% |
| `crates/zylcode-nav/src/model.rs` | 118 | 72 | 38.98% |
| `crates/zylcode-core/src/delivery.rs` | 306 | 181 | 40.85% |

Those two files alone hold **33% of every uncovered line in the workspace**, and the Tauri one is
precisely the IPC surface Phase 2 hardens. The aggregate 80.61% is healthy *because* the well-tested
core carries it; the boundary layer — where untrusted input actually arrives — is the least-tested
code in the repository. **Phase 2 and Phase 3 converge on the same fix: test the IPC boundary.**

---

## 2. Current PR inventory

15 open. **Correcting my earlier report: 11 of 15 are fully green, not 3.**

### 2.1 Green and merge-ready (all checks pass)

| PR | Title | Reviews | Merge state | Blocker |
| --- | ---: | ---: | --- | --- |
| #42 | framer-motion 11.18.2 → 14.6.0 | 1 | **CLEAN** | none — mergeable now |
| #41 | @testing-library/user-event 14.6.7 → 14.6.8 | 1 | **CLEAN** | none — mergeable now |
| #40 | eslint-plugin-react-refresh 0.4.26 → 0.5.7 | 1 | **CLEAN** | none — mergeable now |
| #39 | taiki-e/install-action 2.87.29 → 2.87.32 | 1 | **CLEAN** | none — mergeable now |
| #16 | tracing-opentelemetry 0.28 → 0.34 | 2 | BEHIND | needs branch update (`strict: true`) |
| #15 | axum 0.7.9 → 0.8.9 | 1 | BEHIND | needs branch update |
| #13 | actions/checkout 4 → 7 | 1 | BEHIND | needs branch update |
| #11 | opentelemetry-otlp 0.28 → 0.33 | 2 | BEHIND | needs branch update |
| #8 | actions/setup-node 4 → 7 | 1 | BEHIND | needs branch update |
| #9 | tauri-action 0 → 1 | **0** | BEHIND | needs a review |
| #37 | `ci(release)`: install xdg-utils for aarch64 bundle | 1 | **DIRTY** | **merge conflict — real fix, needs manual resolution** |

Note #16/#15/#11 are **major** bumps that are nevertheless fully green — but §12 still requires
founder approval for major migrations. Listed for approval, not for auto-merge.

### 2.2 Failing — root causes diagnosed from CI logs

| PR | Title | Failing check | Root cause |
| ---: | --- | --- | --- |
| #14 | rusqlite 0.31 → 0.41 | all 3 OS legs + `coverage` | `error[E0277]: the trait bound 'u64: FromSql' is not satisfied` at `apps/zylcode-desktop/src-tauri/src/main.rs:1159`. rusqlite 0.41 removed `FromSql for u64` (SQLite integers are `i64`). **Fix: read `i64`, cast, range-check.** Small, well-understood. |
| #12 | toml 0.8.2 → 1.1.8 | all 3 OS legs + `coverage` | fails at `Cargo test (workspace)` — toml 1.x API/parse-behaviour change |
| #17 | eslint-plugin-react-hooks 5.2.0 → 7.1.1 | all 3 OS legs (`Frontend lint`) | v7 breaks our ESLint 9 flat config; Coverage still passes |
| #18 | react + @types/react | **`frontend-coverage` only** | CI passes; the 0.16pt coverage margin is tripped by the bump |

### 2.3 Branch protection on `main` (verified via API)

```
required_status_checks (strict: true):
  - check / test / frontend (ubuntu-22.04)
  - check / test / frontend (windows-latest)
  - check / test / frontend (macos-latest)
  - coverage
required_pull_request_reviews: 1 approval
enforce_admins: true
required_conversation_resolution: true
allow_force_pushes: false · allow_deletions: false
```

**Gap found:** `frontend-coverage` is **NOT** a required status check, while `coverage` (Rust) is.
A PR can merge with a failing frontend coverage gate. That is an inconsistency worth closing — see
F4.

---

## 3. Findings ranked by engineering value and risk

### 3.1 Tier A — high value, low risk (within granted authority)

| ID | Finding | Engineering defect | Est. impact | Risk |
| --- | --- | --- | --- | --- |
| **F4** | `frontend-coverage` not a required check while `coverage` is | The two coverage gates are not equivalent in enforcement; the FE gate can be bypassed on merge | CI/CD + Test | **LOW** |
| **F3** | Frontend coverage margin is **0.16pt** (70.16 vs 70) | The gate is effectively at the trip wire; any dependency bump perturbs it (proven by #18). Real fragility | Test Coverage | LOW |
| **F1** | Root-level lint/format not discoverable; no root `format:check` | Configs exist but only package-scoped, so tooling and humans looking at the repo root find nothing; root `pnpm lint` has no `format:check` counterpart | Code Cleanliness | **LOW** |
| **F2** | CI gate names not machine-readable | Quality gates genuinely run but a scanner reading step names cannot see them | CI/CD Maturity | LOW |
| **F6** | 4 stale Dependabot PRs green for days, #37 real fix conflicted | Maintenance backlog is not converging — the actual defect behind the History & Maintenance score | Dependency Health + History | LOW |
| **F8** | No documented fresh-clone procedure | The working build sequence exists only implicitly in `ci.yml`; a newcomer cannot reproduce a build from docs | Docs & Onboarding | LOW |

### 3.2 Tier B — high value, medium risk (within authority, needs careful testing)

| ID | Finding | Engineering defect | Est. impact | Risk |
| --- | --- | --- | --- | --- |
| **F5** | 4 input-taking Tauri commands lack `iv::` validation | See §3.3 for the corrected count. Genuine hardening gap on `terminal_exec` and `register_mcp_bridge` | Architecture + Security | **MEDIUM** |
| **F7** | No per-command validation coverage table | The mapping of command → validator is undocumented, so coverage cannot be reviewed or asserted | Architecture | LOW |
| **F9** | Weakest FE modules uncovered: `repoIntel.ts` 46%, `version.ts` 70%, `terminal.ts` 76%, `toolRunner.ts` 76%, `gitStatus.ts` 76% | Test gaps concentrated exactly where the product's risk is (terminal execution, tool runner, repo intel) | Test Coverage | LOW |

**Correcting my earlier report on validation coverage.** I previously said "6 of 17 input-taking
commands unvalidated." Re-reading the bodies: **`set_provider_config` and `reorder_provider_chain`
already contain substantial inline validation** (`timeout_ms` range-checked `1000..=300_000`; empty
order, duplicate kinds and unknown providers all rejected). They are not `iv::`-wrapped, but they
are not unvalidated either. **The genuine gap is 4 commands, not 6:**

| Command | Actual state |
| --- | --- |
| `terminal_exec` | deserialises a `TerminalRequest` and drives shell execution — **needs bounds validation**. Priority 1. |
| `register_mcp_bridge` | `id`, `endpoint`, `transport` unvalidated — **needs validation**. Priority 2. A test already exists (`register_mcp_bridge_rejects_invalid_transport`). |
| `marketplace_search` | `query: String` unvalidated — needs `iv::validate_query` (1,024 char cap already defined). |
| `package_start` | `version: Option<String>` unvalidated — needs bounds. |
| `set_provider_config` | **already validates `timeout_ms` range inline** — wrap, don't redesign. |
| `reorder_provider_chain` | **already validates empty/duplicate/unknown inline** — wrap, don't redesign. |

This is exactly the "do not blindly add validators that break existing workflows" instruction, and
the corrected count matters: rewriting the two that already validate would have been unnecessary risk.

### 3.3 Tier C — high value, high risk (**founder approval required**)

| ID | Finding | Engineering defect | Est. impact | Risk |
| --- | --- | --- | --- | --- |
| **F10** | God-file sprawl: **17 files >1000 LOC, 57 >500** (DataFactor scored 15/50 — it has *grown*) | Reviewability, modularity, the C-rubric's 500 LOC ceiling | Code Cleanliness (**up to ~9**, largest single lever) | **HIGH** |

Order: `first_mission/mod.rs` (2535) → `agent/mod.rs` (2312) → `real_tools.rs` (2233) →
`router.rs` (2198) → `queries.rs` (2085) → `cli/main.rs` (2130).

> ⚠ **`router.rs` is currently carrying ~249 lines of uncommitted sponsored work.** It must not be
> refactored until that work is committed and green. It is deliberately sequenced after the others.

### 3.4 Excluded — would raise the score without improving the engineering

Listed so the decision is visible and auditable, **not** to be actioned:

- Editing test fixtures or the secret-detector patterns to silence DataFactor's 3 false positives.
- Adding a root ESLint/Prettier config *with different rules* (the fix is placement, not policy).
- Raising coverage thresholds before measuring actuals.
- Splitting files to hit a line target rather than along real seams.
- Any commit-history manipulation, fabricated contributor or timestamp change.

### 3.5 Cannot be engineering-fixed

| ID | Finding | Reality |
| --- | --- | --- |
| **F11** | History & Maintenance = 38 | 59-day span, one human author. DataFactor's own words: *"nothing cosmetic fixes this."* Only sustained cadence and, if genuine, a second contributor move it. Fabrication is prohibited by §10 of the directive. |

---

## 4. Expected implementation sequence

| Phase | Scope | Authority | Gate to exit |
| --- | --- | --- | --- |
| **0** | Merge #42, #41, #40, #39 (CLEAN); update-behind and merge #16, #15, #13, #11, #8; resolve #37 conflict; request review on #9 | **Granted** (low-risk, independently verifiable) | All merged PRs green on `main` |
| **0b** | Diagnose and fix #18 (FE coverage margin) by **adding tests**, not moving the threshold | **Granted** | `frontend-coverage` green |
| **1** | F1 root lint/format discoverability + root `format:check`; F2 CI gate naming; F4 make `frontend-coverage` required; F8 README "Fresh Clone Setup" | **Granted** | CI green on a branch; docs verified by executing them |
| **2** | F5 validate the 4 genuine gaps (priority `terminal_exec` → `register_mcp_bridge`); F7 coverage table | **Granted** | `cargo test -p zylcode-core input_validation` + full workspace green + desktop IPC smoke |
| **3** | F9 tests for the weakest FE modules; then raise thresholds **only to measured values** | **Granted** | Measured coverage ≥ threshold with real margin |
| **4** | F10 god-file extraction, one PR each, LOC order | **⛔ FOUNDER APPROVAL REQUIRED** | 4 gates green per PR, sizes re-measured |
| **5** | F11 cadence; ADR for error-tracking absence; tracing/release documentation | **Granted** (ADR is documentation) | Docs merged |
| **6** | F14 major dependency migrations (#14 rusqlite, #12 toml, #17 eslint-hooks, #16/#15/#11 majors) | **⛔ FOUNDER APPROVAL REQUIRED** | Per-item migration plan + all gates green |

### 4.1 Dependencies between tasks

```
Phase 0  ──► Phase 0b ──► Phase 3 (thresholds can only rise once margin exists)
   │
   └──────► Phase 1 (F1/F2 land after backlog clears, so CI history stays readable)
                 │
                 └──► F4 (make FE coverage required) is SAFE ONLY AFTER Phase 0b
                       otherwise every open PR turns red.
Phase 2 ── independent of all others; can start immediately.
Phase 4 ── BLOCKED on founder approval, and F10's `router.rs` item is additionally
            BLOCKED on the sponsored Respan work being committed first.
Phase 6 ── BLOCKED on founder approval; #14/#12/#17 each need their own migration plan.
```

**Critical ordering constraint:** F4 (making `frontend-coverage` required) must not land before
Phase 0b, because the gate currently sits at a 0.16pt margin — enforcing it immediately would block
every open PR including the green ones.

---

## 5. Verification requirements

**Every change, without exception:**

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --all-targets
pnpm --filter zylcode-desktop lint
pnpm --filter zylcode-desktop typecheck
pnpm --filter zylcode-desktop format:check
pnpm --filter zylcode-desktop test
python scripts/check_hardcoded_secrets.py
python scripts/check_retracted_claims.py
cargo audit && pnpm audit --prod
```

**Phase-specific:**
- **Phase 2:** `cargo test -p zylcode-core input_validation`, plus positive *and* negative tests per
  command, plus a manual desktop IPC smoke of the touched commands.
- **Phase 3:** `pnpm --filter zylcode-desktop test:coverage` and
  `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` — thresholds raised only to
  values already observed passing with margin.
- **Phase 4 (per PR):** the four cargo gates green **and** re-measured LOC counts recorded in the
  improvement register.
- **Phase 0 (per PR):** CI + both coverage checks green *on that PR* before merge.

**Evidence policy:** every register row marked VERIFIED must cite the command run and its output.
A row is never VERIFIED because code was written.

---

## 6. Founder approval requirements

**Required before I act (per §12):**

1. **Phase 4 / F10** — the first high-risk architectural extraction. Specifically:
   - approve the extraction seam for `first_mission/mod.rs`, and
   - note `router.rs` is deferred until the sponsored Respan work is committed.
2. **Phase 6 majors** — rusqlite #14, toml #12, eslint-plugin-react-hooks #17, and the green majors
   #16 (tracing-opentelemetry), #15 (axum), #11 (opentelemetry-otlp), #9 (tauri-action).
3. **F4** — making `frontend-coverage` a required status check (a branch-protection change).
4. **F9-adjacent** — any change to coverage threshold values.
5. **The pending benchmark change** (§1.1 B): **keep or revert** the `context.rs` ranking work.
6. **Privacy/telemetry** — whether to integrate an error tracker or write an ADR recording its
   deliberate absence. (My recommendation: **ADR only.** ZylCode is local-first; shipping stack
   traces to a third party is a product decision, not a CI fix.)

**No approval needed (within granted authority):** Phase 0 merges of already-green PRs, Phase 1
discoverability, Phase 2 validation, Phase 3 tests, Phase 5 documentation, and any low-risk
independently verifiable improvement.

---

## 7. DataFactor reassessment schedule

| Milestone | Target | Reassess when |
| --- | ---: | --- |
| Starting baseline | **62.7** | — (2026-10-29 assessment) |
| First milestone | 70+ | After Phases 0 + 0b + 1 land |
| Second milestone | 75+ | After Phases 2 + 3 |
| Main objective | **80+** | After Phase 4 (first three extractions) |
| Long-term ambition | 90 | Only after Phases 4 + 5 + sustained Phase 6 cadence |

**Protocol per §11 — after each phase:** record the commit and merged PRs → run the full suite →
record coverage → record `cargo audit`/`pnpm audit --prod` → re-measure structural metrics (LOC
counts, validation coverage, coverage %) → identify remaining findings → obtain a fresh DataFactor
assessment → compare against the previous score.

**Score progression table — to be maintained; an estimate is never reported as an achieved score:**

| Milestone | Target | Measured | Date | Notes |
| --- | ---: | ---: | --- | --- |
| Starting baseline | 62.7 | **62.7** | 2026-10-29 | DataFactor report |
| First milestone | 70+ | — | — | |
| Second milestone | 75+ | — | — | |
| Main objective | 80+ | — | — | |
| Long-term ambition | 90 | — | — | |

**Known cap:** F11 (History & Maintenance) is time-bound. Even if every engineering category reached
~90, a 38 in History would hold the composite below 90. The honest path to 90 runs through calendar
time and real cadence, not through further code changes. **80 is achievable by engineering. 90 is
achievable only by engineering *plus* time.**

---

## 8. Immediate next step

Proceeding under §12 authority with **Phase 0** (merging the four CLEAN PRs and clearing the
backlog) and **Phase 0b** (widening the frontend coverage margin with real tests), because both are
low-risk, independently verifiable, and unblock every later phase.

**Holding for your instruction on:** §6 items 1–6 — most urgently the **keep/revert decision on the
benchmark change**, and approval to begin Phase 4.
