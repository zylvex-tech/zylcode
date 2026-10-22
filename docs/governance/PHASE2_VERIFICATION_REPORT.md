# Phase 2 — Independent Verification and Remediation

**Date:** 2026-10-21 (updated 15:45)
**Scope:** Read-only verification, then remediation. No pushes, no merges, no repository-settings
changes. Every number below was measured in this session; where a check could not be executed it is
recorded as BLOCKED or NOT RUN with the exact reason.

---

## 1. EXECUTIVE SUMMARY

**The coverage gate now genuinely works.** This was the largest open item and it is now resolved
with hard evidence: the coverage workflow on `main` completed end-to-end and measured
**80.63% line coverage against the 78% threshold**. Before this session the gate had run three
times and died at the test stage every time, so the threshold had never once been evaluated and no
coverage figure had ever been emitted.

**A real test-isolation defect was found, root-caused, and fixed.** The
`NETWORK_EGRESS_COUNT` spy was process-global while the lock protecting it was module-private, so a
dispatching test in another module of the same crate could invalidate four tests' no-egress
assertions. The mechanism is proven, not inferred, and the arithmetic matches the observed failure
exactly. The fix is thread-local scoping and touches no production code.

**What remains unresolved:** `main` still has no branch protection of any kind, so none of the CI
work is *required* to pass before a merge; and the direct dependency `lru` 0.12.5 carries two unsound
advisories, one of whose affected APIs is called in production code (§7.1).

| | |
|---|---|
| Verified resolved | Coverage gate + measured coverage; PR #22 merge; egress-spy race (**fix written AND test-verified**, §5.1) |
| Newly delivered | `.env.example`, rewritten CONTRIBUTING §§1-3 |
| Still open | Branch protection on `main`; `lru` 0.12.5 unsound advisories (§7.1) |
| Committed locally | 3 commits on `fix/router-egress-spy-thread-local` |

---

## 2. VERIFIED STARTING STATE

Verified at session start, then re-verified after a `git fetch`:

| Item | Value |
|---|---|
| Repository root | `C:/Users/MBARIESERVICESLTD/WorkBuddy/Worktrees/zylcode/origin-main-8cdd991f` |
| Branch at start | `ci/coverage-fetch-depth` |
| HEAD at start | `5b6cc0a4f6d38c10a967ef56bbb9245d46f13a9f` |
| `origin/main` at start | `0c3d7345c71cf72e271681da466d520cdbff3649` |
| Working tree at start | clean except the untracked `docs/governance/PHASE2_VERIFICATION_REPORT.md` |
| User's separate checkout | `C:/Projects/zylcode`, `competition/natlas-2026` @ `c5c9138…` — **distinct, untouched** |

**The state changed mid-session.** `origin/main` moved `0c3d734 → ea23092`:

```
ea23092  Merge pull request #22 from zylvex-tech/ci/coverage-fetch-depth
5b6cc0a  ci(coverage): fetch full history so git-history tests match the main CI job
0c3d734  Merge pull request #6 from zylvex-tech/ci/coverage-gate
```

**PR #22 is MERGED.** Verified via the API, not assumed:

- `state = MERGED`, `mergedAt = 2026-10-21T14:30:01Z`, merge commit `ea230920cb`
- Its four checks all report **SUCCESS**: `coverage`,
  `check / test / frontend (ubuntu-22.04)`, `(windows-latest)`, `(macos-latest)`
- The merge is clean: `git diff 5b6cc0a ea23092` is empty, and the merge commit's parents are
  `0c3d734` and `5b6cc0a`

No owner action is required for PR #22 — it is already merged.

---

## 3. ROOT-CAUSE FINDINGS

### 3.1 The coverage gate was failing because of shallow history — CONFIRMED, and now fixed on main

Before PR #22, `coverage.yml` used `actions/checkout@v4` with the default depth-1 history while
`ci.yml` used `fetch-depth: 0`. `context_builder_uses_intelligence_ranking` derives co-change
evidence from `git log`, so with no history it could not surface `agent.rs` and failed.

**Proven by experiment, not inference.** The prebuilt `target/debug/zylcode.exe` was run against
the failing test's exact query in two trees:

| Tree | Result |
|---|---|
| Full history (173 commits) | `agent.rs` appears at score 0.97, cited *"changes together with 'crates/zylcode-core/tests/crash_recovery.rs' in git history"* |
| `--depth 1` clone | `agent.rs` **absent entirely**; `first_mission::FirstMission::maybe_crash` appears instead; no co-change citation produced at all |

That matches the CI failure output exactly. PR #22's `fetch-depth: 0` is the correct and sufficient
fix for this failure, and it is now on `main`.

### 3.2 A second, independent failure: the egress-spy race — CONFIRMED and FIXED

PR #22's own coverage run still failed, on a different test:

```
router::tests::d1_vector_cache_cross_prompt_contamination_guard  (router.rs:1872)
assertion `left == right` failed: the D1 guard test must not perform network egress
  left: 5
 right: 3
```

**Mechanism, confirmed by source inspection:**

- `NETWORK_EGRESS_COUNT` was a **process-global** `#[cfg(test)] static AtomicU64`
  (`router.rs:22`), incremented once per `call_provider` immediately before `req.send()`.
- **Four** tests assert `egress_before == egress_after` on it — not three as an earlier revision of
  this report stated: `synthetic_offline_dispatch_returns_parseable_payload`,
  `synthetic_offline_is_immune_to_ambient_api_keys`,
  `non_synthetic_provider_without_keys_still_degrades_offline`, and
  `d1_vector_cache_cross_prompt_contamination_guard`.
- All four hold `DISPATCH_MUTEX` — so my initial hypothesis, that the four were racing each other,
  was **wrong** and was discarded on the evidence.
- The actual hole: `DISPATCH_MUTEX` is **private to `router.rs`'s test module**. A dispatching test
  in any other module of the same crate cannot acquire it, yet shares the counter, because
  `cargo test` runs all of a crate's `#[cfg(test)]` modules in one binary on multiple threads.

**The interfering test, identified:** `pipeline_offline_generates_artifacts_and_passes_verification`
(`pipeline.rs:834`) dispatches through `RouterConfig::default()`, which sets
`fallback_provider: LocalOllama`. That makes the no-keys fast path's `both_need_keys` guard
`false`, so the fast path is bypassed and the primary provider is attempted, then its Ollama
fallback — **two HTTP attempts per dispatch**, which is exactly the observed `left: 5, right: 3`.

**Corroborated in this environment:** `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY` and
`DEEPSEEK_API_KEY` are all set here (presence and length only checked; values never printed), so the
mechanism is live.

**Fix:** the spy is now thread-local. `cargo test` runs each test on its own thread and
`#[tokio::test]` defaults to a current-thread runtime, so a thread-local counter is scoped to exactly
one test and no sibling can perturb it. This removes the whole class of interference rather than
serialising against tests this module cannot see. Both the spy and its single increment site were
already `#[cfg(test)]`, so **no production code changed**. No assertion was loosened, removed, or
ignored.

### 3.3 Disproven / corrected hypotheses

| Earlier claim | Verdict |
|---|---|
| "three tests share the counter" | **Wrong.** Four do. Corrected by exhaustive grep. |
| "the four tests race each other" | **Wrong.** All four hold `DISPATCH_MUTEX`. Discarded. |
| "measured baseline 80.03" (commit `497fec2`) | **Now confirmed, superseded.** The gate never emitted a figure before; it now measures **80.63%**. |

---

## 4. CHANGES MADE

Two commits on `fix/router-egress-spy-thread-local`, based on `origin/main` (`ea23092`).
**Local only — not pushed.**

| Commit | Files | Purpose |
|---|---|---|
| `69f1715` | `crates/zylcode-core/src/router.rs` | Scope the test-only egress spy per thread instead of per process. 46 insertions, 19 deletions — 10 mechanical edits (1 declaration, 1 increment, 8 reads) plus 2 comment blocks. All inside `#[cfg(test)]` code. |
| `683fc61` | `.env.example` (new, 124 lines), `CONTRIBUTING.md` | Document the developer environment from validated source references. |

`.env.example` carries 16 environment variables, each with its exact `file:line` read site and the
default the code falls back to when unset. **All 16 citations were machine-verified** to resolve to
the code they claim. Every assignment is an empty placeholder; there is no credential-shaped value
in the file, confirmed by the secret scanner with the file staged.

---

## 5. TEST RESULTS

PASS = executed and passed. FAIL = executed and failed. BLOCKED = could not execute.
NOT RUN = not executed. UNVERIFIED = evidence insufficient.

### Executed locally on commit `5e0e555` (2026-10-22, after the sandbox blocker cleared)

| Check | Command | Result | Exit |
|---|---|---|---|
| Rust formatting | `cargo fmt --all -- --check` | **PASS** | 0 |
| Router tests | `cargo test -p zylcode-core --lib router::` | **PASS** (32 passed, 0 failed) | 0 |
| Core lib tests | `cargo test -p zylcode-core --lib` | **PASS** (499 passed, 0 failed) | 0 |
| Full workspace tests | `cargo test --workspace --all-targets` | **PASS** (exit 0; every test binary green) | 0 |
| Clippy, warnings denied | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (0 source lints; 21 sandbox lock-file warnings only) | 0 |
| Locked resolution | `cargo metadata --locked --format-version 1` | **PASS** | 0 |
| Secret scan | `python scripts/check_hardcoded_secrets.py` | **PASS** (clean) | 0 |
| Secret-scan control | `python scripts/check_hardcoded_secrets.py --selftest` | **PASS** (13 detect, 29 no-match) | 0 |
| Retracted-claim guard | `python scripts/check_retracted_claims.py` | **PASS** (11 baselined) | 0 |
| Rust advisories | `cargo audit` | **0 vulnerable**; **6 informational warnings** — see §7.1 | 0 |
| JS advisories | `pnpm audit --prod` | **PASS** (none known) | 0 |
| Frontend lint | `pnpm --filter zylcode-desktop lint` | **PASS** (0 errors, 36 warnings) | 0 |
| Frontend format | `pnpm --filter zylcode-desktop format:check` | **PASS** | 0 |
| Frontend types | `pnpm --filter zylcode-desktop typecheck` | **PASS** | 0 |
| Frontend tests | `pnpm --filter zylcode-desktop test` | **PASS** (16 files, **108 tests**) | 0 |
| N-ATLAS SDK types | `pnpm --filter @zylvex/natlas-bridge typecheck` | **PASS** | 0 |
| N-ATLAS SDK smoke | `node --experimental-strip-types apps/natlas-sdk/test/smoke.mjs` | **PASS** (**16 passed, 0 failed**) | 0 |

### Not executed locally

| Check | Status | Reason |
|---|---|---|
| Rust coverage measurement, locally | **NOT RUN** | Requires `cargo-llvm-cov` and a full instrumented build. Measured instead in CI (§6). |

### 5.1 The fix is now test-verified — superseding the earlier "compile-verified only" note

**The Rust tests ran and passed on 2026-10-22.** An earlier revision of this section said the fix
was compile-verified only and that the tests had not been run. That is no longer true: the sandbox
blocker cleared, and the full battery in §5 executed green on commit `5e0e555`.

The run that matters most for the egress-spy fix:

```
$ cargo test -p zylcode-core --lib router::
running 32 tests
test router::tests::synthetic_offline_dispatch_returns_parseable_payload ... ok
test router::tests::synthetic_offline_is_immune_to_ambient_api_keys ... ok
test router::tests::non_synthetic_provider_without_keys_still_degrades_offline ... ok
test router::tests::d1_vector_cache_cross_prompt_contamination_guard ... ok
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 467 filtered out; finished in 5.26s
```

All four tests that assert `egress_after == egress_before` are present in that list and passed. Note
that `synthetic_offline_is_immune_to_ambient_api_keys` passed **with provider credentials present in
this environment's shell** — which is precisely the condition that used to perturb the process-global
counter.

### 5.1.1 A genuine defect in the change itself, found and fixed

The first clippy run after the edit reported exactly one error:

```
error: unused doc comment
  --> crates/zylcode-core/src/router.rs:17:1
   | rustdoc does not generate documentation for macro invocations
   = note: `-D unused-doc-comments` implied by `-D warnings`
```

`thread_local!` is a macro invocation, so `///` doc comments cannot attach to it. This was a real
defect in the change, caught by compile verification, and fixed by using `//` instead. It is recorded
here rather than quietly dropped.

### 5.1.2 Clippy output contains sandbox noise — distinguished, not hidden

The clippy run printed 21 warnings, all of them:

```
warning: error deleting lock file for incremental compilation session directory
`\\?\...\target\debug\incremental\<crate>-<hash>\s-<hash>.lock`: Access is denied. (os error 5)
```

These are the sandbox refusing to delete incremental-compilation lock files, **not** source lints. The
distinction was verified rather than assumed: filtering the output for `^warning|^error` while
excluding the lock-file lines and the `generated N warning` summaries leaves **nothing**. There are
zero source-code lints, and clippy exits 0 under `-D warnings`.

### 5.2 Remote evidence (GitHub Actions)

| Run | Workflow | Branch / SHA | Conclusion |
|---|---|---|---|
| 39617320736 | Coverage | `main` @ `ea23092` | **success** (measured 80.63%, see §6) |
| 39617320807 | CI | `main` @ `ea23092` | **success** — all 3 matrix jobs |
| 39618023835 | release.yml | `main` @ `aae6b4e` | **failure**, zero jobs (see below) |
| 39600712193 | release.yml | `main` @ `aae6b4e` | **failure**, zero jobs (see below) |
| PR #22 (4 runs) | CI + Coverage | `ci/coverage-fetch-depth` @ `5b6cc0a` | **all success** |

**Correction to an earlier interim reading.** Run 39617320807 was recorded as `cancelled` while it
was still executing. It has since completed: `completed/success`, with all three matrix jobs
(`ubuntu-22.04`, `windows-latest`, `macos-latest`) succeeding. The corrected value is the one above.

**The two `release.yml` failures are explained, and benign.** Both runs report
`event=push`, `head_branch=main`, `head_sha=aae6b4e`, `triggering_actor=zylvex-tech`, and contain
**zero jobs**. The evidence:

- `aae6b4e` is a 2026-10-03 commit that is an ancestor of the merge `ea23092`, and the
  `release.yml` it carries is the *historical* version, which has both a `branches:` filter and a
  `tags:` filter, and names an invalid `releases: write` permission. GitHub rejects that workflow at
  validation time, so it fails with zero jobs before any step starts — exactly what its own header
  comment describes.
- `ci.yml` and `coverage.yml` on that commit have `branches:`-only filters. A *branch* push to `main`
  would have triggered all three workflows. Only `release.yml` ran. Therefore the triggering ref was
  **not** a branch push to `main`; it was a ref that only `release.yml`'s `tags:` filter matches, with
  GitHub reporting `head_branch=main` because the tagged commit lives on `main`.
- No tag currently points at `aae6b4e` (`v0.1.0`, `v0.2.0`, `v0.2.0-dev` point elsewhere), so the ref
  was created and removed again.
- The `release.yml` now on `main` is tag-only (`tags: ['v*']`) and carries no invalid permission, so
  this failure mode cannot recur from the current file.

The exact ref that was pushed is not recoverable from the API (it no longer exists), so the trigger
mechanism is recorded as **inferred from trigger-filter evidence, not directly observed**. What *is*
directly observed: no release was published, no jobs ran, and `main` itself is unaffected.

---

## 6. COVERAGE — MEASURED

**Status: PASS.** This is the first coverage measurement in the repository's history that was
produced by a completed run. The three prior runs (39599616288 and its predecessors) all failed or
were cancelled before the summary stage.

| Field | Value |
|---|---|
| Workflow run | `39617320736` — "Coverage" |
| Trigger | `push` |
| Branch / SHA | `main` @ `ea230920cbe39c17e9ea55f0dec47e988632a669` |
| Started | 2026-10-21T14:30:04Z |
| Job | `coverage` — step "Coverage gate (lines >= 78%)" |
| Step conclusion | **success** |
| Command | `cargo llvm-cov --workspace --summary-only --fail-under-lines 78` |
| Threshold | 78% lines (`--fail-under-lines 78`) |

### 6.1 Measured line coverage

Extracted verbatim from the run log (`gh run view 39617320736 --log`):

```
TOTAL    60576    11731    80.63%    4175    1080    74.13%    40132    7816    80.52%    0    0    -
```

| Metric | Value |
|---|---|
| **Line coverage** | **80.63%** |
| Regions | 74.13% |
| Functions | 80.52% |
| Instrumented lines | 60,576 |
| Missed lines | 11,731 |
| Covered lines | 48,845 |
| Threshold | 78% |
| **Margin over threshold** | **+2.63 percentage points** |

This supersedes the "80.03% baseline" figure quoted in commit `497fec2`, which was an estimate rather
than a measurement. 80.63% is measured, from a run that completed.

### 6.2 Test outcomes inside the coverage run

| Metric | Value |
|---|---|
| `test result:` lines in the log | 30 |
| Tests passed | **840** |
| Tests failed | **0** |
| Tests with a non-zero `failed` count | **0** (every one of the 30 lines reads `; 0 failed;`) |

### 6.3 Was the 78% threshold genuinely evaluated?

**Yes.** Evidence, not assumption:

1. `--fail-under-lines 78` is present in the command line of the "Coverage gate" step in
   `.github/workflows/coverage.yml`.
2. The run reached and printed the `llvm-cov` summary table. `cargo-llvm-cov` runs the tests first
   and **exits with code 101 on test failure before it ever computes or prints coverage**. Printing
   the table therefore proves the test stage completed successfully *and* that control reached the
   threshold comparison.
3. The step concluded `success`. Had 80.63% been below 78, the same command would have exited
   non-zero and the step would have failed. It did not.
4. The `fetch-depth: 0` from PR #22 is present in this workflow file, so the history-dependent
   repository-intelligence inputs were fully materialised when the gate ran.

### 6.4 Residual coverage risk

The margin is **+2.63 pp**, which is real but not generous. 11,731 missed lines remain. Nothing in
this repository is currently gating *region* or *function* coverage (74.13% and 80.52% respectively);
only lines are gated. Raising the line threshold is **not** recommended at this margin — see §12.

---

## 7. SECURITY AND DEPENDENCY RESULTS

**Status: PASS for vulnerabilities. This section was corrected on 2026-10-22 — see §7.1.** No check
was skipped, and no vulnerability was suppressed, ignored, or allow-listed.

| Check | Command | Result |
|---|---|---|
| Rust vulnerability audit | `cargo audit` | **PASS** — `vulnerabilities: {found: false, count: 0, list: []}` |
| Rust advisory *warnings* | `cargo audit` | **6 warnings** — see §7.1. This is **not** a pass. |
| Frontend production audit | `pnpm audit --prod` | **PASS** — no findings |
| Lockfile integrity | `cargo … --locked` (CI) | **PASS** — `Cargo.lock` is in sync with `Cargo.toml` |
| Secret scan | `gitleaks` step in `ci.yml` | **PASS** — no findings |
| Secret-scanner selftest | `gitleaks detect --redact` selftest step | **PASS** — scanner proven able to fail |
| Retracted-claims guard | retracted-claim scan step | **PASS** |

### 7.1 Correction: the earlier "warnings: 0" claim was wrong

An earlier revision of this section reported `cargo audit` as `vulnerabilities.count: 0, warnings: 0`
and stated that "no unmaintained-dependency or yanked-crate advisories [are] outstanding." **The
warnings half of that claim was false.** It came from a parsing error: `cargo audit --json` emits a
top-level `warnings` object whose `list` key is *always empty*; the real warnings are keyed by
category (`warnings.unmaintained`, `warnings.unsound`, `warnings.yanked`). Reading `warnings.list`
therefore reports zero regardless of how many warnings exist.

The verified result, from `cargo audit` against 598 locked crate dependencies:

| Category | Count | Advisories |
|---|---|---|
| unmaintained | 2 | `proc-macro-error` 1.0.4 (RUSTSEC-2024-0370); `unsafe-libyaml` 0.2.11 (RUSTSEC-2024-0449) |
| unsound | 3 | `glib` 0.18.5 (RUSTSEC-2024-0429); **`lru` 0.12.5 (RUSTSEC-2026-0002)**; **`lru` 0.12.5 (RUSTSEC-2026-0253)** |
| yanked | 1 | `tokio` 1.54.0 |
| **vulnerabilities** | **0** | — |

**The material finding: `lru` 0.12.5 is a *direct* workspace dependency**
(`Cargo.toml:67 lru = "0.12"`, `crates/zylcode-core/Cargo.toml:24 lru = { workspace = true }`), and it
carries two `unsound` advisories — not a transitive one-off:

- **RUSTSEC-2026-0253** — "Potential use-after-free due to lack of panic safety in `LruCache::pop()`".
  Patched in `>= 0.18.2`.
- **RUSTSEC-2026-0002** — "`IterMut` violates Stacked Borrows by invalidating internal pointer".
  Patched in `>= 0.16.3`.

**Exposure in this codebase, verified by reading the call sites:**

| Advisory API | Called here? | Evidence |
|---|---|---|
| `LruCache::pop()` | **YES** | `crates/zylcode-core/src/router/cache.rs:62` — `guard.pop(&key);` inside `SpeculativeCache::get()`, on TTL expiry |
| `LruCache::iter_mut()` | No | `LruCache` is used in exactly one place (`router/cache.rs:19,27`) and only `get`/`put`/`len`/`clear`/`pop` are called on it. The `iter_mut()` calls elsewhere in the tree (`missions.rs`, `artifact_bus.rs`, `factory/graph.rs`, `intelligence/manifest.rs`, `router.rs:1642`, `nav/index.rs`) are all on `Vec`/slice types. |

So one of the two unsound APIs **is** exercised by production code. This is a real, currently-unfixed
defect in a direct dependency, and it is **not** in the Dependabot queue — Dependabot sees `0.12.5`
as up to date within the `"0.12"` requirement, so it will never propose the bump.

`lru` 0.18.3 and 0.19.0 are available on crates.io. The fix is a workspace requirement bump, and it
is **not applied here** because it is a dependency-version change requiring owner approval and a
full re-verification pass (see §14).

### 7.2 Other caveats

- `pnpm audit --prod` excludes dev dependencies by design. Dev-only advisories are therefore **not
  covered** by this result.
- The secret scan runs on the CI runner against the checked-out tree. It has **not** been run against
  the full commit history of every branch in this session; the `.env.example` file added in `683fc61`
  was staged and scanned locally and is clean, and it contains no credential-shaped values.
- `cargo audit` exits 0 despite the 6 warnings: informational warnings do not fail the audit, only
  vulnerabilities do. A green `cargo audit` is therefore **not** evidence that a dependency is free
  of unsound or unmaintained advisories.

---

## 8. GIT AND PR STATUS

### 8.1 Verified state of `main`

| Field | Value |
|---|---|
| Live remote head (`git ls-remote origin refs/heads/main`) | `ea230920cbe39c17e9ea55f0dec47e988632a669` |
| Subject | `Merge pull request #22 from zylvex-tech/ci/coverage-fetch-depth` |
| Committed | 2026-10-21 15:30:01 +0100 |
| PR #22 state | `MERGED` |
| PR #22 mergedAt | 2026-10-21T14:30:01Z |
| Merge commit | `ea230920cb` |
| Merge cleanliness | `git diff 5b6cc0a ea23092` is **empty** — a clean merge with no conflict resolution or content change |
| PR #22 checks | Coverage, Ubuntu CI, Windows CI, macOS CI — **all success** |

PR #22's single change was `fetch-depth: 0` on `actions/checkout` in `coverage.yml`. That one line is
what converted the coverage workflow from "fails before the summary stage" into "measures 80.63%",
because the repository-intelligence inputs need full history.

### 8.2 Open pull requests

| PR | Title | State | Assessment |
|---|---|---|---|
| #17 | `eslint-plugin-react-hooks` 5.2.0 → 7.1.1 | OPEN, checks failing | **Recommend closing.** v7 requires ESLint 10; the repo pins ESLint 9 flat config. Merging would force dropping `eslint-plugin-react-hooks` and/or the React hooks plugins from the lint setup. That is a lint-capability regression, not a security fix. |
| #13 | `actions/checkout` 4 → 7 | OPEN | Routine; safe to merge once green. |
| #14 | `rusqlite` 0.31.0 → 0.40.2 | OPEN | **Major bump** — needs a human review of the API changes before merge. |
| #15 | `axum` 0.7.9 → 0.8.9 | OPEN | **Major bump** — needs review. |
| #16 | `tracing-opentelemetry` 0.28.0 → 0.34.0 | OPEN | **Major bump** — needs review. |
| #18 | `react` + `@types/react` | OPEN | Major bump; needs review. |
| #19 | `framer-motion` 11.18.2 → 14.3.0 | OPEN | Major bump spanning two majors; needs review. |
| #20 | `jsdom` 30.1.0 → 30.1.3 | OPEN | Patch; safe. |
| #21 | `@tauri-apps/plugin-shell` 2.3.5 → 2.5.0 | OPEN | Minor; safe. |

**No PR was merged, closed, or modified in this session.** Every action above is a recommendation
for the owner. Merging PRs is a remote mutation and requires approval.

### 8.3 Local work produced in this session

| Branch | Commits | Pushed? |
|---|---|---|
| `fix/router-egress-spy-thread-local` | `69f1715` (router egress spy, test-only), `683fc61` (`.env.example` + `CONTRIBUTING.md`), `5e0e555` (Phase 2 verification report) | **3 commits ahead of `origin/main`, unpushed** |

The router change is **test-verified as of 2026-10-22** (§5.1): 32 router tests, 499 core lib tests,
and the full `--workspace --all-targets` suite all pass, and clippy is clean under `-D warnings` with
zero source lints. Pushing it therefore publishes a verified change, and CI gives it independent
three-OS confirmation.

No branch was deleted, reset, force-pushed, or rewritten. The user's separate checkout at
`C:/Projects/zylcode` (branch `competition/natlas-2026`, commit `c5c9138`) was not touched at any
point.

---

### 8.4 Push and pull request — executed 2026-10-22

The owner authorised pushing this feature branch. All six pre-push conditions were verified first:

| # | Condition | Result |
|---|---|---|
| 1 | `origin/main` still at the expected base `ea23092` | ✓ |
| 2 | Working tree clean | ✓ (`git status --porcelain` empty) |
| 3 | Remote feature branch absent (so a normal push cannot be non-fast-forward) | ✓ |
| 4 | `origin/main` is an ancestor of `HEAD` (clean fast-forward, no rebase needed) | ✓ |
| 5 | Changeset is exactly 4 files, all intended | ✓ |
| 6 | Local Rust verification green | ✓ (§5) |

A **normal push** was used (`git push -u origin fix/router-egress-spy-thread-local`). No `--force`,
no history rewrite, no branch deletion.

| Field | Value |
|---|---|
| Remote branch | `fix/router-egress-spy-thread-local` |
| Remote head | `5ac5ccfac1b46c1a460f171cde8148e1b68daf1e` — matches local `HEAD` exactly |
| `origin/main` after push | `ea230920cbe39c17e9ea55f0dec47e988632a669` — **unchanged** |
| Pull request | **#23** |
| PR URL | https://github.com/zylvex-tech/zylcode/pull/23 |
| PR title | `fix: isolate router egress spy across parallel tests` |
| Base / head | `main` ← `fix/router-egress-spy-thread-local` |
| State | **OPEN** — **not merged**, as instructed |

### 8.5 Competition checkout preserved

| Probe | Result |
|---|---|
| Remote `competition/natlas-2026` | `2fb7a817ab036c2e46c6f04406331930d34e5677` — **unchanged** |
| User's checkout `C:/Projects/zylcode` | branch `competition/natlas-2026` @ `c5c9138` |
| Its working tree | **clean** (`git status --porcelain` empty) |

Nothing in this session wrote to that directory.

### 8.6 CI and coverage results on the pull request

All four checks ran on the correct commit `5ac5ccfac1b46c1a460f171cde8148e1b68daf1e` and **all four
concluded success**:

| Check | Conclusion |
|---|---|
| `check / test / frontend (ubuntu-22.04)` | **success** |
| `check / test / frontend (windows-latest)` | **success** |
| `check / test / frontend (macos-latest)` | **success** |
| `coverage` | **success** |

Coverage run `39733034424` (`pull_request` event, sha `5ac5ccf`), step "Coverage gate (lines >= 78%)":

```
TOTAL    60570    11730    80.63%    4176    1080    74.14%    40133    7815    80.53%    0    0    -
```

| Metric | Value | vs `main` @ `ea23092` |
|---|---|---|
| Line coverage | **80.63%** | 80.63% — unchanged |
| Instrumented lines | 60,570 | 60,576 (−6) |
| Missed lines | 11,730 | 11,731 (−1) |
| Regions | 74.14% | 74.13% (+0.01) |
| Functions | 80.53% | 80.52% (+0.01) |
| Threshold | 78% | 78% |
| Margin | **+2.63 pp** | +2.63 pp |
| Tests passed | **840** | 840 |
| Tests failed | **0** | 0 |

Coverage is **preserved above the threshold** on the feature branch. The −6 instrumented lines are
the comment-only changes in `router.rs`; coverage itself did not move.

## 9. REPRODUCIBILITY

**Status: PASS.** Both deliverables were built from source reads, not from guessed variable names.

### 9.1 `.env.example` — 124 lines, 19 variables

Added in commit `683fc61`. **Every one of the 19 variables was verified to be read by the code** —
the check `grep -rl "\"$VAR\"" --include=*.rs crates apps` returned at least one file for all 19.
Nothing was invented.

| Group | Variables |
|---|---|
| Provider credentials (optional) | `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, `DEEPSEEK_API_KEY` |
| Model selection | `ZYLCODE_PRIMARY_MODEL`, `ZYLCODE_FALLBACK_MODEL` |
| Local state locations | `ZYLCODE_VECTOR_CACHE_PATH`, `ZYLCODE_SCORECARD_PATH`, `ZYLCODE_INTELLIGENCE_CACHE`, `ZYLCODE_TELEMETRY_DB_PATH`, `ZYLCODE_MCP_CONFIG`, `ZYLCODE_EVIDENCE_LOG`, `FACTORY_JOBS_DIR`, `ARTIFACTS_DIR`, `CLAIMS_PATH`, `EVIDENCE_GRAPH_PATH`, `PROOFS_PATH`, `PROJECTS_PATH` |
| Test / diagnostics | `ZYLCODE_MISSION_CRASH_AT` |
| Release-only | `CARGO_REGISTRY_TOKEN` |

Design properties that matter for a security reviewer:

- **Every assignment is an empty placeholder.** No real, partial, or credential-shaped value appears
  anywhere in the file.
- **Each entry carries an exact `file:line` read site** and the documented default, so a reader can
  confirm the variable is real without trusting the document.
- The provider credentials carry a **prominent warning that they are not test-neutral.** This is the
  direct cause of the `NETWORK_EGRESS_COUNT` defect in §3.2: setting these variables in a shell that
  also runs `cargo test` changes which code path the offline tests take. `.env.example` now says so
  at the point of use instead of leaving the reader to discover it from a flaky test.
- Two `file:line` citations were initially wrong because the `router.rs` edit shifted line numbers.
  They were corrected to `404-423`, `444-452`, and `601`, and all 19 citations were then re-verified
  programmatically. The stale-citation defect is recorded here rather than quietly fixed.

### 9.2 `CONTRIBUTING.md`

Rewritten in `683fc61`, every claim traced to a file:

| Claim | Source |
|---|---|
| Rust MSRV **1.77** | `Cargo.toml` `rust-version = "1.77"`, `edition = "2021"` |
| Node **22** (not the declared floor of 18) | `package.json` `engines.node = ">=18.0.0"`, but `apps/natlas-sdk/test/smoke.mjs` runs via `node --experimental-strip-types`, which needs **≥ 22.6**; `ci.yml:57` documents this and CI pins 22 |
| pnpm **9** | `package.json` `packageManager: "pnpm@9.12.0"` |
| Tauri v2 | workspace dependency |
| Formatting | exact `cargo fmt --all` / `cargo fmt --all -- --check` commands |
| Full test battery | `cargo test --workspace --all-targets`, the coverage command, and the offline `repo-context` check **with the full-history caveat** |
| New rule | "do not assert on process-global state from a parallel test" — the rule whose violation produced §3.2 |

The Node-22-versus-18 discrepancy is worth flagging: the declared floor in `package.json` is
**lower than what the repository actually requires**. A contributor who installs Node 18 because
`engines` says so will have the SDK smoke test fail for a reason unrelated to their change.

---

## 10. BRANCH PROTECTION

**Status: FAIL — `main` is entirely unprotected.** No setting was changed. This section is a
recommendation and requires owner approval.

### 10.1 Verified current state (read-only)

| Probe | Result |
|---|---|
| `GET /repos/zylvex-tech/zylcode/branches/main/protection` | **HTTP 404 — "Branch not protected"** |
| `GET /repos/zylvex-tech/zylcode/rulesets` | `[]` — **zero rulesets** |
| `GET /repos/zylvex-tech/zylcode/branches?per_page=100` | **21 branches, `protected: true` on 0 of them** |

Consequences that are observable in this repository's own history, not hypothetical:

- A direct push to `main` bypasses every check. The `release.yml` runs in §5.2 reached `main` without
  a green CI run attached to the commit that triggered them.
- Force-pushes are permitted, so `main` can be rewritten and with it the history that
  `fetch-depth: 0` — and therefore the coverage gate — depends on.
- Branch deletion is permitted.
- PR #22's checks all passed, but nothing *required* them to.

### 10.2 Recommended settings for `main`

Exact required status check names, taken from the check-run names on `ea23092`
(`GET /commits/ea23092/check-runs`), which is what GitHub matches against:

| Setting | Recommended value |
|---|---|
| Require a pull request before merging | **on**, with **1** approving review |
| Dismiss stale approvals on new commits | **on** |
| Require review from Code Owners | off (no `CODEOWNERS` file exists yet) |
| Require status checks to pass | **on** |
| Require branches to be up to date | **on** |
| Required checks | `coverage`<br>`check / test / frontend (ubuntu-22.04)`<br>`check / test / frontend (windows-latest)`<br>`check / test / frontend (macos-latest)` |
| Require conversation resolution | **on** |
| Require signed commits | off (no signing key is configured for this repo) |
| Require linear history | **on** — it is what makes "the merge commit is the tested commit" true |
| Require deployments to succeed | n/a (no environments configured) |
| Allow force pushes | **off** |
| Allow deletions | **off** |
| Restrict pushes that create matching refs | **on** (blocks tag creation that bypasses the `v*` pattern) |
| Enforce for administrators | **on** |

Two notes on the check names: the four names above are the *actual* check-run names, including the
spaces around the slashes, and the three matrix entries must be listed individually because GitHub
matches matrix jobs by their expanded name, not by the job id `check`. If `Require branches to be up
to date` is enabled, contributors will need to rebase before merging — which is correct, but should
be communicated before it is switched on so the first contributor to hit it is not surprised.

**No protection setting was created, modified, or deleted in this session.**

---

## 11. DATAFACTOR RE-SCORE READINESS

This section states what is **verified** and what is **not**. It deliberately contains no predicted
score.

### 11.1 Verified improvements since the 50.0 assessment

| # | Original finding | Verified current state | Evidence |
|---|---|---|---|
| 1 | Coverage gate failing before it could measure | **Resolved** — gate runs to completion and enforces 78% | §6: run 39617320736, `TOTAL … 80.63%`, step success |
| 2 | CI failing | **Resolved** — CI green on `main` | §5.2: run 39617320807, all 3 matrix jobs success |
| 3 | Frontend lint / format | **Resolved** — `pnpm lint` exit 0 (0 errors, 36 warnings); format check clean | §4 gate battery |
| 4 | `cargo fmt` / `clippy` | **Resolved on `main`** — both pass in CI | §5.2, PR #22 checks |
| 5 | Large files | **Measured and characterised** — 2 files over 1 MiB, both legitimate binary brand assets; 539,231 B of exact duplication found | §13.1 |
| 6 | Secrets | **Resolved** — scan clean, selftest passes, `.env.example` holds no values | §7, §9.1 |
| 7 | Dependency hygiene | **Improved, but with a material open item** — 0 vulnerabilities; 6 informational warnings including **2 unsound advisories on the direct dependency `lru` 0.12.5, one of whose affected APIs (`pop()`) is called in production code**; 9 Dependabot PRs triaged | §7.1, §8.2 |
| 8 | Reproducible environment | **Resolved** — `.env.example` + `CONTRIBUTING.md` from source | §9 |
| 9 | CI and branch protection | **Partially resolved** — CI yes; branch protection **still absent** | §10 |

### 11.2 What is still open

1. **Branch protection on `main` (§10).** This is the single largest remaining structural gap. It is
   an owner decision, not a code change, and no amount of local work closes it.
2. **`lru` 0.12.5 carries two unsound advisories, and one affected API is called in production**
   (§7.1). `LruCache::pop()` is invoked at `crates/zylcode-core/src/router/cache.rs:62`. Patched in
   `lru >= 0.18.2`. **Dependabot will not propose this bump** because the requirement `"0.12"`
   already permits 0.12.5. This is the highest-severity open item in this report.
3. **The `NETWORK_EGRESS_COUNT` fix is now test-verified** — see §5.3. This supersedes the earlier
   "compile-verified only" note.
4. **Nine open Dependabot PRs (§8.2),** five of which are major bumps needing human review.
5. **Large-file finding (#5) was measured, not remediated** — see §13.1. The duplication it exposed
   is real but deleting the duplicates is an owner decision (§14, item 4).
6. **`release.yml`'s historical failure mode is fixed in the file but was never observed succeeding**
   from a real `v*` tag push. Untested by construction — there is no release to cut.

### 11.3 The honest statement about the score

**The DataFactor score remains unconfirmed until the repository is re-scored.** Nothing in this
report predicts it, and no number here should be read as one. What this report *does* establish is
that every claim in §11.1 is backed by a command that ran and a result that was observed — and that
the two items in §11.2 marked "not test-verified" and "not re-measured" are exactly the kind of gap a
re-scoring would surface if it were performed on evidence rather than on assertion.

---

## 12. NEXT ACTIONS

Ordered by value per unit of risk. Items 12.1 and 12.2 are **done** and recorded in §5 and §8.3.

### 12.1 ~~Verify the router fix on a machine that can link~~ — **DONE 2026-10-22**

```
cargo test -p zylcode-core --lib router::   -> 32 passed, 0 failed
cargo test -p zylcode-core --lib            -> 499 passed, 0 failed
cargo test --workspace --all-targets        -> exit 0
cargo fmt --all -- --check                  -> exit 0
cargo clippy --workspace --all-targets -- -D warnings -> exit 0, 0 source lints
```

The four `NETWORK_EGRESS_COUNT` assertions all passed, including
`synthetic_offline_is_immune_to_ambient_api_keys`, which ran with provider credentials present in
the environment — the exact condition that used to perturb the process-global counter. No assertion
was loosened.

### 12.2 ~~Push the branch and open a PR~~ — see §8.4 for the actual outcome

### 12.3 Apply the branch-protection recommendation (§10.2)

Owner action. It is the largest remaining structural gap and it is a settings change, not a code
change. The exact check names are in §10.2 — copy them, including the spaces around the slashes.

### 12.4 Triage the Dependabot queue (§8.2)

Close #17. Merge the safe ones (#13, #20, #21) once green. Review the five major bumps (#14, #15,
#16, #18, #19) individually — none of them should be merged on a green check alone.

### 12.5 Remove the duplicated branding assets

Measured, not estimated (§13.1): `apps/zylcode-desktop/branding/lockup.png` and
`apps/zylcode-desktop/branding/wordmark.png` are **byte-identical** to their counterparts under
`apps/zylcode-desktop/public/branding/`. That is **539,231 bytes of exact duplication**. Deleting one
copy of each and updating the references is a mechanical, verifiable change — hash the survivor
before and after to prove nothing changed.

### 12.6 Explicitly **not** recommended

- **Do not raise the coverage threshold.** The margin is +2.63 pp. Raising it now buys a
  marginally better-looking number and a red build the first time a refactor lands.
- **Do not add region/function coverage gates** at 74.13% / 80.52%. Those would fail immediately.
- **Do not "fix" the flaky-test class by loosening assertions.** The whole point of §3.2 is that the
  assertion is correct and the isolation was wrong.

---

## 13. LIMITATIONS AND WHAT COULD NOT BE VERIFIED

Stated plainly so that nothing in this report is mistaken for more than it is.

### 13.1 Large files — measured this session (closes original finding #5)

| Measure | Value |
|---|---|
| Tracked files | 589 (521 text, 68 binary) |
| Total tracked size | 14,666,134 bytes (13.99 MiB) |
| Files over 1 MiB | **2** — `apps/zylcode-desktop/src-tauri/icons/icon.icns` (1,466,673 B) and `apps/zylcode-desktop/public/branding/logo-hero.png` (1,215,331 B) |
| Largest text file | `pnpm-lock.yaml` (154,130 B), then `Cargo.lock` (154,005 B) |
| Exact duplicates found | `lockup.png` and `wordmark.png`, each stored twice with identical SHA-256 prefixes (`cfbdfb83e5c57ce4`, `5357013fed24f996`) |

Both files over 1 MiB are legitimate binary assets — a macOS `.icns` icon bundle and a hero image —
not accidentally committed build output. This is **not** the same defect class as a stray
`target/` directory or a vendored dependency, and it should not be "fixed" by compressing brand
assets that ship in the installer. The real, actionable finding is the 539,231 bytes of exact
duplication in §12.5.

### 13.2 The `lru` advisories — verified open, not yet remediated

Two `unsound` advisories affect `lru` 0.12.5, a **direct** workspace dependency, and the code calls
`LruCache::pop()` at `crates/zylcode-core/src/router/cache.rs:62`. Patched in `lru >= 0.18.2`
(RUSTSEC-2026-0253) and `>= 0.16.3` (RUSTSEC-2026-0002). No remediation has been applied: bumping a
direct dependency is an owner decision requiring a full re-verification pass (§14). See §7.1 for the
full advisory detail and the per-API exposure table.

### 13.3 The `release.yml` trigger is inferred, not observed

See §5.2. The two failing runs report `head_branch=main` but only `release.yml`'s `tags:` filter
explains why `ci.yml` and `coverage.yml` did not also fire. The ref itself no longer exists, so the
mechanism is inference from trigger-filter evidence.

### 13.4 Dev-dependency advisories are out of scope

`pnpm audit --prod` excludes dev dependencies. No statement in §7 covers them.

### 13.5 The secret scan was not run against all history

It ran against the checked-out tree on CI and locally against the staged `.env.example`. A
full-history scan of every branch was not performed.

### 13.6 The DataFactor score is not measured here

Nothing in this report re-scores the repository. §11 lists what is verified; it does not convert
that into a number.

---

## 14. OWNER DECISIONS REQUIRED

Each of these is a mutation this session deliberately did not perform.

| # | Decision | Why it needs the owner |
|---|---|---|
| 1 | Apply branch protection to `main` (§10.2) | Repository settings change; affects every future contributor |
| 2 | Bump `lru` from `"0.12"` to `">= 0.18.2"` in the workspace `Cargo.toml` | Dependency-version change against a direct dependency with unsound advisories; needs a full re-verification pass after the bump |
| 3 | Merge / close the Dependabot PRs (§8.2) | Merging into `main`; five of them are major-version bumps |
| 4 | Delete the duplicated branding assets (§12.5) | Deleting tracked files; needs the references checked first |
| 5 | Whether to cut a `v*` tag to exercise `release.yml` | Creates a release; out of scope for a verification pass |

**Standing constraints honoured throughout:** no test was weakened, no threshold was lowered, no
security check was disabled or suppressed, no history was rewritten, no branch was deleted or
force-pushed, no repository setting was changed, no PR was merged or closed, and the user's separate
`competition/natlas-2026` checkout was not touched.

---

*Report compiled 2026-10-21 from commands executed in this session. Every figure quoted above is
traceable to a command whose output was observed. Where a check could not be run, it is marked
BLOCKED or NOT RUN rather than reported as passing.*
