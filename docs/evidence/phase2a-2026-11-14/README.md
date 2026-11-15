# Phase 2A remediation — captured verification evidence

**Captured:** 2026-11-15
**Platform:** Windows 11 (developer workstation, not a CI runner)
**Toolchain:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`
**Branch:** `fix/phase2a-scope-and-evidence`, based on `29638d4`
**Corpus:** the branch working tree (767 tracked files, 753 indexed). The benchmark is not a
fixed fixture; see the corpus note at the top of `crates/zylcode-core/tests/repo_intelligence_benchmark.rs`.

Each log is the complete stdout+stderr of one command as run from the repository root of the
remediation worktree, with `CARGO_TARGET_DIR` pointing at a shared target directory. The first
line is the command; **the last line is `EXIT:<status>`**. Nothing is trimmed, summarised or
reordered. The logs are committed so a reviewer does not depend on a developer machine's
temporary directory.

| File                  | Command                                                                                         | Purpose                                                 |
| --------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `01-fmt.log`          | `cargo fmt --all -- --check`                                                                    | Formatting gate                                         |
| `02-check.log`        | `cargo check --workspace --all-targets`                                                         | Compilation gate                                        |
| `03-clippy.log`       | `cargo clippy --workspace --all-targets -- -D warnings`                                         | Lint gate                                               |
| `04-intelligence.log` | `cargo test -p zylcode-core --lib intelligence::`                                               | Classifier/scanner unit tests                           |
| `05-benchmark.log`    | `cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture --test-threads=1` | Retrieval quality + scan measurement + scope assertions |
| `06-workspace.log`    | `cargo test --workspace --all-targets`                                                          | Full workspace test gate                                |
| `07-registry.log`     | `python scripts/check_capability_registry.py`                                                   | Registry shape gate                                     |

## What these logs establish, and what they do not

They establish that the commands above passed on this commit on this machine, and they carry the
raw metric lines (`Files indexed`, `Average Precision@10`, `MEASUREMENT scan_files=…`), the
per-suite `test result:` lines, and each command's exit status.

They do **not** establish cross-platform CI. No remote CI run exists for this branch yet; that
appears when the branch is pushed. They also do not establish an agreed performance acceptance
budget — the throughput figure is a measurement, and the enforced floor is a collapse detector,
per the owner-approved exception recorded in `docs/PHASE2A_COMPLETION_REPORT.md` §7.

Absolute timings are machine-specific. Compare file counts and exit statuses across machines
before comparing durations.
