# `scripts/`

One Python file per claim the repository makes that can be checked mechanically.
Each is wired into `.github/workflows/ci.yml`, and `python scripts/governance.py`
runs them all locally — the same commands, not a second implementation.

## Gates

| Script                           | Fails when                                                                                                    | Run                                                                 |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `check_retracted_claims.py`      | a claim that was measured, found unsupported and removed reappears                                            | `python scripts/check_retracted_claims.py`                          |
| `check_capability_registry.py`   | the registry's shape breaks: missing rung, unknown status, DISPUTED without a dispute record, drifted summary | `python scripts/check_capability_registry.py`                       |
| `check_capability_probes.py`     | a `policy.must_probe` capability names no real test, file or command                                          | `python scripts/check_capability_probes.py`                         |
| `render_status_board.py`         | `docs/roadmap/ZYLCODE_ROADMAP_V2.md` drifts from `docs/governance/phase-status.json`                          | `python scripts/render_status_board.py --check`                     |
| `render_facts.py`                | the README's counts or status date drift from the registry, probes and phase file                             | `python scripts/render_facts.py --check`                            |
| `check_conditions.py`            | a Phase 2A audit condition no longer holds in the state the register claims                                   | `python scripts/check_conditions.py`                                |
| `check_flaky_register.py`        | an `#[ignore]` is undeclared, a declared quarantine is stale, or a reported flake was silently quarantined    | `python scripts/check_flaky_register.py`                            |
| `check_tool_catalogue_counts.py` | the 27 / 12 / 20 figures disagree with the source or the docs                                                 | `python scripts/check_tool_catalogue_counts.py`                     |
| `freeze_question_set.py`         | a question set under `docs/benchmarks/` has no freeze lock                                                    | `python scripts/freeze_question_set.py --check-all docs/benchmarks` |
| `check_hardcoded_secrets.py`     | a high-confidence provider credential appears in the tree                                                     | `python scripts/check_hardcoded_secrets.py`                         |
| `check_branch_protection.py`     | live `main` protection differs from `docs/governance/branch-protection.json`                                  | `python scripts/check_branch_protection.py`                         |

`check_branch_protection.py` is the one gate CI does not run: reading protection
needs an admin-scoped token, and a gate that reports UNKNOWN on every contributor
machine teaches people to ignore it. It is an owner-run audit, and it separates
"drift" from "cannot read" so an unknown is never reported as a pass.

## Runners

| Command                                       | What it does                                           |
| --------------------------------------------- | ------------------------------------------------------ |
| `python scripts/governance.py`                | every fast gate above, as a table                      |
| `python scripts/governance.py --list`         | what would run, nothing executed                       |
| `python scripts/governance.py --with-network` | adds the branch-protection drift check                 |
| `python scripts/governance.py --with-rust`    | adds `cargo fmt --check`, `clippy -D warnings`, `test` |
| `python scripts/governance.py --with-node`    | adds `pnpm run format:check`                           |

Exit codes are 0 pass, 1 failure, 2 not evaluated — a gate that could not run is
reported as _not a pass_, never as success.

## Conventions these scripts share

- **`--self-test` on every gate.** The gate's own fixtures run in CI, so a future
  edit that widens acceptance fails there rather than in an audit.
- **Exit 2 for unreadable input**, distinct from exit 1 for a real finding. A
  missing file is not drift.
- **`--check` never mutates.** Renders write; checks compare. A CI job must verify,
  not rewrite.
- **No wall clock.** Where a document needs a date, it comes from a declared field
  in the source file (`phase-status.json`'s `updated`), because a generator that
  says "today" produces a different document every day.
