# BENCHMARK PROGRAM — SWE-bench Verified and Terminal-Bench 2.0

**Status:** TARGETS AND DISCLOSURE POLICY ONLY. No benchmark numbers are claimed in this
document. Every number that appears here later must carry the reproduction block defined
in §4, or it does not belong in this file.

**Created:** 2026-09-23
**Predecessor finding:** Phase 2A was rejected because its headline metric counted
`target/` build output and its benchmark failed on re-execution. This document exists to
make that class of failure impossible for the external benchmark program.

---

## 1. What is being measured, and with which harness

| Benchmark | Measures | Harness under test |
|---|---|---|
| SWE-bench Verified | End-to-end issue resolution on 500 human-validated real GitHub issues | ZylCode agent loop (`AgentLoop` + best-of-N) as a standalone scaffold |
| Terminal-Bench 2.0 | Long-horizon terminal task completion | ZylCode agent loop with the shell toolset (`fs.*`, `git.*`, `shell.execute`) |

**Harness disclosure (mandatory in every publication):** results are attributed to the
ZylCode scaffold with the model named per run. A score is meaningless without both —
the harness-vs-model comparison literature (SEAL vs Claude Code on the same model shows
a ~10-point spread) means an undisclosed harness invalidates the claim. Every published
result states: harness version (git SHA), model + provider, temperature, per-candidate
budget, N in best-of-N, and the verifier command.

## 2. What exists today vs. what the benchmarks require

| Requirement | Status today | Gap |
|---|---|---|
| Real tool execution (fs/git/shell/search) | 🟢 Built — 12 real executors, fail-closed gate | — |
| Bounded execution | 🟢 Built — steps, timeouts, cancellation | — |
| Crash recovery | 🟢 Built — checkpoint + ledger reconciliation | — |
| Best-of-N with evidence-scored selection | 🟢 Built (`best_of_n.rs`) — N candidates, real verifier, ledger-recorded selection, refuse-if-all-fail | Single-candidate CLI today; patch-sampling candidate generator is Phase 3 work |
| Self-verification gate | 🟢 Built (`SelfVerificationGate`) — refuses unevidenced acceptance | — |
| Persisted intelligence index | 🟢 Built — content-hash cache kills per-step re-index | — |
| Context retrieval | 🟢 Built — deterministic, 20-query known-answer benchmark | Precision@10 = 0.48 leaves headroom; SWE-bench needs higher |
| Sandboxed execution workers | 🔴 Not built — verifier runs in a plain subprocess with a timeout | **Blocking for Terminal-Bench**: untrusted-repo tasks need OS isolation (Phase 7A) |
| Docker-based evaluation client | 🔴 Not built — SWE-bench runs candidates in per-instance containers | **Blocking for SWE-bench**: the harness must submit patches to the official evaluator |
| Multi-file patch generation + apply | 🔴 Not built — the loop executes tools in-place today | **Blocking for SWE-bench**: candidates must be exportable as `git diff` patches |
| Model-agnostic routing with measured performance | 🟡 Providers wired; routing not measurement-based | Terminal-Bench economy runs need cost-aware routing |

**Publication gate:** no external number is published until the two 🔴-blocking rows for
the target benchmark are built and the harness runs unattended on the official evaluator
container.

## 3. Target ladder (targets, not claims)

| Rung | Definition | SWE-bench Verified target | Terminal-Bench 2.0 target |
|---|---|---|---|
| T1 — Reproducible baseline | Harness runs unattended end-to-end on the official evaluator; any score recorded is real | ≥ 5% (floor: the pipeline works) | ≥ 10% |
| T2 — Competitive floor | Above the median of published open scaffolds on the same model class | ≥ 30% | ≥ 30% |
| T3 — Flagship | Best-of-N + self-verification + measured routing all engaged | ≥ 55% | ≥ 50% |

A rung is reached only when **three consecutive independent runs** at the same SHA
produce scores within ±2 points of each other. A single lucky run is R1 (observed
once), never a result.

## 4. Mandatory reproduction block (per published run)

```
harness:        zylcode @ <git SHA>
model:          <provider/model, temperature, max tokens>
best_of_n:      N=<n>, verifier=`<exact command>`, per-candidate timeout=<s>
index:          intelligence-index.json fingerprint <first 12 hex>
date:           <UTC timestamp>
machine:        <CPU, RAM, OS>
run commands:   <the exact evaluator invocation(s)>
results:        <raw evaluator output file(s), committed under docs/benchmark/runs/>
variance:       scores of the 3 consecutive runs
```

## 5. Anti-overclaim rules (carried from the Five Laws)

1. **Builders do not certify their own work.** Numbers are R4 (reproducible) only after
   a party that did not build the harness re-runs the reproduction block.
2. **The verifier is never mocked in a scored run.** The scored verifier is the official
   evaluator or the real test suite — never an injected or heuristic pass signal.
3. **Failure data is not discarded.** Every scored run's ledger session (all N candidate
   outcomes, including failures) is committed alongside the score.
4. **A sub-threshold result is published as a sub-threshold result.** The Phase 2A
   failure was hiding a bad number; the remedy is publishing honest ones.
5. **Retracted-claims guard applies.** `scripts/check_retracted_claims.py` runs against
   every benchmark publication commit.

## 6. Execution order to first published number

1. **Phase 3A — patch export + candidate sampling** (unblocks SWE-bench submission):
   best-of-N candidates become `git diff` patches; the loop applies, verifies, and
   exports the winner.
2. **Phase 3B — evaluator client** (unblocks SWE-bench): run the official
   `swebench` harness container against exported patches; adopt its verdict as the
   verifier.
3. **Phase 7A slice — worker sandbox** (unblocks Terminal-Bench): container-per-task
   execution with network deny-by-default; the ledger records the sandbox config
   alongside each outcome.
4. **T1 baseline runs** on both benchmarks; commit reproduction blocks.
5. Iterate T2 → T3 with routing engaged; independent audit before any R4 claim.
