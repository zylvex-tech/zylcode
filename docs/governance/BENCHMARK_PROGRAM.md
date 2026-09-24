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
| Best-of-N with evidence-scored selection | 🟢 Built (`best_of_n.rs`, `patch_best_of_n.rs`) — N model-sampled patches, per-candidate worktree verification, concurrent fan-out, ledger-recorded selection, refuse-if-all-fail, winner export | Exported patch format is `git apply`-compatible unified diff |
| Self-verification gate | 🟢 Built (`SelfVerificationGate`) — refuses unevidenced acceptance | — |
| Persisted intelligence index | 🟢 Built — content-hash cache kills per-step re-index | — |
| Context retrieval | 🟢 Built — deterministic, 20-query known-answer benchmark | Precision@10 = 0.48 leaves headroom; SWE-bench needs higher |
| Sandboxed execution workers | 🟢 Built (`sandbox.rs`, Phase 7A slice) — container-per-candidate docker verifier: `--network=none`, `--read-only` rootfs, size-capped noexec tmpfs, CPU/memory/PID caps, worktree-only writable mount, ready-marker env; **fails closed** (no daemon or image ⇒ refusal, never host fallback); concurrent-safe. In-container commissioning run requires an operator-provided image. | Podman/other backends not yet supported; Windows requires Linux containers |
| Docker-based evaluation client | 🔴 Not built — SWE-bench runs candidates in per-instance containers | **Blocking for SWE-bench**: the harness must submit patches to the official evaluator |
| Multi-file patch generation + apply | 🟢 Built — Phase 3A candidates are `git diff` patches applied via `git apply` in isolated worktrees | Benchmark task instance → model context mapping is the remaining harness work |
| Model-agnostic routing with measured performance | 🟡 Providers wired; routing not measurement-based | Terminal-Bench economy runs need cost-aware routing |

**Publication gate:** no external number is published until the 🔴-blocking rows for
the target benchmark are built and the harness runs unattended on the official evaluator
container. (The sandbox row is built but its full in-container commissioning gate still
requires an operator-provided image on the runner.)

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

1. ~~Phase 3A — patch export + candidate sampling~~ **Done** (`4f80b31`): model-sampled
   `git diff` candidates, per-candidate worktree verification, concurrent fan-out
   (`8181a0c`), winner export byte-identical to what was verified.
2. ~~Phase 7A slice — worker sandbox~~ **Done** (`sandbox.rs`): container-per-candidate
   verification, network deny-by-default, fail-closed without a backend; the sandbox
   policy is recorded in every outcome's evidence. Remaining for the gate: operator
   image provisioning + an in-container commissioning run.
3. **Phase 3B — evaluator client** (unblocks SWE-bench): run the official
   `swebench` harness container against exported patches; adopt its verdict as the
   verifier. The sandbox verifier is the template for this client.
4. **T1 baseline runs** on both benchmarks; commit reproduction blocks.
5. Iterate T2 → T3 with routing engaged; independent audit before any R4 claim.
