# Phase 2A Audit Verdict — 2026-11-16

**Auditor:** Independent AI agent (WorkBuddy AI / Claude-class model, acting as ZylCode independent auditor)
**Commit audited:** `c32f3d8` (code head of PR #69 `fix/phase2a-scope-and-evidence`)
**Branch head:** `bef322a` (documentation and evidence-log commits only, on top of `c32f3d8`)
**PR:** #69 on `https://github.com/zylvex-tech/zylcode`
**Independence:** I did not author, review or modify this change set. I received the audit request as my instructions, not the builder's conclusions.
**Verdict:** **ACCEPTED WITH CONDITIONS**

> **Provenance.** This document is the auditor's verdict, transcribed into the repository unedited by
> the builder. The builder did not write, revise or soften any part of it. It was returned by the
> independent auditor session commissioned against
> [`PHASE2A_REAUDIT_REQUEST_2026-11-16.md`](PHASE2A_REAUDIT_REQUEST_2026-11-16.md).

---

## Commands I ran, and what I observed

All commands were run in a clean detached worktree at `c32f3d8` (`/c/Temp/p2a-audit-2026-11-16`),
zero dirty files, on Windows 11 with `rustc 1.97.1`, `cargo 1.97.1`. `CARGO_TARGET_DIR` was pointed
at a temp directory because the sandbox blocks writes under `target/debug`. `NO_PROXY=127.0.0.1,localhost`
was set for network-probe tests (see Finding F1).

### §4.1 — Formatting gate

```
$ cargo fmt --all -- --check
(exit 0, no output)
```

**PASS.**

### §4.2 — Compilation gate

```
$ cargo check --workspace --all-targets
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 8m 13s
(exit 0)
```

**PASS.**

### §4.3 — Lint gate

```
$ cargo clippy --workspace --all-targets -- -D warnings
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 39s
(exit 0, zero warnings)
```

**PASS.**

### §4.4 — Intelligence unit tests

```
$ cargo test -p zylcode-core --lib intelligence::
test result: ok. 110 passed; 0 failed; 0 ignored; 0 measured; 591 filtered out; finished in 22.47s
```

**PASS.** 110 intelligence tests, all green.

### §4.5 — Retrieval quality + scan measurement + scope assertions (the merge gate)

```
$ cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture --test-threads=1

=== Indexing Performance (measurement, not acceptance proof) ===
Files scanned: 768
Files indexed: 754
Symbols extracted: 3210
Packages: 8
Dependency edges: 7
Scan duration: 13.020s
Scan throughput: 59.0 files/second
MEASUREMENT scan_files=768 indexed_files=754 scan_seconds=13.020 files_per_second=59.0

=== Repository Intelligence Benchmark ===
Files indexed: 754
Symbols indexed: 3210
Packages: 8
Entry points: 34

Average Precision@10: 0.64
Average Recall@10: 0.96

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 170.51s
```

**PASS.** Both benchmark tests green. All headline numbers reproduce within expected variance:

- Files indexed: **754** (builder claimed 754–760) ✅
- Symbols: **3210** (builder claimed 3210) ✅
- Packages: **8** (builder claimed 8) ✅
- Entry points: **34** (builder claimed 34) ✅
- Precision@10: **0.64** (builder claimed 0.64, threshold ≥0.60) ✅
- Recall@10: **0.96** (builder claimed 0.96, threshold ≥0.60) ✅
- Scan throughput: **59.0 files/s** (above the 10 files/s collapse floor) ✅

### §4.6 — Full workspace tests

```
$ cargo test -p zylcode-core --lib (with NO_PROXY=127.0.0.1,localhost)
test result: ok. 701 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 52.91s
```

**PASS.** 701 zylcode-core lib tests, all green.

The previously failing `router::tests::synthetic_offline_dispatch_returns_parseable_payload` passes:

```
$ cargo test -p zylcode-core --lib synthetic_offline_dispatch_returns_parseable_payload
test router::tests::synthetic_offline_dispatch_returns_parseable_payload ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 700 filtered out
```

**One network-probe test** (`probe_classifies_a_refused_connection_as_network_error`) fails in this
environment when the HTTP proxy is set and `NO_PROXY` is empty. With `NO_PROXY=127.0.0.1,localhost`
it passes. See Finding F1 for attribution.

### §4.7 — Registry shape gate

```
$ python scripts/check_capability_registry.py
OK: 37 capabilities valid - R2 17, R3 20; statuses DISPUTED 12, EXTERNAL_BLOCKER 1, GREEN 22, PARTIAL 2
(exit 0)
```

**PASS.**

### §4.8 — Entry point, run for real

```
$ cargo run -p zylcode-cli -- repo-context "where is the retrieval ranking implemented?"

task: where is the retrieval ranking implemented?
indexed: 754 files, 3210 symbols, 8 packages, 34 entry points (14049 ms)
top results:
   1.65 [symbol] crates::zylcode-core::src::provider_scorecard::ProviderScorecard::ranking :: ...
   1.00 [file] crates/zylcode-core/src/provider_scorecard.rs :: ...
   0.87 [symbol] crates::zylcode-core::src::provider_scorecard::MIN_SAMPLES_FOR_RANKING :: ...
   0.87 [symbol] crates::zylcode-core::src::router::TokenRouter::provider_ranking :: ...
   ...
```

**PASSES (exit 0).** The CLI produces ranked results with reasons. However, the top-1 result
(`ProviderScorecard::ranking`) is about provider ranking, not intelligence retrieval ranking — see
Finding F2.

Second query:

```
$ cargo run -p zylcode-cli -- repo-context "where is should_exclude defined"

top results:
   2.87 [symbol] crates::zylcode-core::src::intelligence::classifier::should_exclude :: ...
   1.43 [file] crates/zylcode-core/src/intelligence/classifier.rs :: ...
   0.97 [symbol] crates::zylcode-core::src::intelligence::classifier::EXCLUDED_DIR_NAMES :: ...
```

**Correct.** Top-1 is the exact symbol, top-2 is the exact file.

---

## Findings against the four falsification tasks

### 5.1 Scope — PASS

**Independent verification:** I confirmed zero `target/`, `node_modules/`, `.git/`, `dist/`, or
`build/` paths in the benchmark output by grep. The `assert_index_scope()` assertion ran in both
benchmark tests and both passed.

**Injection test:** I created `crates/zylcode-core/target/fake/injected.rs` containing a Rust source
file and re-ran the benchmark. The file was NOT indexed — file count remained 768 scanned / 754
indexed, identical to the clean run. The scanner correctly excluded the nested `target/` directory.

**Near-match safety:** The classifier tests (`dont_exclude_near_match_directory`) prove `mytarget/`,
`src/target_helpers.rs`, `src/build.rs`, `docs/build-guide.md`, and `crates/x/distances.rs` are NOT
excluded. The component-based matching (`Path::components()`) correctly distinguishes `target` as a
path component from `target` as a substring.

### 5.2 Thresholds — PASS

**Assertion verification:** The benchmark asserts `avg_precision >= 0.60` and `avg_recall >= 0.60` at
line 464 and 474 of `repo_intelligence_benchmark.rs`. These are in the merge-gate test
`benchmark_repository_intelligence`, not in an `#[ignore]`d test.

**Forced regression — threshold:** I temporarily changed the precision floor from 0.60 to 0.99 in a
copy of the test file. The test **FAILED**:

```
thread 'benchmark_repository_intelligence' panicked:
Average Precision@10 = 0.65, expected >= 0.60
test result: FAILED. 0 passed; 1 failed
```

(The error message format string still said 0.60 because I only changed the assertion condition, not
the message — but the assertion used 0.99 and 0.65 < 0.99, so it failed.) **The threshold is live.**

**Forced regression — scope:** I temporarily added `"src"` to the `FORBIDDEN_INDEX_COMPONENTS` list.
The test **FAILED** with 287 violations:

```
thread 'benchmark_repository_intelligence' panicked:
index scope violated: 287 path(s) under excluded directories
(first 10: ["apps/natlas-sdk/src/index.ts", ...])
test result: FAILED. 0 passed; 1 failed
```

This exactly reproduces the builder's committed evidence (`03-negative-scope.log`). **The scope
assertion is live and its forbidden list is independent from the classifier's exclusion list.** Both
probes were reverted; the tree was restored to clean.

### 5.3 Entry points — PASS (with observation)

**CLI surface:** `zylcode repo-context <task>` works, produces ranked results with reasons, and is a
named CLI subcommand. I invoked it with two queries and confirmed real output.

**Agent integration:** `ContextBuilder::build()` calls `find_relevant_files_intelligent()` which uses
`PersistedIndex::build()` — the same pipeline as the CLI. The integration test
`context_builder_uses_intelligence_ranking` proves the agent's context includes files the keyword
heuristic cannot produce (specifically `crash_recovery` and `ledger.rs` via co-change). The test
`agent_context_carries_deterministic_graph_citations` proves all citations start with
`DETERMINISTIC` (never `HEURISTIC`). Both tests pass.

**Observation (F2):** The CLI's top-1 answer for "where is the retrieval ranking implemented?" is
`ProviderScorecard::ranking` — a provider-ranking method, not the intelligence retrieval ranking. The
system matched on the keyword "ranking" but did not disambiguate "retrieval" context. This does not
falsify the benchmark question set (which does not include this query), but it indicates the CLI
answers keyword-shaped queries plausibly rather than correctly for cases outside the benchmark
corpus. The builder recorded this observation in the evidence README without ruling on it.

### 5.4 R3 ruling — per-capability decision

The product surfaces that establish reachability:

1. **CLI:** `zylcode repo-context <task>` → `PersistedIndex::build()` → `build_repo_query()` →
   `RepoQuery::relevant_context()`
2. **AgentLoop:** `ContextBuilder::build()` → `find_relevant_files_intelligent()` → same pipeline +
   `nav_api::agent_citation_block()`

The canonical pipeline (`build_repo_query`) exercises: scanner, manifest discovery, Rust symbol
extraction, dependency graph, entry points, architecture fingerprint, git history, and the
query/retrieval layer.

| #   | Capability                 | Builder's rung     | Auditor's ruling | Reasoning                                                                                                                                                                                                                                                                         |
| --- | -------------------------- | ------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `repository_scanner`       | R2                 | **R3**           | First step of the pipeline invoked by both surfaces. Scope assertion proves correctness. User sees "754 files indexed".                                                                                                                                                           |
| 2   | `manifest_intelligence`    | R2                 | **R3**           | Package/workspace discovery runs in the pipeline. User sees "8 packages".                                                                                                                                                                                                         |
| 3   | `rust_symbol_index`        | R2                 | **R3**           | Symbol extraction runs in the pipeline. User sees ranked symbol results.                                                                                                                                                                                                          |
| 4   | `typescript_symbol_index`  | R2                 | **R2**           | The TS extractor exists in `symbols.rs` but `build_repo_query()` and the benchmark only extract `Language::Rust` symbols. TS symbol extraction is never invoked through any product surface. Remains R2.                                                                          |
| 5   | `package_dependency_graph` | R2                 | **R3**           | Q6 proves `dependents_of('zylcode-core')` returns correct results. Dependency distance feeds ranking.                                                                                                                                                                             |
| 6   | `git_change_graph`         | R2                 | **R3**           | Full git history feeds co-change mining. Integration test proves `ledger.rs` surfaces via co-change evidence.                                                                                                                                                                     |
| 7   | `architecture_fingerprint` | R2                 | **R3**           | Architecture fingerprint generated in pipeline and used in retrieval ranking.                                                                                                                                                                                                     |
| 8   | `incremental_indexing`     | R2                 | **R3**           | `PersistedIndex` persists to `.zylcode/intelligence-index.json` with content-hash file-level invalidation. Fast path loads from cache; slow path re-indexes and writes through. Reachable via CLI and AgentLoop. Symbol-level invalidation is not implemented (known limitation). |
| 9   | `repository_query_api`     | R2 (p12 claims R3) | **R3**           | `RepoQuery::relevant_context()` is the direct call from the CLI. 14 query methods exist. User invokes one through `repo-context`.                                                                                                                                                 |
| 10  | `context_retrieval`        | R2 (p12 claims R3) | **R3**           | `relevant_context()` produces ranked results with reasons. Integration test proves it's not the keyword heuristic. Consumed by CLI and AgentLoop.                                                                                                                                 |
| 11  | `repository_benchmark`     | R2                 | **R2**           | A test harness, not a product capability. No user invokes the benchmark through a product surface.                                                                                                                                                                                |
| 12  | `documentation_accuracy`   | R2                 | **R2**           | No actor entry point. The scope dispute is about what "documentation accuracy" means as a capability.                                                                                                                                                                             |

**Summary:** 9 of 12 disputed entries advance to R3. 3 remain R2 (`typescript_symbol_index`,
`repository_benchmark`, `documentation_accuracy`).

The builder's p12_rebaseline claimed `repository_query_api` and `context_retrieval` as R3/GREEN. I
confirm R3 for both. I also find that 7 additional entries the builder left at R2 are now R3 through
the same surfaces — the builder was conservative rather than overclaiming, which is the right
direction.

---

## Findings (auditor-initiated)

### F1 — Network-probe test fails without proxy bypass (environment, not code)

`router::tests::probe_classifies_a_refused_connection_as_network_error` fails when
`HTTP_PROXY`/`HTTPS_PROXY` are set and `NO_PROXY` is empty. The proxy at `127.0.0.1:60830` intercepts
the test's connection to a closed port on `127.0.0.1:<ephemeral>` and returns an HTTP rejection,
causing `ProviderRejected` instead of `NetworkError`. With `NO_PROXY=127.0.0.1,localhost`, the test
passes.

**Attribution:** `router.rs` was NOT modified by this PR (0 lines in the diff). The test is
pre-existing. This is an environment artifact, not a code defect. CI (Ubuntu, macOS, Windows) does
not set a proxy, so this test passes in CI.

### F2 — CLI top-1 for "retrieval ranking" is keyword-correct but semantically wrong

Running `zylcode repo-context "where is the retrieval ranking implemented?"` returns
`ProviderScorecard::ranking` (provider ranking) as top-1, not `intelligence::query::relevant_context`
or `context.rs` (where retrieval ranking is actually implemented). The system matched on "ranking"
but not "retrieval". This is the known limitation of keyword-based retrieval and does not contradict
the benchmark's 0.64 precision (which is measured on a different question set). The builder recorded
this observation honestly in the evidence README.

### F3 — TypeScript symbol extraction is dead code in the pipeline

`build_repo_query()` and `index_zylcode()` both filter to `Language::Rust` only. The `extract_symbols`
function supports TypeScript (it dispatches on language), but the canonical pipeline never calls it
with a TypeScript file. This means `typescript_symbol_index` is R2 — the code exists and is tested in
isolation, but no product surface exercises it.

### F4 — Three GREEN/R2 conflicts remain unresolved

The registry declares three entries as GREEN with R2 evidence (`observation_loop`, `repair_loop`,
`test_model_client`). These are pre-existing conflicts, declared honestly by the builder. The auditor
does not resolve them in this verdict — they are outside Phase 2A's scope and predate this PR.

---

## Conditions

1. **C1 — `typescript_symbol_index` must remain R2/DISPUTED.** The TS symbol extractor is not
   exercised through any product surface. If the builder wishes to claim R3, they must wire TS symbol
   extraction into `build_repo_query()` and demonstrate a query that returns a TypeScript symbol
   through the CLI or AgentLoop. This is a Phase 2B+ task.

2. **C2 — `repository_benchmark` and `documentation_accuracy` must remain R2/DISPUTED.** A test
   harness is not a product capability. Documentation accuracy has no actor entry point. These are
   correctly R2.

3. **C3 — The three GREEN/R2 conflicts (`observation_loop`, `repair_loop`, `test_model_client`) must
   be resolved in a future governance pass.** They predate this PR and are not blocking, but
   GREEN-below-R3 is a model violation that must not persist indefinitely. The builder has declared
   them; the resolution is the architecture owner's call.

4. **C4 — The question set remains builder-authored.** 28 questions, 8 of which (Q21–Q28) were added
   expressly to raise the average. A 0.64 on a builder-authored set is weaker evidence than the
   number suggests. The next phase should add independently-authored questions or a held-out
   evaluation set.

5. **C5 — Throughput has a collapse floor, not an acceptance budget.** The scan measures 59–471
   files/s depending on cache warmth. No machine-independent acceptance budget exists. The owner
   authorized this as an explicit exception (P0.4). The auditor accepts this as a condition, not a
   blocker, because the collapse floor (10 files/s) catches catastrophic regressions and the scan is
   not on the user's critical path (the persisted cache serves repeat invocations).

6. **C6 — Symbol resolution remains regex-based (DEFINITION_INDEXED only).** `IMPORT_RESOLVED` and
   `REFERENCE_RESOLVED` are not implemented. Cross-file references are best-effort. This is a known
   limitation and does not block Phase 2A, but Phase 2B's semantic resolution work must build on this
   foundation.

---

## What would have changed my verdict

The strongest counter-argument to ACCEPTED WITH CONDITIONS is that the CLI's top-1 answer for "where
is the retrieval ranking implemented?" is semantically wrong — it returns provider ranking, not
intelligence retrieval ranking. If the benchmark question set had included this query and it had
scored 0.00 precision, the average would drop from 0.64 to 0.62 (still above 0.60). But if several
such queries existed and dropped the average below 0.60, the threshold would fail and the verdict
would be NOT ACCEPTED.

The reason this does not change my verdict is that the benchmark question set is separate from ad-hoc
CLI queries, and the builder honestly disclosed this limitation. The benchmark measures 28 specific
questions and passes its threshold. The CLI's keyword-shaped retrieval is a known limitation of
regex-based retrieval, not a fabrication.

A second counter-argument is that 8 of 28 benchmark questions were added by the builder to raise the
average. If those 8 were removed, the average over the original 20 would be:
Q1–Q10 average precision = (0.90+0.30+0.60+0.50+0.70+1.00+0.20+0.70+1.00+0.50)/10 = 0.64
Q11–Q20 average precision = (0.40+0.70+0.50+0.20+0.30+0.70+0.90+0.90+0.80+0.40)/10 = 0.58
Combined Q1–Q20 = (6.40+5.80)/20 = 0.61 — still above 0.60, but barely.

This is close enough that the condition C4 (add independent questions) is warranted. If the original
20-question average had been below 0.60, the verdict would have been NOT ACCEPTED.

---

## Limitations of this audit

- **Single platform.** All commands ran on one Windows 11 workstation. CI covers Ubuntu, macOS, and
  Windows, but I did not independently verify the other platforms. I relied on the builder's claim of
  CI green at `29638d4` and the PR's CI status.
- **Full workspace `--all-targets` test suite was not completed** due to timeout constraints in the
  sandbox. I ran `cargo test -p zylcode-core --lib` (701 passed, 0 failed) and the benchmark suite (2
  passed, 0 failed), plus targeted context_builder tests (4 passed, 0 failed). The remaining crate
  test suites (zylcode-cli, zylcode-desktop, zylcode-mcp, zylcode-nav) were compiled and checked but
  not all test binaries were executed.
- **The network-probe test** (`probe_classifies_a_refused_connection_as_network_error`) requires
  `NO_PROXY` to be set in this environment. This is an environment artifact, not a code defect, and
  the test passes with the proxy bypass.
- **No live-model end-to-end agent turn was captured.** The AgentLoop integration is proven through
  unit tests (`context_builder_uses_intelligence_ranking`,
  `agent_context_carries_deterministic_graph_citations`) and source inspection, not through a
  captured agent session. This is consistent with the builder's stated limitation.
- **The benchmark question set is builder-authored.** I did not author independent questions. My
  falsification confirmed the threshold is live and the scope is clean, but the question set's
  coverage is the builder's assertion.
- **I did not audit the rung backfill for the 22 GREEN entries** beyond confirming the registry shape
  gate passes. Those entries are pre-existing and outside Phase 2A's scope.

---

## Reproduction

All commands were run from `/c/Temp/p2a-audit-2026-11-16` (detached HEAD at `c32f3d8`, clean tree).

```bash
# Create the audit worktree
git worktree add --detach /c/Temp/p2a-audit-2026-11-16 c32f3d8
cd /c/Temp/p2a-audit-2026-11-16

# Environment
export MSYS_NO_PATHCONV=1
export CARGO_TARGET_DIR=/c/Users/<user>/AppData/Local/Temp/p2a-audit-target
export NO_PROXY=127.0.0.1,localhost
export no_proxy=127.0.0.1,localhost

# 1. Formatting
cargo fmt --all -- --check                          # EXIT:0

# 2. Compilation
cargo check --workspace --all-targets               # EXIT:0

# 3. Clippy
cargo clippy --workspace --all-targets -- -D warnings  # EXIT:0

# 4. Intelligence unit tests
cargo test -p zylcode-core --lib intelligence::       # 110 passed; 0 failed

# 5. Benchmark (the merge gate)
cargo test -p zylcode-core --test repo_intelligence_benchmark -- --nocapture --test-threads=1
#   Files indexed: 754
#   Average Precision@10: 0.64
#   Average Recall@10: 0.96
#   2 passed; 0 failed

# 6. Workspace lib tests
cargo test -p zylcode-core --lib                     # 701 passed; 0 failed

# 7. Previously failing router test
cargo test -p zylcode-core --lib synthetic_offline_dispatch_returns_parseable_payload
#   1 passed; 0 failed

# 8. Context builder integration tests
cargo test -p zylcode-core --lib context_builder     # 4 passed; 0 failed

# 9. Registry gate
python scripts/check_capability_registry.py          # EXIT:0, 37 valid

# 10. CLI entry point
cargo run -p zylcode-cli -- repo-context "where is should_exclude defined"
#   Ranked results with reasons, top-1 correct

# Falsification: threshold regression
# (temporarily set avg_precision >= 0.99 → test FAILED with 0.65 < 0.99)

# Falsification: scope regression
# (temporarily added "src" to FORBIDDEN_INDEX_COMPONENTS → test FAILED with 287 violations)

# Falsification: nested target/ injection
# (created crates/zylcode-core/target/fake/injected.rs → NOT indexed, count unchanged)
```

---

## Conclusion

Phase 2A's code is sound and the evidence survives independent audit. The headline defect from the
2026-09-16 audit — a scanner that indexed 15K build artifacts on Windows — is fixed. The scope
assertion is live and independently verified. The benchmark thresholds are live and independently
verified to fail on regression. The intelligence pipeline is reachable through two product surfaces
(CLI and AgentLoop). The builder was conservative in its rung claims, leaving 7 entries at R2 that
are now demonstrably R3.

The conditions are about what remains honestly incomplete: TypeScript symbols are not exercised
through the pipeline (C1), the question set is builder-authored (C4), throughput has no
machine-independent budget (C5), and symbol resolution is regex-only (C6). None of these are
fabrications or defects — they are known limitations, declared openly, that should be addressed in
Phase 2B and beyond.

**Verdict: ACCEPTED WITH CONDITIONS.**

Phase 2B may begin. The six conditions (C1–C6) are tracked, not ignored.

---

_Audit performed independently. All findings are reproducible from the Reproduction section above._
