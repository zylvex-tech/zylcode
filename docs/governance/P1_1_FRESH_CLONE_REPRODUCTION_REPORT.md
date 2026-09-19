# P1.1 — FRESH-CLONE INDEPENDENT REPRODUCTION REPORT

**Executed by:** Buffy (Codebuff), under the P1.1 master instruction.
**Date:** 2026-09-18
**Scope guard honoured:** no source code modified, no failing test fixed, no capability
rung changed, no feature work. Every number below is measured in the fresh clone; where
a prior claim could not be confirmed, it is recorded as a discrepancy, not reconciled.

---

## 1. Remote SHA and clone provenance

| Item | Value |
|---|---|
| Remote HEAD at execution (`git ls-remote origin refs/heads/main`) | `8c847154356745d4f4a6b01ee03b2d23ac3870f1` |
| Clone source | `https://github.com/zylvex-tech/zylcode.git` |
| Clone destination | `C:/Projects/zylcode-p11/zylcode` (brand-new; no prior ZylCode state on that path) |
| Clone HEAD after checkout | `8c847154356745d4f4a6b01ee03b2d23ac3870f1` — **identical** |
| Working tree after clone (`git status --porcelain \| wc -l`) | `0` |
| Pre-existing stray directories | none — `target`, `node_modules`, `.wt`, `.zylcode` all absent |

---

## 2. Environment

| Component | Version / state |
|---|---|
| OS | Windows (Git Bash) |
| rustc / cargo | 1.97.1 (8bab26f4f 2026-07-14) / 1.97.1 |
| Node | v24.21.0 — **deviation from CI's Node 20** (noted; frozen install still resolved) |
| pnpm | 9.12.0 — matches CI's pnpm 9 |
| `core.autocrlf` | `true` (ambient, unchanged) |

Ambient credential environment (presence only; values never printed):

| Variable | State | Risk note |
|---|---|---|
| `OPENROUTER_API_KEY` | SET — placeholder value | the exact hazard the router hermeticity tests target |
| `ANTHROPIC_API_KEY` | SET (51 chars, non-placeholder) | real-shaped ambient key present during tests |
| `DEEPSEEK_API_KEY` | SET (35 chars, non-placeholder) | same |
| `GITHUB_TOKEN` | SET (40 chars) | not used by the battery |

The battery therefore ran **with ambient provider credentials present** — the
hermeticity claims were tested under adversarial conditions, which strengthens them.

---

## 3. Verification battery — commands, durations, results

All raw output preserved in `C:/Projects/zylcode-p11/evidence/` (files 01–08).
Durations are wall-clock, cold start unless noted.

| # | Command | Exit | Duration | Result |
|---|---|---|---|---|
| 1 | `cargo check --workspace --all-targets` | 0 | 244.9 s | PASS (cold; 4m04s) |
| 2 | `cargo check --workspace --all-targets --all-features` | 0 | 3.5 s | PASS (warm) |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 34.4 s | PASS — zero warnings |
| 4 | `cargo test --workspace --all-targets --no-fail-fast` | 0 | 530.9 s | **417 passed / 0 failed** |
| 5 | `python scripts/check_retracted_claims.py` | 0 | 0.8 s | OK — 11 baselined / 0 new |
| 6 | `pnpm install --frozen-lockfile` | 0 | 21.8 s | PASS — lockfile resolved cleanly |
| 7 | `pnpm --filter zylcode-desktop build` | 0 | 23.8 s | PASS — dist emitted (index.js 305.81 kB) |
| 8 | `cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture` | 0 | 103.1 s (incl. compile) | **2 passed / 0 failed** |

Test count reproduced exactly: the builder claim of **417/0 is independently
reproduced** on a clean clone.

---

## 4. Scanner exclusion evidence (measured, not assumed)

Corpus contaminators actually present in the clone at benchmark time:

| Directory | Files on disk |
|---|---|
| `target/` (full cold build output) | 10,392 |
| `node_modules/` (frozen install) | 6,294 |
| `.git/` | 28 |
| **Total contaminating files** | **16,686+** |

**Trap-file experiment:** `scanner_probe_trap.rs` was planted inside both `target/`
and `node_modules/` before the benchmark run, then removed afterwards.

Benchmark actuals on the contaminated tree:

| Metric | Measured | Prior builder claim |
|---|---|---|
| Entries scanned | **330** | 344 (builder tree, extra untracked non-ignored files present) |
| Files indexed | **284** | — (285 tracked files; 1 below the indexed count is `.gitattributes`-class non-source) |
| Symbols extracted | **1,545** | 1,545 — **exact match** |
| Packages | 5 | 5 |
| Scan duration | **462.99 ms** | ~0.52 s — consistent |
| Precision@10 | 0.23 (bound ≥ 0.20) | 0.24 |
| Recall@10 | 0.43 (bound ≥ 0.30) | **0.58 — variance, see D3** |

Exclusion proof:

- Trap files: `grep -c "trap"` over the full benchmark output → **0** (never indexed).
- Any `target/` or `node_modules/` path in output → exactly 1 match, which is cargo's
  own test-runner banner (`Running tests\repo_intelligence_benchmark.rs
  (target\debug\deps\…)`), not scanner output.
- Conclusion: the scanner demonstrably excluded `target/`, `node_modules/`, `.git/`,
  and did not index the traps, despite 16,686 contaminating files on disk.
  **The Windows scanner fix is reproduced.** The historical failure mode (225 s scans
  indexing build output) is not reproducible.

---

## 5. Discrepancies found (recorded, not fixed)

**D1 — Stale self-referential SHA in the governance document (defect, confirmed).**
`docs/governance/SOURCE_OF_TRUTH_AND_BUILD_ORDER.md` line 27 states
`origin/main = fbbb4fd` and line 162 says the doc-correcting commit "lifted it to
`fbbb4fd`". Remote HEAD at reproduction is `8c84715…` — the document is stale by one
commit, exactly as the independent review identified. A document cannot reliably name
the SHA of the commit that contains it.

**Recommended formulation (non-self-referential):**

> `origin/main` is the authoritative remote source of truth. The integration tree
> first reached the remote at `6e93c43d` (Gate-0 squash); subsequent
> governance/documentation corrections followed. The current authoritative SHA is
> whatever `git ls-remote origin refs/heads/main` returns — determine it, do not
> hard-code it here.

Applied as a separate commit **after** this report, per the review's sequencing rule.

**D2 — Stale ci.yml comment (known, still present).** `.github/workflows/ci.yml:84`
still reads "the measured figure is 112 tool IDs across 16 categories" (retracted; the
guard's own rule table now measures 27 across 8). Comment-only, no behavioural effect;
remains the withheld delta pending a workflow-scoped credential decision.

**D3 — Recall@10 is non-deterministic across runs.** Builder run: 0.58; fresh clone:
0.43. Both clear the ≥ 0.30 assertion, but a metric that moves 0.15 between runs is
not a stable benchmark. Likely corpus/query-set iteration-order dependence.
Recommendation (future, not this unit): seed or sort the query set so recall is
reproducible run-to-run.

**D4 — Builder "344 files scanned" vs clone "330".** Consistent with the builder tree
carrying ~14 additional untracked, non-gitignored files at scan time; the clone scans
the pure tracked corpus (330 entries / 284 indexed of 285 tracked). The fix claim
under test — no descent into build output — is unaffected.

**D5 — CI remains not GREEN (external).** Not re-verified here beyond the prior
record: the account-level Actions billing lock (BLOCKED_EXTERNAL) still stands; no
commit can show green CI until it is resolved. Local reproduction is therefore the
only verification authority, which is precisely what this report provides.

---

## 6. Verdict

**PASS — recommend proceeding to the Phase 2A independent re-acceptance audit.**

Basis: on a brand-new clone of remote HEAD `8c84715`, with ambient provider
credentials present and `core.autocrlf=true`, the full prescribed battery passes:
compile (0), all-features compile (0), clippy `-D warnings` (0), **417/0 tests**,
claims guard (11/0), frozen frontend install + build (0/0), and the repository
intelligence benchmark (**2/0**) with demonstrable exclusion of 16,686 build/dependency
files and trap files. All five discrepancies are documentation-level or external; none
blocks the audit.

**Explicitly NOT claimed by this report:**

- No capability rung is changed. R4 for Gate-0 now has its committed reproduction
  procedure (this document) and reviewable raw evidence, but **rung promotion is the
  independent auditor's call**, not the builder's.
- Phase 2A remains RE-OPENED until the re-acceptance audit accepts it.
- R3 tool capabilities remain 0 until commissioned through a named product surface.
- CI is not GREEN and cannot be certified locally.
