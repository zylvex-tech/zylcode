# Flaky and quarantined tests

An ignored test produces no result. That is the whole problem: a red test that
becomes `#[ignore]`d is a green build over an invisible failure, and nothing in CI
can tell the difference between a deliberate quarantine and a deleted assertion.

So the repository keeps one list, `docs/governance/flaky-tests.json`, and
`scripts/check_flaky_register.py` derives the ignored set from source and compares
it on every gate run.

## The policy

| Rule                                              | Why                                                                                            |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Every `#[ignore]` must be in `quarantined`        | An undeclared skip is indistinguishable from a deleted test.                                   |
| Each entry names the class and the exit condition | "Needs Ollama" and "costs the owner's key" are different problems with different fixes.        |
| A reported flake that still runs is listed too    | "It fails sometimes" is a measurement, not folklore; the entry says it is unverified if it is. |
| A quarantined test that disappears is an error    | Deleting the test does not discharge the quarantine — the entry has to go with it.             |

## Currently quarantined (4)

| Test                                               | File                          | Class               |
| -------------------------------------------------- | ----------------------------- | ------------------- |
| `test_real_provider_commissioning`                 | `tests/commissioning_test.rs` | external dependency |
| `live_provider_commissioning_probe`                | `tests/live_commissioning.rs` | spends owner credit |
| `live_natlas_returns_a_genuine_non_empty_response` | `tests/natlas_live.rs`        | external dependency |
| `live_acceptance_journey`                          | `tests/natlas_live.rs`        | external dependency |

None of the four is a workaround for a failing assertion. Each needs something the
machine running CI does not have: a live Ollama model, the owner's provider key, or
a live N-ATLAS endpoint. The `unquarantine_when` field in the register is where the
condition for running them again is written down.

## Reported, still running (1)

`terminal::tests::a_relative_cwd_runs_inside_the_workspace` was reported failing
intermittently on Windows before 2026-11-18. It is **not** ignored. It stays in the
default suite, and the register records the report rather than a reproduction — the
entry says so explicitly, because a flake nobody reproduced is a hypothesis.

## Adding a quarantine

1. Add `#[ignore = "<reason>"]` to the test.
2. Add the entry to `flaky-tests.json` with the same reason string, a `class`, and
   an `unquarantine_when`.
3. Run `python scripts/check_flaky_register.py`. It fails if the reason text differs
   by a character, which is deliberate: two copies of a reason that disagree is how
   "temporarily disabled" becomes permanent.

## What this does not do

It does not detect flakiness. Nothing here observes a test twice; it only makes the
set of tests with no result a stated, reviewable decision.
