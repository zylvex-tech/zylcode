# Phase 2B — Delivery Plan (Repository Reasoning & Impact Analysis)

> **STATUS: NOT STARTED. PLANNING ARTEFACT ONLY.**
> Phase 2B is blocked until Phase 2A is formally accepted by the independent auditor. Nothing in
> this document authorises implementation, and no Phase 2B code exists on any branch. Merging this
> file records the scope; it does not begin the phase.

- **Date:** 2026-11-11
- **Gate:** Phase 2A `ACCEPTED` (verdict owner: independent auditor, not the builder)
- **Authoritative sources re-read for this plan:**
  - `docs/roadmap/ZYLCODE_ROADMAP_V2.md` §Phase 2B (deliverables, entry points, benchmark, rung)
  - `docs/architecture/INTELLIGENCE_GRAPH.md` §6.1–6.8 (reasoning layer), §7 (benchmark)
  - `docs/architecture/AGENT_KERNEL.md` §4.2–4.3 (required context assembly, why it is not cosmetic)
  - `docs/PROJECT_MASTER_STATUS.md` §6 (near-term ordering)
  - `docs/governance/ZYLCODE_CAPABILITY_MODEL.md` §4–§6 (status/rung computation)
- **Blocks on completion:** Phase 3A and everything downstream.

---

## 1. Objective

Turn the accepted repository index into **actionable engineering intelligence**: given a change, a
symbol or a task, produce a ranked, reasoned, verified answer about what is affected and what
context is sufficient — reachable from the agent loop and the CLI.

Reasoning over a mis-scoped index yields confidently wrong answers, which is why Phase 2A had to be
correct first. This plan assumes the accepted 2A index and does not re-litigate it.

---

## 2. Definition of done

The phase closes when every row below has a committed entry point, a passing acceptance test, a
recorded evidence artefact and an independent review verdict. Rows are traced to the roadmap's
Phase 2B deliverable list.

| #   | Deliverable (roadmap)            | Work package | Accepted when                                                                                                                                                                     |
| --- | -------------------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Definition/reference traversal   | WP1, WP2     | `IMPORT_RESOLVED` and `REFERENCE_RESOLVED` are produced by a real resolver, and the honesty markers that currently say "not implemented" are retired **only** where they are true |
| D2  | Test-to-code relationships       | WP3          | Bidirectional query answers "which tests exercise X" and "what does test T cover"; Q5 and Q6 are non-zero                                                                         |
| D3  | Impact analysis                  | WP4          | `impact(symbol_or_file)` returns a transitively affected set, ranked, **with a reason per result**                                                                                |
| D4  | Change risk                      | WP5          | A risk signal per change **with its contributing factors**, never a bare number                                                                                                   |
| D5  | Architectural boundary detection | WP6          | Intended module/layer boundaries identified from manifest structure plus observed dependency direction; violations flagged with provenance                                        |
| D6  | Task→context retrieval           | WP7          | Minimum sufficient working set for a task, token-aware, ranked with reasons                                                                                                       |
| D7  | Incremental context assembly     | WP7          | Context updates as a task evolves without a full re-scan                                                                                                                          |
| D8  | Repository diagnostics           | WP7          | Index health exposed: coverage, staleness, unsupported files, parse failures; fail-closed status codes                                                                            |
| D9  | Benchmark suite                  | WP8          | P@10 ≥ 0.60 and R@10 ≥ 0.60 on ≥ 20 known-answer questions; **a keyword baseline must fail**; deterministic across runs                                                           |
| D10 | Agent integration                | WP8          | Reachable from `AgentLoop::assemble_context()` **and** the CLI                                                                                                                    |

**Rung target:** R3 minimum, R4 preferred. No deliverable may be reported above R2 until it has a
named committed entry point and a reproduction block, per `ZYLCODE_CAPABILITY_MODEL.md` §6.

---

## 3. Work packages

Each package lists expected behaviour, owner role, acceptance test, evidence artefact and
dependencies. Owners are roles, assigned at start of work, not names invented now.

### WP1 — Semantic resolution

- **Expected behaviour:** imports and references resolved beyond regex, using tree-sitter or an LSP
  client, with per-file failure isolation.
- **Owner:** core-intelligence engineer.
- **Acceptance:** on the repository itself, `IMPORT_RESOLVED` coverage is measured and reported as
  a percentage; unresolvable files fail closed with `UNSUPPORTED_SEMANTIC_INDEX` / `PARSE_FAILED`
  rather than silently degrading.
- **Evidence:** committed transcript with command, environment, raw counts, commit SHA.
- **Depends on:** 2A accepted; no other package.

### WP2 — Definition / reference traversal

- **Expected behaviour:** go-to-definition and find-references across the graph, with result
  provenance distinguishing definition from mention.
- **Owner:** core-intelligence engineer.
- **Acceptance:** for a set of hand-verified symbols, definition lookup returns the defining
  location; reference lookup returns all call sites with no duplicates and no build output.
- **Evidence:** fixture table of symbol → expected locations, plus a live run.
- **Depends on:** WP1.

### WP3 — Test-to-code relationships

- **Expected behaviour:** bidirectional mapping between tests and the code they exercise, derived
  from name conventions **and** observed imports/references.
- **Owner:** core-intelligence engineer.
- **Acceptance:** Q5 (test discovery) and Q6 (reverse dependency) score above zero and are asserted
  in the gate; a deliberately renamed test is still attributed by its imports.
- **Evidence:** benchmark per-question output showing both non-zero; a negative fixture proving the
  gate fails if either returns to zero.
- **Depends on:** WP1.

### WP4 — Impact analysis

- **Expected behaviour:** `impact(symbol | file)` traverses the dependency graph transitively and
  ranks by distance, reference strength (definition vs mention), test relationship and change
  recency. Every result carries a `reason`.
- **Owner:** reasoning engineer.
- **Acceptance:** for a change to a core type, the returned set includes its direct dependents,
  its tests and the entry points that reach it, ordered sensibly; a result without `reason` is
  impossible by construction (type-level guarantee, not convention).
- **Evidence:** committed transcript for two real symbols and one file; a test asserting every
  result has a non-empty reason.
- **Depends on:** WP2, WP3.

### WP5 — Change risk

- **Expected behaviour:** per-change risk signal with the factors that produced it (fan-in, test
  coverage presence, boundary crossing, churn, unsupported-language files touched).
- **Owner:** reasoning engineer.
- **Acceptance:** two changes of visibly different blast radius produce different signals, and the
  factors are inspectable; no factor is invented when evidence is absent (it is reported as
  unknown, not zero).
- **Evidence:** a table of N changes with factors and the resulting signal, plus the test that
  recomputes it.
- **Depends on:** WP4, WP6.

### WP6 — Architectural boundary detection

- **Expected behaviour:** infer intended module/layer boundaries from manifest structure and
  observed dependency direction; flag violations (e.g. a crate reaching upward) with provenance.
- **Owner:** architecture engineer.
- **Acceptance:** the known layering of this workspace is recovered without hand-authored rules;
  at least one seeded violation is detected and one legitimate edge is not.
- **Evidence:** boundary map plus detection transcript, including the deliberately seeded case.
- **Depends on:** WP1.

### WP7 — Task context, incrementality and diagnostics

- **Expected behaviour:** assemble the minimum sufficient working set for a task, token-aware;
  update it incrementally as the task evolves without a full re-scan; expose index health.
- **Owner:** reasoning engineer.
- **Acceptance:** a task that touches two packages yields a working set under a stated token
  budget containing the files a human needed; a mid-task edit updates context without re-indexing
  the repository; diagnostics report coverage, staleness and unsupported files with fail-closed
  codes.
- **Evidence:** transcript for the task run, the incremental update, and the diagnostics output.
- **Depends on:** WP4, WP5.

### WP8 — Integration and benchmark

- **Expected behaviour:** `AgentLoop::assemble_context()` consumes the reasoning layer; CLI exposes
  `zylcode repo impact <symbol>` and keeps `zylcode repo context <task>`; the benchmark grows to
  ≥ 20 questions and asserts a keyword baseline fails.
- **Owner:** platform engineer, with the intelligence owner for the benchmark corpus.
- **Acceptance:** the agent turn demonstrably uses ranked, reasoned context (not the file-tree
  walker); CLI commands return the same answers as the agent path; P@10 and R@10 ≥ 0.60 asserted;
  the `grep` baseline is measured and fails the same bar; two consecutive runs are byte-identical.
- **Evidence:** committed transcripts for both entry points, the benchmark dump with per-question
  scores, and the keyword-baseline comparison.
- **Depends on:** WP4–WP7.

---

## 4. Controls

| Control                       | Concrete artefact in this plan                                                                                                             |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Requirements inventory        | §2 table (D1–D10), each traced to the roadmap's Phase 2B list                                                                              |
| Implementation ownership      | Owner role per work package in §3; one package per PR                                                                                      |
| Acceptance tests              | Acceptance criterion + evidence artefact per package; asserted in CI where automatable                                                     |
| Security and regression gates | Existing CI (3-OS, clippy, fmt, audit, secret scan, retracted claims, registry gate) stays required; new graph features must not weaken it |     | Evidence and closure register | The requirement-to-evidence matrix in §4.1; a deliverable is closed only by an independent review, never by its author |

### 4.1 Requirement-to-evidence matrix

This is the control that prevents the phase ending with substantial implementation and incomplete
acceptance evidence. Every deliverable must fill every column; an empty cell blocks closure, no
matter how much code landed.

Cells marked _to be named at WP start_ are deliberately not invented here: naming a module or test
before the work package begins would be a placeholder presented as a plan.

| ID / requirement (spec reference)                                                                   | Implementation surface (target)                                                                                                   | Automated acceptance test                                                                                                         | Integration demonstration                                                | Performance and security evidence                                                                                     | Acceptance owner                            |
| --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- |
| **D1** Semantic resolution — `IMPORT_RESOLVED`, `REFERENCE_RESOLVED` (`INTELLIGENCE_GRAPH.md` §6.2) | `crates/zylcode-core/src/intelligence/symbols.rs` (existing), new resolver module under `intelligence/` (name fixed at WP1 start) | Resolution-coverage test reporting a measured percentage; unresolvable input yields `UNSUPPORTED_SEMANTIC_INDEX` / `PARSE_FAILED` | CLI run on this repository with the coverage figure in the output        | Resolution cost per file (time and memory); no new network egress; no secret-file reads (2A exclusion still enforced) | Independent reviewer for WP1; G2B-5 auditor |
| **D2** Test-to-code relationships (`roadmap` 2B row 3; graph §6.3)                                  | `intelligence/` test-graph module + query surface (to be named at WP3 start)                                                      | Q5 and Q6 asserted non-zero in the merge gate, plus a negative fixture proving the gate fails if either returns to zero           | `zylcode repo impact` transcript showing test fan-out for a real symbol  | Cost of the join across symbols/tests; no weakening of existing CI gates                                              | Independent reviewer for WP3; G2B-5         |
| **D3** Impact analysis, ranked with a reason per result (`roadmap` row 1; graph §6.1)               | `intelligence/` impact module; ranking factors from dependency, reference, test and change graphs                                 | Test asserting **every** result carries a non-empty `reason`; transitive-set test on two hand-verified symbols                    | `zylcode repo impact <symbol>` transcript, plus agent-path equivalence   | p95 latency for a symbol with high fan-in, with the fixture size stated; read-only, no new tool risk                  | Independent reviewer for WP4; G2B-5         |
| **D4** Change risk with contributing factors (`roadmap` row 4; graph §6.4)                          | `intelligence/` risk module consuming D3 output                                                                                   | Two changes of different blast radius produce different signals; absent evidence reported as unknown, never as zero               | Risk output for a real pending change, factors shown                     | Determinism of the signal across two runs; no PII or secret content read                                              | Independent reviewer for WP5; G2B-5         |
| **D5** Architectural boundary detection (`roadmap` row 5; graph §6.5)                               | `intelligence/architecture.rs` (existing) extended with boundary inference                                                        | Seeded-violation test: one planted upward dependency flagged, one legitimate edge not flagged                                     | Boundary map printed for this workspace                                  | Inference cost; provenance recorded for every boundary and violation                                                  | Independent reviewer for WP6; G2B-5         |
| **D6** Task→context retrieval, token-aware (`roadmap` row 6; graph §6.6)                            | `context_builder.rs` (existing seam) + retrieval module                                                                           | Working-set test: a two-package task yields the human-needed files under a stated token budget                                    | `zylcode repo context "<task>"` transcript                               | Token count and latency recorded per query; no outbound calls                                                         | Independent reviewer for WP7; G2B-5         |
| **D7** Incremental context assembly (`roadmap` row 7; graph §6.6)                                   | `context_builder.rs` + index store invalidation path                                                                              | Mid-task edit updates context **without** a full re-scan, proven by an index-pass counter                                         | Transcript of an evolving task showing incremental updates               | Re-scan avoided measured in files hashed; cache correctness on edit                                                   | Independent reviewer for WP7; G2B-5         |
| **D8** Repository diagnostics (`roadmap` row 8; graph §6.7)                                         | `intelligence/` diagnostics surface, CLI-exposed                                                                                  | Tests for coverage, staleness, unsupported and parse-failure states, including the three fail-closed codes                        | `zylcode repo diagnostics`-style transcript (command fixed at WP7 start) | Diagnostics cost; no secret content in output                                                                         | Independent reviewer for WP7; G2B-5         |
| **D9** Benchmark, grep-resistant (`roadmap` row 9; graph §7)                                        | `crates/zylcode-core/tests/repo_intelligence_benchmark.rs` (existing) extended to ≥ 20 questions                                  | P@10 ≥ 0.60 and R@10 ≥ 0.60 asserted; Q5/Q6 non-zero; keyword baseline measured and failing the same bar                          | Benchmark dump with per-question scores, committed as evidence           | Two consecutive runs byte-identical; runtime recorded, perf not gating correctness                                    | Independent reviewer for WP8; G2B-5         |
| **D10** Agent and CLI agreement (`roadmap` row 10; graph §6.8; `AGENT_KERNEL.md` §4.2)              | `AgentLoop::assemble_context` + `crates/zylcode-cli/src/main.rs`                                                                  | Test asserting the agent path and CLI return the same ranked, reasoned result for the same query                                  | Paired transcripts: one agent turn, one CLI run                          | Agent turn latency delta recorded; no privilege or tool-scope expansion                                               | Independent reviewer for WP8; G2B-5         |

**Closure rule.** A row is complete only when the implementation cell names real files, the test
cell names a test that exists and runs in CI, the demonstration cell links a committed transcript
with its commit SHA, and the evidence cells contain measurements rather than intentions. A row with
any empty or "to be named" cell is `IN PROGRESS`, never closed.

---

## 5. Sequencing and gates

| Step | Content                                | Gate                                                                     |
| ---- | -------------------------------------- | ------------------------------------------------------------------------ |
| 0    | Phase 2A verdict                       | `ACCEPTED` — nothing below starts before this                            |
| 1    | WP1, WP2, WP6 (resolution + traversal) | Gate G2B-1: resolution coverage measured, traversal tests green          |
| 2    | WP3, WP4 (tests + impact)              | Gate G2B-2: Q5/Q6 non-zero and impact reasons type-enforced              |
| 3    | WP5, WP7 (risk + context/diagnostics)  | Gate G2B-3: risk factors inspectable; incremental update proven          |
| 4    | WP8 (integration + benchmark)          | Gate G2B-4: agent and CLI agree; benchmark green; keyword baseline fails |
| 5    | Independent Phase 2B acceptance        | Gate G2B-5: final evidence review, blocker disposition, closure          |

Each gate requires: tests green, a committed reproduction artefact, and an independent review of
that work package. A red gate stops the sequence; it does not get waived.

---

## 6. Non-goals and standing rules

- **No 3A work.** The project system starts only after 2B is accepted.
- **No reopening accepted 2A items** without new evidence of a regression. A regression is claimed
  with a failing test or a measured contradiction, not an opinion.
- **No semantic-resolution claims before the resolver exists.** The current honesty markers in
  `symbols.rs` stay until they are true.
- **No self-acceptance.** A builder may report READY FOR REVIEW; only an independent reviewer or
  auditor closes a gate.
- **No green-CI equivalence.** Passing checks demonstrate the tests pass; they do not demonstrate
  the deliverable is correct.
- **No metric that a keyword baseline could pass.** The benchmark must remain grep-resistant.

---

## 7. Risks

| Risk                                                              | Mitigation                                                                                                                    |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Tree-sitter/LSP integration drags beyond the phase                | WP1 is scoped to measured coverage on this repository; partial coverage is reported as coverage, not as semantic completeness |
| Benchmark over-fitting to builder-authored questions              | Keyword baseline comparison plus an independent reviewer's sample of unseen questions                                         |
| Reasoning layer repeats the 2A "library without a surface" defect | WP8 is a hard gate: no package is accepted without an entry point and a transcript                                            |
| Index staleness silently degrading answers                        | WP7 diagnostics expose staleness; fail-closed status codes are tested                                                         |
| Scope creep into 3A concepts (projects, missions)                 | Non-goals section; any 3A idea is recorded and deferred, not implemented                                                      |

---

## 8. What "closed" means

Phase 2B is closed when an independent auditor records, against a named commit:

1. each of D1–D10 either accepted or explicitly deferred with a reason;
2. the benchmark reproduced independently, including the keyword-baseline comparison;
3. both entry points invoked, with transcripts;
4. the registry updated so that every claimed capability carries a `rung` and its basis;
5. any remaining limitation classified as blocking or non-blocking, with an owner.

Until then the phase is `IN PROGRESS` at best, and `BLOCKED` today.
