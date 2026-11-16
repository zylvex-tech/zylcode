# Phase 2A submission evidence — 2026-11-16

**Captured:** 2026-11-16
**Commit:** `0d7e71a` (submission head of PR #69; code head `c32f3d8`, one docs commit on top)
**Platform:** Windows 11 developer workstation
**Toolchain:** `rustc 1.97.1`, `cargo 1.97.1`
**Command prefix:** every log is the complete stdout+stderr of one command run from this worktree,
and every log's last line is `EXIT:<status>`.

These logs exist for the independent audit requested in
[`docs/governance/PHASE2A_REAUDIT_REQUEST_2026-11-16.md`](../../governance/PHASE2A_REAUDIT_REQUEST_2026-11-16.md).
They are evidence supplied by the builder, **not** a verdict.

| File                        | Command                                                                                          | Result                                                        |
| --------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| `01-benchmark.log`          | `cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture --test-threads=1`  | `EXIT:0` — 755 indexed, P@10 0.64, R@10 0.96                  |
| `02-negative-threshold.log` | Same test with the Precision floor temporarily raised to 0.99                                    | `EXIT:101` — **the assertion fails**, so it is not decorative |
| `03-negative-scope.log`     | Same test with `src` temporarily added to the forbidden components                               | `EXIT:101` — 287 violations reported                          |
| `04-repo-context.log`       | `cargo run -p zylcode-cli --quiet -- repo-context "where is the retrieval ranking implemented?"` | `EXIT:0` — real ranked output with reasons                    |
| `05-registry.log`           | `python scripts/check_capability_registry.py`                                                    | `EXIT:0` — 37 valid, 12 DISPUTED                              |

The two negative probes were applied to
`crates/zylcode-core/tests/repo_intelligence_benchmark.rs` and **reverted immediately**; the tree was
clean before each commit and is clean in this change set. They exist because the earlier audit's
central objection was that a check which cannot fail certifies nothing.

## What the probes establish

- **Threshold probe:** with the floor at 0.99 and the measured value at 0.64, the merge-gate test
  fails (`test result: FAILED … EXIT:101`). The 0.60 threshold is live.
- **Scope probe:** adding `src` to the independent forbidden-component list produces 287 violations
  and two failing tests. The scope assertion is live, and its forbidden list is separate from the
  classifier's, so weakening the classifier cannot silently satisfy it.

## An observation the auditor should weigh (not a finding of this evidence set)

`04-repo-context.log` answers "where is the retrieval ranking implemented?" with, in order:
`provider_scorecard::ProviderScorecard::ranking`, `provider_scorecard.rs`, `MIN_SAMPLES_FOR_RANKING`,
`TokenRouter::provider_ranking` — keyword matches on the word "ranking" — before any
`intelligence::` file. The top-1 result is not where retrieval ranking is implemented.

That is the kind of query the roadmap's P1.8 test was meant to exclude ("a bar `grep` would fail").
It does not falsify the benchmark's own question set, which is separate; it does indicate the CLI
surface answers keyword-shaped queries plausibly rather than correctly. The builder records it here
rather than leaving the auditor to discover it, and does not rule on it.

## What these logs do not establish

- Cross-platform behaviour. CI on `0d7e71a` covers Ubuntu, macOS and Windows, but these logs are from
  one Windows host.
- Retrieval quality in general. 0.64 is measured on a builder-authored 28-question set.
- That the four falsification tasks in the audit request are complete. Tasks 5.1, 5.2 and 5.3 are
  partially exercised here; 5.4 (the R3 ruling) is the auditor's alone.
