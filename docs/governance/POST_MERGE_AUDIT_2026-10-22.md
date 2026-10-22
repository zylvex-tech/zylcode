# Post-merge audit — `main` @ `3bd5059` (PR #23 + PR #24)

**Audit date:** 2026-10-22
**Audited commit:** `3bd50599e70942a7bc00c50e4f83f12db8fdebce` (tip of `origin/main`)
**Auditor mandate:** read-only verification of the merge outcome of PR #23 and PR #24,
with no repository mutation performed during evidence collection.
**Base for this changeset:** `origin/main` @ `3bd5059`.

Every result below is classified **VERIFIED** (the check ran and its outcome was
observed), **FAILED** (the check ran and did not pass), **BLOCKED** (the check could
not be run for an environmental reason), or **NOT RUN** (no execution exists to
inspect). No result is reported as passing on the strength of an earlier commit.

---

## 1. Executive summary

| # | Question | Result | Classification |
|---|---|---|---|
| 1 | Are PR #23 and PR #24 merged into `main`? | Yes — `main` @ `3bd5059` | **VERIFIED** |
| 2 | Is the router egress-spy fix present in `main`'s tree? | Yes | **VERIFIED** |
| 3 | Is the `lru`/Tokio dependency fix present in `main`'s tree? | Yes | **VERIFIED** |
| 4 | Did CI pass on the **final `main` commit**? | Yes — CI run #120 and Coverage run #16, both `success` | **VERIFIED** |
| 5 | Did any check fail anywhere in the merge chain? | Yes — CI run #118 on the intermediate merge commit `184e178`, macOS job | **VERIFIED (failure, since superseded)** |
| 6 | Are the `lru` unsound advisories resolved? | Yes — `lru` 0.18.5, both advisories absent from the audit | **VERIFIED** |
| 7 | Is the Tokio yanked-release issue resolved? | Yes — Tokio 1.54.1, no `yanked` warnings | **VERIFIED** |
| 8 | Are any *vulnerable* advisories outstanding? | No — 0 vulnerabilities, 3 informational warnings | **VERIFIED** |
| 9 | Is `main` protected by branch protection or a ruleset? | **No** | **FAILED** |
| 10 | Is the repository ready for a DataFactor re-submission? | Substantially, with two governance gaps (§6) | **VERIFIED (conditional)** |

**The one-sentence answer:** both fixes are merged, CI is green on the resulting
`main` commit with 840 passing tests on each of three operating systems and zero
Clippy lints, the dependency exposure that blocked the earlier review is closed, and
the remaining risks are governance-shaped rather than code-shaped — `main` is
unprotected, and one flaky macOS test was observed on an intermediate commit before
the fix landed.


---

## 2. Verified Git and merge state

### 2.1 `main` history

```
3bd5059  Merge pull request #23 from zylvex-tech/fix/router-egress-spy-thread-local
184e178  Merge pull request #24 from zylvex-tech/fix/lru-0-12-unsound-advisories
ea23092  Merge pull request #22 from zylvex-tech/ci/coverage-fetch-depth
```

`3bd5059` is a two-parent merge; its first parent is `184e178`, its second is
`1aaca56`. `184e178` is itself a merge of `f0c729b` onto `ea23092`.

### 2.2 Pull requests

| PR | Title | State | Merged at (UTC) | Head SHA | Merge commit |
|---|---|---|---|---|---|
| #23 | `fix: isolate router egress spy across parallel tests` | MERGED | 2026-10-22T11:32:49Z | `1aaca56fdba9c6168fcce8db3af72c19fc71c380` | `3bd50599e70942a7bc00c50e4f83f12db8fdebce` |
| #24 | `fix(deps): move lru off 0.12.5 (two unsound advisories) and tokio off a yanked release` | MERGED | 2026-10-22T11:18:19Z | `f0c729b0477c005ce7bfb37a202565d06dcd58f7` | `184e178c0d33a8241b6a21b5529d10e6625733d5` |

Both were merged with the **merge-commit** strategy (not squash, not rebase), so the
original head SHAs remain addressable in history — which is what makes the per-commit
CI comparison in §3 possible.

### 2.3 Both changes are present in `main`'s tree — VERIFIED

| Change | Evidence in `main`'s tree | Classification |
|---|---|---|
| `lru` requirement | `Cargo.toml:67` → `lru = "0.18"` | **VERIFIED** |
| `lru` resolution | `Cargo.lock` → `lru` **0.18.5** (blob `8a02bffb…`) | **VERIFIED** |
| Tokio resolution | `Cargo.lock` → `tokio` **1.54.1** | **VERIFIED** |
| `tokio` requirement | `Cargo.toml:37` unchanged at `version = "1.38"` | **VERIFIED** |
| Router egress spy | `thread_local!` with `const { Cell::new(0) }` in `crates/zylcode-core/src/router.rs` | **VERIFIED** |
| Egress increment site | 1 increment site, gated by `#[cfg(test)]` | **VERIFIED** |
| Egress assertions | 4 assertion sites, all intact | **VERIFIED** |
| `.env.example` | present, 5,200 bytes, 19 variables, all empty placeholders | **VERIFIED** |

`main`'s `Cargo.lock` and `Cargo.toml` blobs (`8a02bffb…`, `6acc2db…`) are
**byte-identical** to those of `f0c729b`, confirming PR #24's content reached `main`
unaltered. Dependency count fell from 598 to **595** packages: `allocator-api2`
0.2.21, `foldhash` 0.1.5 and `hashbrown` 0.15.5 dropped out of the graph.

### 2.4 Untouched by this changeset

| Item | State | Classification |
|---|---|---|
| `competition/natlas-2026` | unchanged at `2fb7a817ab036c2e46c6f04406331930d34e5677` | **VERIFIED** |
| Both feature branches | still present on the remote, not deleted | **VERIFIED** |
| Force-pushes / history rewrites | none performed | **VERIFIED** |


---

## 3. CI evidence on the final `main` commit

The mandate required distinguishing checks that passed **on the final `main`
commit** from checks that passed only on **earlier PR commits**. Those are different
claims and they are reported separately below.

### 3.1 Workflow runs triggered by the `3bd5059` push

| Workflow | Run | Event | Conclusion | Created (UTC) | Finished (UTC) |
|---|---|---|---|---|---|
| CI | #120 (`39771400143`) | `push` | **success** | 11:32:53 | 12:00:13 |
| Coverage | #16 (`39771400207`) | `push` | **success** | 11:32:53 | 11:38:49 |

No `release` workflow ran: `ci.yml` and `coverage.yml` both filter on `branches:`
only, and the historical `release.yml` filters on `tags:` only, so a branch push
correctly triggers exactly these two.

### 3.2 Check runs attached to `3bd5059` — VERIFIED

| Check run | Conclusion | Completed (UTC) |
|---|---|---|
| `check / test / frontend (ubuntu-22.04)` | **success** | 12:00:12 |
| `check / test / frontend (windows-latest)` | **success** | 11:41:21 |
| `check / test / frontend (macos-latest)` | **success** | 11:39:21 |
| `coverage` | **success** | 11:38:49 |

**Honest timing note.** When the check-run set was first queried at 11:55:28Z, the
`ubuntu-22.04` job was still `in_progress` with no conclusion. It was **not** reported
as passing at that point. It was re-queried until it completed at 12:00:12Z with
`success`, and only then is it recorded above. The final state of run #120 is
`completed / success`.

### 3.3 Step-level evidence from the `ubuntu-22.04` job of run #120

The Linux job is the strictest of the three: `cargo clippy -- -D warnings` is
`continue-on-error` on Windows and macOS but a **hard gate** on Linux.

| Step | Observed output | Classification |
|---|---|---|
| `cargo check --workspace --all-targets` | `Finished dev profile` | **VERIFIED** |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished dev profile` — **0 lints** | **VERIFIED** |
| `cargo fmt --all -- --check` | no diff emitted | **VERIFIED** |
| `cargo test --workspace --all-targets` | **840 passed, 0 failed** across 30 test binaries | **VERIFIED** |
| `check_retracted_claims.py` | `OK: no NEW retracted claims (11 pre-existing baselined occurrence(s), 11 baseline line(s))` | **VERIFIED** |
| `cargo audit` | `3 allowed warnings found`, 0 vulnerabilities, exit 0 | **VERIFIED** |
| `pnpm audit --prod` | `No known vulnerabilities found` | **VERIFIED** |
| `check_hardcoded_secrets.py` | `secret scan: clean -- no credential-shaped literal in tracked files.` | **VERIFIED** |
| `pnpm --filter zylcode-desktop lint` | warnings only (36 accepted, itemised in `ACCEPTED_LINT_WARNINGS.md`); 0 errors | **VERIFIED** |
| `pnpm --filter zylcode-desktop format:check` | `Checking formatting...` — no violations | **VERIFIED** |
| `pnpm --filter @zylvex/natlas-bridge typecheck` | exit 0 | **VERIFIED** |
| `node --experimental-strip-types apps/natlas-sdk/test/smoke.mjs` | 16 assertions, all `ok` | **VERIFIED** |
| `pnpm --filter zylcode-desktop build` | `✓ 470 modules transformed.` / `✓ built in 2.76s` | **VERIFIED** |

Test counts are **identical on all three operating systems**: 840 passed, 0 failed on
ubuntu-22.04, windows-latest and macos-latest. `zylcode-core`'s lib binary alone
reports **497 passed; 0 failed**.

### 3.4 The coverage gate — with a correction to the earlier report

The gate is `cargo llvm-cov --workspace --summary-only --fail-under-lines 78`.
The `TOTAL` row of run #16 on `3bd5059` reads:

| Metric | Total | Missed | Cover |
|---|---|---|---|
| **Lines** (the gated metric) | 40,133 | 7,815 | **80.53%** |
| Regions | 60,570 | 11,730 | 80.63% |
| Functions | 4,176 | 1,080 | 74.14% |

**Correction.** `PHASE2_VERIFICATION_REPORT.md` §6 records "80.63% lines
(60,576 / 11,731)". That is **mislabelled**: 80.63% is the **Regions** figure. The
**Lines** figure on `main`'s head is **80.53%** (40,133 / 7,815), confirmed against
the column header of the `llvm-cov` summary table. Both numbers clear the 78% floor —
the gated metric clears it by **2.53 percentage points** — but the label was wrong and
is corrected here rather than propagated.

The lowest-covered files are `crates/zylcode-nav/src/model.rs` (38.98% lines),
`crates/zylcode-mcp/src/lib.rs` (63.46%) and `crates/zylcode-mcp/src/registry.rs`
(61.96%).

### 3.5 A check that failed on an intermediate commit — and what it proves

CI run **#118** (`39769570784`), triggered by the push of the **intermediate** merge
commit `184e178`, **failed**:

| Check run on `184e178` | Conclusion |
|---|---|
| `check / test / frontend (macos-latest)` | **failure** |
| `check / test / frontend (ubuntu-22.04)` | success |
| `check / test / frontend (windows-latest)` | success |
| `coverage` | success |

The failing step was `Cargo test (workspace)`. The failing test was:

```
thread 'router::tests::d1_vector_cache_cross_prompt_contamination_guard' panicked at
crates/zylcode-core/src/router.rs:1872:9:
assertion `left == right` failed: the D1 guard test must not perform network egress
  left: 8
 right: 5
test result: FAILED. 496 passed; 1 failed
```

This is the **exact defect PR #23 was written to fix**: a shared process-wide egress
counter incremented by unrelated parallel tests. The counter read 8 instead of 5
because three other tests on other threads had incremented the same global.

**Two facts make this finding load-bearing rather than alarming:**

1. `184e178` and `f0c729b` have **byte-identical trees** (`7cabc439…`). `f0c729b`'s
   macOS job passed; `184e178`'s failed. Same code, different outcome — the failure is
   **flakiness**, not a regression introduced by the merge.
2. On `3bd5059`, which contains PR #23's `thread_local!` fix, the same test
   `d1_vector_cache_cross_prompt_contamination_guard ... ok` on **both** macOS and
   Linux, and the whole suite is green on all three OSes.

So PR #23 is not merely theoretically justified — it closed a failure that was
**observed in CI on this repository's own history**.


---

## 4. Dependency audit results

`cargo audit` 0.22.2 was run against `main`'s tree (`3bd5059`, 595 crate
dependencies, RustSec database at 1,349 advisories). Because the working checkout of
the audit branch is `3bd5059` itself, the lockfile audited is `main`'s lockfile.

### 4.1 Headline result — VERIFIED

```
vulnerabilities: 0
warnings:       3   (2 unmaintained, 1 unsound)
exit code:      0
```

The CI's own `cargo audit` step in run #120 independently reproduced the identical
result — `Crate: proc-macro-error / unsafe-libyaml / glib`, `warning: 3 allowed
warnings found`, 595 crate dependencies. Two independent executions, same answer.

### 4.2 The three advisories this audit was asked to confirm resolved

| Item | Status | Evidence |
|---|---|---|
| **RUSTSEC-2026-0002** (`lru` `IterMut` unsound) | **RESOLVED** | `lru` resolves to 0.18.5; advisory absent from the audit output |
| **RUSTSEC-2026-0253** (`lru` `pop()` panic-safety unsound) | **RESOLVED** | `lru` resolves to 0.18.5; advisory absent from the audit output |
| **Tokio yanked release** | **RESOLVED** | `tokio` resolves to 1.54.1; the audit emits **no `yanked` category at all** |

All three are **VERIFIED resolved**.

A note on the `lru` upgrade's blast radius: `crates/zylcode-core/src/router/cache.rs`
is the only `lru` consumer in the workspace, and it uses `LruCache::new`, `get`,
`pop`, `put`, `len` and `clear` — every one of which is unchanged between `lru` 0.12
and 0.18. The upgrade is therefore API-compatible by construction, which the green
build and 840 tests confirm empirically.

### 4.3 The three remaining advisories, in full

#### RUSTSEC-2024-0429 — `glib` 0.18.5 — **unsound**

| Field | Value |
|---|---|
| Advisory | RUSTSEC-2024-0429 (alias GHSA-wrw7-89jp-8q8g) |
| Affected dependency | `glib` 0.18.5 |
| Class | unsound |
| Affected functions | `glib::VariantStrIter::{next, next_back, nth, nth_back, last}` — `>=0.15.0, <0.20.0` |
| Fixed version | **`>= 0.20.0`** (latest published is **0.22.10**) |
| Dependency path | `zylcode-desktop` → `tauri` 2.12.4 → `gtk` 0.18.2 / `webkit2gtk` / `wry` / `tao` → `glib` 0.18.5 |
| Platform | **Linux only.** Not reachable on Windows or macOS — `cargo tree -i glib` on the current platform prints "nothing to print". It *is* compiled on the ubuntu-22.04 CI job, which installs `libgtk-3-dev` and `libwebkit2gtk-4.1-dev`. |

**Root cause (from the advisory):** `VariantStrIter::impl_get` passed `&p` — an
immutable reference to a `*mut c_char` initialised to `NULL` — as an out-argument to
the variadic C function `glib_sys::g_variant_get_child`. The variadic signature is
what stopped the compiler rejecting the mutability mismatch. Recent rustc versions
discard the unsound writes, so `CStr::from_ptr` then received `NULL` and crashed.

**Practical remediation:** `glib` cannot be bumped in isolation — it is a transitive
dependency of the whole GTK binding stack, and `gtk` 0.18.2 requires `glib` 0.18.x.
The only real lever is bumping `tauri` (currently 2.12.4; latest stable is **2.12.5**,
and 3.0.0-alpha.5 exists). **Whether 2.12.5 vendors `glib >= 0.20` has not been
verified** and must be checked before the bump is attempted.

**Risk today: low.** Re-verified on `main`'s tree: `grep -rn "VariantStrIter"
crates/ apps/` returns **0 matches**. The unsound code path is not reachable from
ZylCode's own source. The exposure is confined to whatever Tauri itself does with
`VariantStrIter`, which ZylCode does not call directly.

#### RUSTSEC-2024-0449 — `unsafe-libyaml` 0.2.11 — **unmaintained**

| Field | Value |
|---|---|
| Advisory | RUSTSEC-2024-0449 |
| Affected dependency | `unsafe-libyaml` 0.2.11 |
| Class | unmaintained |
| Fixed version | **none exists** — the repository was archived read-only on 2024-03-25 and 0.2.11 is the final release |
| Dependency path | `zylcode-core` **and** `zylcode-mcp` → `serde_yaml` 0.9.34+deprecated → `unsafe-libyaml` 0.2.11 |

`unsafe-libyaml` is a c2rust machine translation of libyaml's C code into unsafe
Rust. Bugs found in it will not be fixed upstream. `serde_yaml` itself is deprecated
(the `+deprecated` suffix is in the lockfile).

**Practical remediation — this one is actionable now.** `serde_yaml` is used at
exactly four call sites, all through the same narrow API:

- `crates/zylcode-core/src/intelligence/manifest.rs:118-119` — `serde_yaml::from_str` on `pnpm-workspace.yaml`
- `crates/zylcode-mcp/src/config.rs:76` — `serde_yaml::from_str` on MCP config
- `crates/zylcode-mcp/src/tool_catalogue.rs:949, 986` — `serde_yaml::from_str` (tests)

**[`serde_norway`](https://crates.io/crates/serde_norway) 0.9.42** is a maintained
fork that preserves the `serde_yaml` 0.9 API and version scheme, so the migration is a
dependency swap plus four import renames — no call-site logic changes. That removes
`unsafe-libyaml` from the graph entirely. Alternatives: `serde_yaml_ng` 0.10.0 (fork,
different major) or `saphyr` 0.2.2 (most recently updated, 2026-10-15, but a different
API and more work).

**Risk today: low.** Reached only when parsing local YAML configuration files that
the user already controls.

#### RUSTSEC-2024-0370 — `proc-macro-error` 1.0.4 — **unmaintained**

| Field | Value |
|---|---|
| Advisory | RUSTSEC-2024-0370 |
| Affected dependency | `proc-macro-error` 1.0.4 |
| Class | unmaintained |
| Fixed version | **none exists** — the maintainer is unreachable, no release in 4 years |
| Dependency path | `zylcode-desktop` → `tauri` 2.12.4 → `glib` 0.18.5 → `glib-macros` 0.18.5 (proc-macro) → `proc-macro-error` 1.0.4 |

**Practical remediation:** no direct action available. It rides on the same
`tauri` bump as `glib`, and even then only if a future `glib-macros` drops it. The
advisory's own suggested alternatives (`manyhow`, `proc-macro2-diagnostics`) are
upstream changes in `glib-macros`, not something this workspace can make.

**Risk today: lowest of the three.** It is a **build-time proc-macro**: it executes
only while compiling `glib-macros`, is never linked into the shipped binary, and never
runs at runtime. Its practical impact is a future maintenance burden, not a defect.

### 4.4 Why `cargo audit` exits 0 here, and why that is correct

`cargo audit` exits non-zero only on a **vulnerable** advisory. `unmaintained` and
`unsound` advisories are informational and exit 0 — which is why `ci.yml` runs plain
`cargo audit` and not `cargo audit --deny warnings`. The three remaining advisories
are documented, reachability-checked and consciously accepted in
`docs/governance/ACCEPTED_ADVISORIES.md`. That file has been **updated in this
changeset**: it previously listed five advisories including the two now-resolved
`lru` ones.

### 4.5 Documentation drift found and corrected in this changeset

Two places still asserted the pre-remediation state:

1. `docs/governance/ACCEPTED_ADVISORIES.md` listed **five** advisories, including
   RUSTSEC-2026-0002 and RUSTSEC-2026-0253 on `lru` 0.12.5. Both rows are now removed.
2. `.github/workflows/ci.yml` carried the comment *"the workspace has 5 accepted
   advisories … RUSTSEC-2026-0002 and -2026-0253 (lru)"*. Corrected to three.

One further inaccuracy in `ACCEPTED_ADVISORIES.md` is worth recording, because it was
**false when written**: the file claimed `grep -rn "IterMut\|\.pop()" crates/`
returned no matches, and that `pop()` "is not on a path ZylCode calls". In fact
`crates/zylcode-core/src/router/cache.rs:61` calls `guard.pop(&key)` on the
`LruCache` — the RUSTSEC-2026-0253 code path **was** live. The row is now moot (the
dependency is upgraded), but the claim itself was wrong and is corrected here so it
does not survive as a precedent for how reachability is asserted.


---

## 5. Branch-protection assessment — **FAILED**

| Check | Result | Classification |
|---|---|---|
| `GET /repos/{owner}/{repo}/branches/main/protection` | **HTTP 404 — "Branch not protected"** | **FAILED** |
| `GET /repos/{owner}/{repo}/rulesets` | `[]` — no rulesets exist | **FAILED** |
| `GET /repos/{owner}/{repo}/rules/branches` | **HTTP 404 — Not Found** | **FAILED** |

`main` has **no branch protection and no ruleset of any kind**. Concretely, this
means nothing in the repository's configuration prevented either merge from landing
with failing, pending, or entirely absent CI.

This is not hypothetical. §3.5 records that CI run #118 — the run triggered by the
push of the merge commit `184e178` — **failed on macOS**. Because `main` is
unprotected, that red build sat on `main`'s first-parent history for roughly fifteen
minutes until PR #23 was merged on top of it and the next CI run went green. Nothing
forced anyone to notice.

### 5.1 Recommended configuration

The check names below are taken from the actual check runs observed in §3.2, not
invented:

| Setting | Recommended value | Rationale |
|---|---|---|
| Require a pull request before merging | **on**, 1 approval | Direct pushes to `main` are currently possible |
| Require status checks to pass | **on** | The single highest-value control |
| Required checks | `check / test / frontend (ubuntu-22.04)`, `check / test / frontend (windows-latest)`, `check / test / frontend (macos-latest)`, `coverage` | These four are the complete set of check runs that actually exist on a push to `main` |
| Require branches to be up to date | **on** | Prevents merging on a stale base, which is how the `184e178` red build would have been missed |
| Require conversation resolution | **on** | Review findings should not be silently ignorable |
| Do not allow bypassing the above | **on** | Otherwise the rules are advisory |
| Allow force pushes | **off** | Already the practice; make it structural |
| Allow deletions | **off** | `main` should be undeletable |

**Note on the flaky macOS test.** If `check / test / frontend (macos-latest)` is made
a required check while that test remains flaky, required checks will intermittently
block legitimate merges — and the standard bad response to a flaky required check is
to disable the check. The ordering in §7 therefore puts the flakiness fix **before**
enabling required checks, so the gate is trustworthy on the day it becomes
mandatory.

---

## 6. DataFactor re-submission checklist

No score is claimed, predicted or implied here. This is a readiness list only.

| # | Item | Status | Classification |
|---|---|---|---|
| 1 | Known dependency-security blocker closed | `lru` 0.18.5, Tokio 1.54.1, 0 vulnerabilities | **VERIFIED** |
| 2 | Full CI green on the final `main` commit | 840 tests × 3 OSes, 0 Clippy lints, all 13 steps pass | **VERIFIED** |
| 3 | Coverage gate met | 80.53% lines vs 78% floor | **VERIFIED** |
| 4 | Secret scanning clean | `secret scan: clean` | **VERIFIED** |
| 5 | Production JS audit clean | `No known vulnerabilities found` | **VERIFIED** |
| 6 | Test-isolation defect demonstrably fixed | Same test failed pre-fix, passes post-fix | **VERIFIED** |
| 7 | Governance docs consistent with reality | `ACCEPTED_ADVISORIES.md` and `ci.yml` comment corrected in this changeset | **VERIFIED** |
| 8 | `main` protected | **Not protected — no ruleset** | **FAILED** |
| 9 | macOS test flakiness addressed | Observed once on `184e178`; root cause understood and fixed, but no re-run evidence of stability across repeated runs | **PARTIAL** |
| 10 | `PHASE2_VERIFICATION_REPORT.md` §6 mislabel corrected | Corrected in this report (§3.4); the original file is left as the historical record | **VERIFIED (correction recorded)** |

**Readiness judgement:** items 1–7 and 10 are satisfied. Item 8 is a configuration
change the owner must make in GitHub settings — it cannot be made from a commit. Item
9 is the one open engineering question. A re-submission made today would rest on
solid engineering evidence and an unguarded `main`.

---

## 7. Prioritized outstanding work

### P0 — before the next merge

1. **Enable branch protection on `main`** (§5.1). Owner action in GitHub settings;
   cannot be done from a commit. Until this exists, every other control in this
   repository is a convention rather than a guarantee.

### P1 — high value, low risk

2. **Migrate `serde_yaml` → `serde_norway` 0.9.42.** Four call sites, all
   `serde_yaml::from_str`. Removes RUSTSEC-2024-0449 / `unsafe-libyaml` from the graph
   entirely and drops the warning count from 3 to 2. This is the only remaining
   advisory with a clean, self-contained fix.
3. **Probe the `tauri` 2.12.4 → 2.12.5 bump for `glib >= 0.20`.** Check whether
   2.12.5's dependency tree vendors `glib` 0.20+. If it does, the bump closes
   RUSTSEC-2024-0429 as well and takes the warning count to 1. If it does not, record
   that finding in `ACCEPTED_ADVISORIES.md` so the next person does not repeat the
   probe.

### P2 — worth doing, not blocking

4. **Re-run the macOS suite several times to quantify residual flakiness.** PR #23
   removed the known cause, but one green run is not proof of stability. A loop of
   ~10 runs would establish a baseline.
5. **Raise the coverage floor from 78% toward 80.5%.** The measured figure is 80.53%
   lines, so the floor has 2.53 points of headroom. Tightening it to 80% converts
   slack into a regression guard at no cost.
6. **Address the three lowest-covered files** — `zylcode-nav/src/model.rs` (38.98%),
   `zylcode-mcp/src/lib.rs` (63.46%), `zylcode-mcp/src/registry.rs` (61.96%).

### P3 — backlog

7. **Dependabot triage.** 15 Dependabot PRs are open. Verified per head SHA on
   2026-10-22: **5 carry a failing check** — #7 (`pnpm/action-setup` 3→6), #8
   (`actions/setup-node` 4→7, mixed), #12 (`toml` 0.8.2→1.1.8), #14 (`rusqlite`
   0.31.0→0.40.2), #17 (`eslint-plugin-react-hooks` 5.2.0→7.1.1) — and **10 are
   green**. Of those 10, **8 are major or semver-0.x-breaking bumps** (#9
   `tauri-action` 0→1, #13 `actions/checkout` 4→7, #18 `react`, #19
   `framer-motion` 11→14, plus #10, #11, #15, #16 which cross a 0.x minor).
   Green CI is not by itself evidence of behavioural equivalence for a breaking
   bump; each needs an individual decision.
8. **Repository hygiene.** 539,231 bytes of exact duplication between
   `apps/zylcode-desktop/branding/` and `public/branding/` (byte-identical
   `lockup.png` and `wordmark.png`). No file exceeds 1 MiB other than those two
   legitimate binary brand assets.

---

## 8. Method and limitations

- All GitHub evidence was gathered through the REST API queried **per SHA**
  (`/commits/{sha}/check-runs`, `/actions/runs?head_sha=…`), never from a PR's
  aggregate status, so a check is attributed to the exact commit that produced it.
- `cargo audit` was executed against `main`'s own lockfile on a checkout of
  `3bd5059`. A detached worktree was used for evidence collection and has been
  removed; no branch was switched, reset or rebased, and no repository file was
  modified during the audit itself.
- The `ubuntu-22.04` job of run #120 was pending at first observation and was
  re-queried until it concluded. Its `success` conclusion is reported only because
  that conclusion was actually observed.
- The `glib` reachability claim (`VariantStrIter`, 0 matches) and the `IterMut`
  claim (0 matches) were re-run against `main`'s tree rather than carried forward
  from an earlier report.
- **Not verified:** whether `tauri` 2.12.5 vendors `glib >= 0.20`; whether the macOS
  suite is stable across repeated runs; the *contents* of any of the 15 open
  Dependabot PRs — their per-head-SHA check status was queried (§7 item 7), but no
  PR was reviewed for behavioural impact.
- **No score is asserted** for DataFactor, and none should be inferred from this
  document.
