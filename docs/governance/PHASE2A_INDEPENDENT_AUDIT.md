# Phase 2A Independent Audit

**Subject:** commit `0ecea8e7ef1ca0cc72608e5041a37f5b8e2133c4` — "feat(intelligence): add repository intelligence foundation"
**Auditor:** Independent verification (does not build Phase 2A, did not author it)
**Audit date:** 2026-09-16
**Repository:** `https://github.com/zylvex-tech/zylcode`
**Branch audited:** `main` @ `0ecea8e` (confirmed identical to `origin/main`)

---

## 0. Why this audit exists

The governing rule of the ZylCode program is:

> **Builders do not certify their own work.**

Phase 2A introduced the Intelligence Graph — the foundational repository model that every
later phase (2B reasoning, 3A Project System, 3B Missions, 14 multi-agent) depends on. If
the foundation's reported metrics are wrong, every downstream phase inherits the error.

The Phase 2A completion report asserts specific numbers. Those numbers were treated as
claims, not facts, until reproduced here.

---

## 1. Verdict

| | |
|---|---|
| **Phase 2A status** | **NOT ACCEPTED** |
| **Recommendation** | Do not begin Phase 2B. Do not write Architecture v2 as if 2A is sound. |
| **Reason** | The headline metric is an artifact of a defect, the benchmark **fails** on independent re-execution, and the subsystem has **zero product integration**. |

The code is real, substantial, and mostly well-structured. The *evidence* is not.

---

## 2. Claim-by-claim verification

| # | Claim in `PHASE2A_COMPLETION_REPORT.md` | Verdict | Evidence |
|---|---|---|---|
| 1 | Commit exists and is pushed | ✅ **TRUE** | `git ls-remote origin` → `refs/heads/main = 0ecea8e` |
| 2 | 19 files changed, ~6,068 insertions | ✅ **TRUE** | `git show --stat 0ecea8e` |
| 3 | 13 intelligence modules implemented | ✅ **TRUE** | `crates/zylcode-core/src/intelligence/` contains all 13 + `mod.rs` |
| 4 | 79 intelligence unit tests, all pass | ✅ **TRUE** | `cargo test -p zylcode-core --lib intelligence` → `79 passed; 0 failed` |
| 5 | 1,444 symbols extracted | ✅ **TRUE / reproducible** | Benchmark run reproduces `Symbols extracted: 1444` exactly |
| 6 | 5 packages, 4 dependency edges, 15 entry points | ✅ **TRUE / reproducible** | Benchmark run reproduces all three exactly |
| 7 | **2 benchmark tests, all pass** | ❌ **FALSE** | `benchmark_indexing_performance` **PANICS**: `Scan should complete in < 120s, took 276.1s` |
| 8 | **"Files indexed: 14,973"** presented as repository intelligence | ❌ **MISLEADING** | ≥98% of indexed files are `target/` build artifacts. Real repo = **254 files**. |
| 9 | **"Scan duration: 12.7s"** | ❌ **NOT REPRODUCIBLE** | Measured **276.1s** — 22× slower. Assertion threshold breached. |
| 10 | "12 new capabilities (11 GREEN)" | ❌ **OVERSTATED** | Module is **not reachable** from AgentLoop, CLI, or desktop app. |
| 11 | "337 workspace tests all pass" | ❌ **FALSE as stated** | `cargo test --workspace` → `zylcode-core` lib: **225 passed; 1 FAILED**. See §6.5 |
| 12 | "Acceptance Demonstration" section | ❌ **NOT EVIDENCE** | Headed "Expected Output". No command, no captured output, no artifact. |
| 13 | "REMOTE_CI = BLOCKED_EXTERNAL" | ✅ TRUE (and decisive) | No CI ever executed this benchmark. No external verification exists. |
| 14 | Report self-consistency | ❌ **BROKEN** | Committed report says `Commit SHA: (Pending)` and `Push Verification: (Pending)` — i.e. it was committed *before* the facts it certifies were known. |

---

## 3. Finding A — The headline metric measures build artifacts, not the repository

### What was claimed

> `Files indexed: 14,973` … `Scanner indexes 14,973 files in 12.7s`
> — `PHASE2A_COMPLETION_REPORT.md`, and `docs/capability-registry.json` (`repository_scanner`)

This number was propagated into the **capability registry as evidence of a GREEN capability**.

### What is actually true

Measured on the same working tree:

```
$ find target -type f -size -512k | wc -l
14900

$ find . -path ./.git -prune -o -name node_modules -prune -o \
       -name target -prune -o -type f -print | wc -l
254
```

Independent benchmark run:

```
Files scanned: 18972
Files indexed: 15064
```

**14,900 of the 15,064 indexed files live under `target/`.** The genuine repository is
**254 files**. The reported "15K file repository intelligence index" is, in substance, a
census of compiled Rust build output.

### Root cause — a Windows path-separator defect

`crates/zylcode-core/src/intelligence/classifier.rs`, `should_exclude()`:

```rust
let excluded = ["/target/", "/node_modules/", "/dist/", "/build/", "/.git/", ...];
for exc in &excluded {
    if path_str.contains(exc) { return true; }   // ← forward slashes
}
```

`path_str` is `path.to_string_lossy()`. On Windows — ZylCode's **primary target platform** —
this yields `C:\Projects\zylcode\target\debug\...`. None of the forward-slash patterns can
ever match. The exclusion list is dead code on Windows.

The `.gitignore` fallback does not rescue it either. The real `.gitignore` contains:

```
/target
**/target
```

`matches_gitignore_pattern()` in `scanner.rs` strips a trailing `/` and then performs
`starts_with` / `ends_with` / filename comparison. Neither `/target` nor `**/target` matches
`target/debug/app.exe`. (`node_modules` *is* excluded — because that entry is a bare
filename, which is the one form the naive matcher handles. This is why the leak is partial
and was easy to miss.)

### Why the tests were green anyway

`scanner.rs::scan_excludes_target` writes its **own fixture `.gitignore` containing `target/`**
— a bare directory pattern, which the naive matcher *does* handle. The test therefore passes
while real-world behavior is broken.

> **This is the core systemic risk.** The test fixture encoded a different pattern from the
> real repository, so a green suite certified a broken scanner. Unit-test green is not
> evidence of correct behavior. Only reproduction against the real repository is.

### Consequence

The benchmark's low retrieval scores are not a fair measure of the retriever — they are
polluted. `Precision@10 = 0.30` was computed over an index in which ~99% of candidates are
build artifacts. **Phase 2B cannot be planned on top of this number**, in either direction.

---

## 4. Finding B — The benchmark suite fails on independent re-execution

```
$ cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture

test benchmark_indexing_performance ...
=== Indexing Performance ===
Files scanned: 18972
Files indexed: 15064
Symbols extracted: 1444
Scan duration: 276.1043943s
Total duration: 286.8497397s

thread 'benchmark_indexing_performance' panicked at
  crates\zylcode-core\src\..\tests\repo_intelligence_benchmark.rs:284:5:
Scan should complete in < 120s, took 276.1043943s

test result: FAILED. 1 passed; 1 failed
```

The report states *"2 benchmark tests (all pass)"* and *"Benchmark tests | 2 | ✅ ALL PASS"*.

Both statements are false under independent execution. Because CI is `BLOCKED_EXTERNAL`, no
external run has ever contradicted them — which is precisely why self-certification is not
acceptable evidence.

### Secondary observation — the thresholds are far weaker than they read

The benchmark asserts only:

```rust
assert!(query.file_count() > 50);
assert!(query.symbol_count() > 20);
assert!(query.package_count() >= 3);
assert!(avg_precision >= 0.25);
assert!(avg_recall    >= 0.30);
```

None of the headline numbers (14,973 / 1,444 / 12.7s) are asserted. A scan indexing **51
files** would satisfy `file_count() > 50` and print "PASS". The acceptance bar does not
measure the capability it claims to certify.

Additionally, `Precision@10` is computed as `relevant / k` with `k = 10` fixed, even though
every query returns exactly 20 results — so precision is structurally penalized and the
metric is not a true precision.

Three of ten questions score **0.00** (Q5 test-file discovery, Q6 reverse dependency,
Q10 build-command discovery). The report acknowledges these as "benchmark gaps" but still
records `context_retrieval` and `repository_benchmark` as **GREEN**.

### Non-determinism

Reported vs. observed averages:

| Metric | Report | Independent run |
|---|---|---|
| Average Precision@10 | 0.30 | 0.31 |
| Average Recall@10 | 0.53 | 0.58 |
| Q6 | P=0.00 R=0.00 | P=0.10 R=0.50 |

The results are not bit-reproducible. A benchmark that cannot be reproduced cannot serve as
a gate.

---

## 5. Finding C — Zero product integration

```
$ grep -rn "intelligence::" --include=*.rs crates/ apps/ \
    | grep -v "src/intelligence/" \
    | grep -v "tests/repo_intelligence_benchmark.rs"

(no output)

$ grep -rn "RepoQuery\|RepoStore" --include=*.rs crates/ apps/ \
    | grep -v "src/intelligence/"

crates/zylcode-core/tests/repo_intelligence_benchmark.rs:10  (test only)
crates/zylcode-core/tests/repo_intelligence_benchmark.rs:16  (test only)
crates/zylcode-core/tests/repo_intelligence_benchmark.rs:66  (test only)
```

`crates/zylcode-core/src/lib.rs:14` declares `pub mod intelligence;` and nothing else in the
workspace consumes it.

- `agent.rs` still builds context via the **pre-existing** `context_builder::ContextBuilder` (`agent.rs:14, 223, 315, 666–687`).
- No CLI subcommand exposes repository intelligence.
- No desktop-app surface exposes it.
- `RepoStore` (SQLite persistence) has no caller outside the module.

**Phase 2A delivered a tested library, not a capability.** The registry records
`repository_scanner`, `rust_symbol_index`, `repository_query_api`, `context_retrieval`,
`git_change_graph` and 6 more as **GREEN**. Under the proof ladder defined in
`ZYLCODE_PROOF_GRAPH.md`, an unreachable module is **R2 (EXECUTED)** at best — tests execute,
but no user or agent can invoke it. It is not R3 (VERIFIED) and it is certainly not the
GREEN the registry implies.

The completion report's own **"Acceptance Demonstration"** section is the clearest signal.
It is titled *"Expected Output"* and contains a hand-written narrative of what the system
*would* answer. There is no command, no captured stdout, no artifact. That is a specification
of intended behaviour, presented in the position where evidence belongs. It is the exact
failure mode this audit process exists to catch.

---

## 6. Finding D — The repository is in a dirty, undocumented state

```
$ git status --porcelain | awk '{print $1}' | sort | uniq -c
     35 ??      (untracked)
     53 M       (modified, uncommitted)
```

53 tracked files are modified but uncommitted, including `crates/zylcode-core/src/agent_protocol.rs`,
`crates/zylcode-core/src/pipeline.rs`, `crates/zylcode-core/src/planner.rs`, and the desktop
app's `main.rs` / `tauri.conf.json`.

Among the untracked files are **three governance drafts that contradict the approved plan**:

| File | Problem |
|---|---|
| `docs/ZYLCODE_CONSTITUTION.md` | Defines **six** engines. The approved architecture defines **seven** (Project System missing). |
| `docs/ZYLCODE_ARCHITECTURE_V2.md` | Same six-engine error; no `PROPOSED` markers for unbuilt systems. |
| `docs/ZYLCODE_ROADMAP_V2.md` | Describes a **14-stage** sequential program. The approved program is **16 phases / 6 epochs**. |

These were never committed, so they are invisible on GitHub — but they sit on the working
tree where the next agent will read them. **An agent following `docs/ZYLCODE_CONSTITUTION.md`
would build the wrong product.** They have been moved to `docs/governance/superseded/` with
explicit banners as part of this audit's remediation.

---

## 6.5 Finding E — The test suite is not green

The report states *"337 workspace tests (all pass)"*. Independently:

```
$ cargo test --workspace

     Running unittests src\lib.rs (target\debug\deps\zylcode_core-840421e82cff2376.exe)
test router::tests::synthetic_offline_dispatch_returns_parseable_payload ... FAILED
test result: FAILED. 225 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

error: test failed, to rerun pass `-p zylcode-core --lib`
```

With detail:

```
$ cargo test -p zylcode-core --lib synthetic_offline_dispatch -- --nocapture

thread 'router::tests::synthetic_offline_dispatch_returns_parseable_payload'
  panicked at crates\zylcode-core\src\router.rs:1030:9:
assertion failed: text.contains("<zylcode-response>")
```

The **synthetic offline provider does not emit the `<zylcode-response>` envelope** that the
router's own test requires. This is a functional break in the offline/air-gapped path — a path
the project explicitly documents (`docs/AIR_GAPPED.md`) and which matters for a local-first
product.

**On attribution — stated precisely.** The audited working tree is dirty (53 modified files).
The failing test lives in `crates/zylcode-core/src/router.rs`, which is **not** modified. The
only router-adjacent modification is `crates/zylcode-core/src/router/decision.rs`, whose diff is
**formatting-only** (rustfmt line-wrapping of string literals and match arms) and cannot change
`dispatch_prompt`'s behaviour. The failure is therefore **most likely pre-existing at `0ecea8e`**.

A clean-checkout confirmation was attempted in an isolated `git worktree`, but the execution
sandbox denied cargo write access outside the primary `target/` directory, so it could not be
completed. **This audit does not claim what it could not verify.**

**Two findings follow regardless of attribution:**

1. The suite is not green, so the "337 all pass" claim is false as stated.
2. **A dirty working tree is not a reviewable artifact.** 53 modified tracked files meant this
   audit could not determine with certainty whether a defect was introduced by Phase 2A or by
   unreviewed later work. That ambiguity is itself a process defect, and it is the direct reason
   remediation item **P1.9** requires the tree to be committed or discarded.

---

## 7. Unverified claims (carried forward)

| Claim | Status |
|---|---|
| "337 workspace tests all pass" | ❌ **CONTRADICTED** — `225 passed; 1 failed` in `zylcode-core`. See §6.5. |
| "Frontend build: PASS" | Not independently confirmed in this audit pass. Must be re-run and recorded with raw output. |
| "Clippy: PASS" / "Formatting: PASS" | Same. |
| "Scan duration: 12.7s" | ❌ **CONTRADICTED** — 276.1s measured (22× slower). |

---

## 8. What is genuinely good in this commit

The audit is not a rejection of the work. The following is sound and should be preserved:

1. **Module decomposition is correct.** `types → classifier → scanner → manifest → symbols → dependency → entry_points → architecture → git → store → query → context` is a clean, testable layering with no circular coupling.
2. **`types.rs` is a genuine canonical model.** `Repository`, `Workspace`, `Package`, `FileNode`, `Symbol`, `Dependency`, `EntryPoint`, `ArchitecturalFingerprint`, `GitCommit`, `ContextResult` are the right primitives.
3. **Provenance is modelled.** `Provenance::{Observed, Parsed, Inferred}` and `ContextResult.reason` mean every retrieval can explain *why*. This is the seed of the Project Knowledge Graph and should be protected.
4. **Honesty about limits.** `symbols.rs` states outright: *"Architecture MUST NOT pretend regex extraction is semantic indexing."* `DEFINITION_INDEXED` vs `IMPORT_RESOLVED` vs `REFERENCE_RESOLVED` is the right distinction, correctly recorded.
5. **Security boundaries are real.** Secret-file exclusion, `follow_symlinks: false`, repo-root-relative paths, local-first with no egress.
6. **79 passing unit tests** is real test mass, and the 1,444-symbol result reproduces exactly.

The failure is not engineering competence. It is **evidence discipline**: inflated headline
metrics, a suite that was reported green without re-execution, and an "acceptance
demonstration" written as expectation rather than captured.

---

## 9. Required remediation before Phase 2A can be re-submitted

Phase 2A is **re-opened**. The following must be completed and independently re-audited.

**P0 — Blocking**

1. **Fix path exclusion.** Make `should_exclude()` path-separator agnostic (normalise to `/`
   before matching, or use `Path::components()`), and replace the hand-rolled gitignore matcher
   with the `ignore` crate (same author as `walkdir`, used by ripgrep). Unit-test against the
   **real repository `.gitignore`**, not a synthetic one.
2. **Re-baseline every number.** Re-run the benchmark against a correctly-scoped index and
   report the new `files_indexed`, precision and recall. Expected genuine index size: **~254 files**.
   All figures in the completion report and capability registry must be corrected, and the
   corrections must be visible as corrections, not silent edits.
3. **Make the benchmark deterministic and self-enforcing.** Assert the headline numbers
   (indexed file count within a tolerance band, symbol count, and a scan-time budget) so the
   suite fails when the metric regresses. Separate the *performance* benchmark from the
   *retrieval quality* benchmark so a slow machine does not red-flag correctness.
4. **Fix the timing budget.** `12.7s` was claimed and `276.1s` measured. Either parallelise the
   scan (hashing 15K files serially is the bottleneck) or set a machine-independent budget
   expressed in files/second. Do not ship a threshold that the author's machine passes and
   a reviewer's machine fails.
5. **Produce a real acceptance demonstration.** A committed transcript — command, raw stdout,
   environment, commit SHA — that a third party can replay verbatim. No "Expected Output".

**P1 — Required for the capability to be honest**

6. **Integrate or downgrade.** Either wire repository intelligence into `AgentLoop`'s context
   assembly (replacing/augmenting `ContextBuilder`) and expose at least one CLI entry point,
   **or** downgrade the 11 GREEN capabilities to `PARTIAL` / `R2` in the registry until
   integration exists. Both are acceptable; claiming GREEN without either is not.
7. **Strengthen the acceptance thresholds.** With a clean index, `Precision@10 ≥ 0.25` is not a
   meaningful bar. Set targets that would fail a keyword search: fix Q5/Q6/Q10 to non-zero
   expectations and require `Precision@10 ≥ 0.60`.
8. **Fill in `Commit SHA` and `Push Verification` inside the report** *before* committing the
   report, or drop those fields. A certificate that predates the thing it certifies is void.
9. **Commit or discard the 53 modified files.** An unreviewed dirty tree is not a reviewable state.

**P2 — Process**

10. The completion report must include a **Reproduction** section: exact commands, environment,
    and raw output. If a claim cannot be reproduced by a third party, it must not be stated.

---

## 10. Process changes this audit forces

1. **The proof ladder in the README is wrong for this purpose.** It is build-centric. A module
   can be fully unit-tested and still be unreachable, unusable, and unverified as a capability.
   `ZYLCODE_PROOF_GRAPH.md` introduces the corrected, capability-agnostic ladder
   **R0 CLAIMED → R1 OBSERVED → R2 EXECUTED → R3 VERIFIED → R4 REPRODUCIBLE → R5 COMMISSIONED**.
   Under it, Phase 2A currently sits at **R2**.
2. **No capability may be marked GREEN on the basis of unit tests alone.** Integration evidence
   is mandatory.
3. **Every completion report must carry a reproduction block.** A report without one is a draft.
4. **`REMOTE_CI = BLOCKED_EXTERNAL` cannot be used as a substitute for local reproduction.**
   With CI blocked, local reproduction is the *only* evidence — which makes it more important,
   not less.

---

## 11. Reproduction appendix

All commands below were run on the audit machine (Windows, `cargo 1.97.1`, `rustc 1.97.1`)
against working tree at `0ecea8e`.

```bash
# 1. Commit exists on the remote
git ls-remote origin | grep refs/heads/main
#   0ecea8e7ef1ca0cc72608e5041a37f5b8e2133c4  refs/heads/main

# 2. Scope of the commit
git show --stat 0ecea8e
#   19 files changed, 6068 insertions(+), 2 deletions(-)

# 3. Intelligence unit tests
cargo test -p zylcode-core --lib intelligence
#   test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 147 filtered out

# 4. Benchmark suite  →  FAILS
cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture
#   Files scanned: 18972
#   Files indexed: 15064
#   Symbols extracted: 1444
#   Scan duration: 276.1043943s
#   panicked: Scan should complete in < 120s, took 276.1043943s
#   test result: FAILED. 1 passed; 1 failed

# 5. Index composition — the decisive measurement
find target -type f -size -512k | wc -l
#   14900
find . -path ./.git -prune -o -name node_modules -prune -o \
       -name target -prune -o -type f -print | wc -l
#   254
git ls-files | wc -l
#   193

# 6. Integration — proves the module is unreachable
grep -rn "intelligence::" --include=*.rs crates/ apps/ \
  | grep -v "src/intelligence/" \
  | grep -v "tests/repo_intelligence_benchmark.rs"
#   (no output)

# 7. Working tree state
git status --porcelain | awk '{print $1}' | sort | uniq -c
#   35 ??
#   53 M

# 8. Full workspace test suite  →  FAILS
cargo test --workspace
#   test result: FAILED. 225 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
#   router::tests::synthetic_offline_dispatch_returns_parseable_payload

# 9. Failing assertion detail
cargo test -p zylcode-core --lib synthetic_offline_dispatch -- --nocapture
#   panicked at crates\zylcode-core\src\router.rs:1030:9:
#   assertion failed: text.contains("<zylcode-response>")
```

---

## 12. Conclusion

Phase 2A's **code** is a reasonable foundation and should not be discarded.

Phase 2A's **evidence** does not survive audit. One headline metric measured the build
directory instead of the repository; the benchmark suite fails when independently executed;
the subsystem is unreachable from the product; the workspace test suite is not green; and the
acceptance demonstration was written as an expectation rather than captured from a run.

**Verdict: NOT ACCEPTED. Phase 2A is re-opened.**

Phase 2B must not begin. Architecture v2 may be written — but it must record the
Intelligence Graph as **R2 / partially implemented**, not as an established foundation.

The audit is not an accusation of bad faith. It is the demonstration of why
*"DeepSeek builds. It does not certify its own work."* is the correct rule: the same
engineer who wrote the scanner also wrote the fixture that hid its defect, and the same
engineer who reported the benchmark green did not re-run it.

---

## 13. Audit sign-off

| Field | Value |
|---|---|
| Commit audited | `0ecea8e7ef1ca0cc72608e5041a37f5b8e2133c4` |
| Remote state | `refs/heads/main` = `0ecea8e` (confirmed via `git ls-remote`) |
| Working tree | **dirty** — 53 modified, 36 untracked |
| Environment | Windows, `cargo 1.97.1`, `rustc 1.97.1` |
| Verdict | **NOT ACCEPTED** |
| Re-opened items | P0 ×5, P1 ×4, P2 ×1 |
| Blocked by this | Phase 2B, and any Architecture v2 claim that 2A is an established foundation |

---

*Audit performed independently. All findings are reproducible from §11.*
