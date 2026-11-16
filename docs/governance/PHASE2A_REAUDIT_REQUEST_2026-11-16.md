# Phase 2A Re-Audit Request — 2026-11-16

**Date:** 2026-11-16
**Submission:** Pull request #69 — `fix/phase2a-scope-and-evidence`
**Code head under audit:** `c32f3d8`
**Branch head at submission:** `f9d267c` — this document only; no source, test or registry change
follows `c32f3d8`
**Base:** `main` @ `29638d4`
**Requested by:** ZylCode engineering (builder — **not** the acceptance authority)
**Audit method:** [`PHASE2A_INDEPENDENT_AUDIT.md`](PHASE2A_INDEPENDENT_AUDIT.md)
**Remediation order:** [`PHASE2A_REMEDIATION_ORDER.md`](PHASE2A_REMEDIATION_ORDER.md)
**Supersedes:** [`PHASE2A_REAUDIT_REQUEST_2026-11-11.md`](PHASE2A_REAUDIT_REQUEST_2026-11-11.md),
which bound its measurements to `c2a2b94`. This request binds every command to `c32f3d8` and to the
committed evidence bundle, so an auditor no longer has to trust a stale SHA.

---

## 1. Status

**SUBMITTED FOR AUDIT.** Not accepted. Not complete. The 12 disputed capability entries remain
`DISPUTED`, Phase 2B remains **BLOCKED**, and neither changes until an independent auditor returns a
verdict recorded in §8.

The builder has not written a verdict, and will not edit the auditor's document.

---

## 2. Who audits

Per governance Law 3 — _builders do not certify their own work_ — the auditor is an **independent AI
agent**, appointed by the repository owner, that:

- did not author, review or modify any part of this change set;
- received this document as its instructions, not the builder's conclusions;
- runs every command itself in a checkout of `c32f3d8` and reports raw output;
- is instructed to try to **falsify** the submission, not to confirm it.

The builder's role ends at supplying the submission, the commands and the evidence. The auditor's
verdict is recorded verbatim. If the verdict is `NOT ACCEPTED`, Phase 2A re-opens and Phase 2B stays
blocked; no code change in this request can override that.

---

## 3. What is being submitted

A six-file change set that repairs the findings of the 2026-09-15 audit:

| Area                               | What the code now does                                                                                                                                                      |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path exclusion (P0.1)              | `should_exclude()` matches `Path::components()`/both separators; real `.gitignore` is loaded through the `ignore` crate; nine exclusion unit tests plus two traversal tests |
| Index scope (P0.2, P0.3)           | `assert_index_scope()` in both benchmark tests: no indexed path may contain `target/`, `node_modules/` or `.git/`                                                           |
| Metric re-baseline (P0.2)          | `docs/PHASE2A_COMPLETION_REPORT.md` §1a and §3: every withdrawn figure with its root cause                                                                                  |
| Benchmark enforcement (P0.3, P1.8) | `Precision@10 >= 0.60` **and** `Recall@10 >= 0.60` asserted over 28 known-answer questions; performance split into its own test                                             |
| Integration (P1.7)                 | option **(a) integrate**: agent context assembly consumes ranked intelligence; `zylcode repo-context <task>` is a named CLI surface                                         |
| Registry honesty (P0.6)            | 12 Phase 2A entries left `DISPUTED` rather than promoted; `rung` present on all entries; three GREEN/R2 conflicts declared                                                  |
| Evidence (P0.5)                    | raw logs committed under `docs/evidence/phase2a-2026-11-14/`, each ending `EXIT:<status>`                                                                                   |

Nothing in this change set promotes a capability to GREEN, and nothing changes a governance
document's rules. This request is a new record; the 2026-09-15 audit and the rejected report remain
in git history, unedited.

---

## 4. Reproduction block

**Environment used for the committed evidence:** Windows 11 developer workstation,
`rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
CI on the same commit: `CI` (Ubuntu, macOS, Windows — each running
`cargo test --workspace --all-targets`) and `Coverage`, all green.

Run everything from a clean checkout of `c32f3d8`. On Windows Git Bash, `export MSYS_NO_PATHCONV=1`
before paths are passed to native tools.

```bash
git checkout c32f3d8

# 1. Formatting gate
cargo fmt --all -- --check

# 2. Compilation gate
cargo check --workspace --all-targets

# 3. Lint gate
cargo clippy --workspace --all-targets -- -D warnings

# 4. Intelligence unit tests
cargo test -p zylcode-core --lib intelligence::

# 5. Retrieval quality + scan measurement + scope assertions (the merge gate)
cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture --test-threads=1

# 6. Full workspace tests
cargo test --workspace --all-targets

# 7. Registry shape gate
python scripts/check_capability_registry.py

# 8. Entry point, run for real
cargo run -p zylcode-cli -- repo-context "where is the retrieval ranking implemented?"
```

Expected metric lines from step 5, as captured in
`docs/evidence/phase2a-2026-11-14/05-benchmark.log`:

```text
Files scanned: 768
Files indexed: 754
Average Precision@10: 0.64
Average Recall@10: 0.96
test result: ok. 2 passed; 0 failed; 0 ignored
```

File counts vary with the live checkout (the corpus is the working tree, not a fixed fixture) — the
**invariant** is the scope assertion, not the number. Durations are machine-specific.

Committed evidence: `01-fmt.log` … `07-registry.log` in `docs/evidence/phase2a-2026-11-14/`, with a
README stating exactly what they do and do not establish.

---

## 5. The four falsification tasks

The auditor should attempt to break each claim, not verify it politely.

### 5.1 Falsify the scope

- Index the repository independently and confirm **no** indexed path contains `target/`,
  `node_modules/` or `.git/`.
- Try to make the scanner index build output: add a nested `target/` tree with a source file in it and
  re-run. It must not appear.
- Confirm `assert_index_scope()` actually fails when an excluded path is injected into the result set.
  A check that cannot fail is not a check.

### 5.2 Falsify the thresholds

- Confirm the benchmark asserts `P@10 >= 0.60` **and** `R@10 >= 0.60` in the merge-gate test.
- Force a regression (for example, filter the ranked results to a subset) and confirm the test fails.
  If it still passes, the threshold is decorative.
- Confirm the numbers are not read from a stale cache or a committed fixture.

### 5.3 Invoke the entry points

- Run `zylcode repo-context "…"` and confirm ranked results carry reasons.
- Confirm agent context assembly consumes that ranking (not the legacy keyword heuristic) and that
  graph citations are deterministic, or explicitly reported unavailable.
- The proof rung for a capability is reachability from a user surface; a library test is R2, not R3.

### 5.4 Rule on the R3 claim

Decide, per capability, whether the 12 `DISPUTED` entries are now R3, should be `PARTIAL`/R2, or
should be re-opened. The registry must match
[`ZYLCODE_CAPABILITY_MODEL.md`](ZYLCODE_CAPABILITY_MODEL.md) §4.2 (GREEN ⇒ R3).

---

## 6. Known limitations (auditor should read before ruling)

Carried from the completion report, deliberately not softened:

- **Symbol resolution is regex-based.** Only `DEFINITION_INDEXED` is produced; `IMPORT_RESOLVED` and
  `REFERENCE_RESOLVED` are not implemented.
- **The question set is builder-authored.** 28 known-answer questions, 8 of them added to raise the
  average. A 0.64 on a builder-authored set is weaker evidence than the number suggests.
- **Throughput is not an acceptance budget.** The scan measurement is gated by a collapse floor so a
  slow machine cannot red-flag correctness; a scan regression would not fail CI.
- **Incremental indexing is file-level only.**
- **The corpus is the working tree**, so file counts move with the checkout and untracked files can
  appear (the evidence README states this rather than claiming a fixed count).
- **Three GREEN entries declare GREEN/R2 conflicts** in the report §10 rather than being silently
  downgraded.
- **Two ⚠️ items remain in the report's Definition of Done table** (timing budget not an acceptance
  budget; three registry exceptions). They are visible on purpose.

If any of these is disqualifying, the correct verdict is `NOT ACCEPTED` or `ACCEPTED WITH
CONDITIONS` — not silence.

---

## 7. What must not happen

- The auditor must not edit source, tests, the registry, or the completion report. It may only read,
  run, and write its verdict.
- The builder must not edit the verdict document after it is written.
- No capability may be promoted to GREEN as part of the audit; the registry change is a separate
  commit made from the verdict, and only after it.
- Phase 2B must not begin on any verdict other than `ACCEPTED` or `ACCEPTED WITH CONDITIONS`
  (conditions are tracked, not ignored).

---

## 8. Verdict

To be written by the auditor at `docs/governance/PHASE2A_AUDIT_VERDICT_2026-11-16.md`:

```markdown
# Phase 2A Audit Verdict — 2026-11-16

**Auditor:** <agent/model identifier and version>
**Commit audited:** c32f3d8
**Independence:** I did not author, review or modify this change set.
**Verdict:** ACCEPTED | ACCEPTED WITH CONDITIONS | NOT ACCEPTED (RE-OPENED)

## Commands I ran, and what I observed

<verbatim, including failures and anything I could not run>

## Findings against the four falsification tasks

1. Scope — <result, including any attempt to break it>
2. Thresholds — <result, including the forced-regression attempt>
3. Entry points — <result of actually invoking them>
4. R3 ruling — <per-capability decision>

## Conditions (if any)

<numbered, testable>

## What would have changed my verdict

<the strongest counter-argument to my own ruling>

## Limitations of this audit

<what I did not verify>
```

Only a verdict in that file moves Phase 2A. Until it exists, the phase is **NOT ACCEPTED** and Phase
2B stays **BLOCKED**.

---

> **Built with evidence. Verified with honesty.**
