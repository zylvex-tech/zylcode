# Impact Analysis (Phase 2B) — design and plan

**Status:** PROPOSED — one enabling slice (C1) is implemented in this change; the analysis engine is
not built. See [§9](#9-implementation-status).

**Input:** Phase 2A **ACCEPTED WITH CONDITIONS** (2026-11-16). Conditions C1, C4 and C6 are folded
into this plan as work items, not deferred.

---

## 1. The question this phase answers

> **"What would this change touch — and how do you know?"**

Phase 2A can find code by description. That is retrieval. Impact analysis is a different claim: given
a _proposed change_, name the files, contracts, tests and call sites it can break, with the evidence
for each, and say plainly what cannot be determined.

The failure mode to design against is confident guessing: a plausible list of files with no reason
attached. Every affected item must carry a **reason and a strength**, and anything outside the index
must be reported as unknown rather than omitted.

---

## 2. What a caller gets

```rust
pub struct ImpactReport {
    pub change: ChangeSpec,             // what was asked about
    pub directly_affected: Vec<Impact>, // symbol/file named by the change
    pub transitively_affected: Vec<Impact>, // reverse reachability, depth-limited
    pub tests_at_risk: Vec<Impact>,     // tests that exercise affected symbols/files
    pub contracts: Vec<Impact>,         // public signatures, config keys, CLI flags, schemas
    pub unknowns: Vec<String>,          // explicitly *not* determined, with why
    pub confidence: Confidence,         // High / Medium / Low + one-line reason
}

pub struct Impact {
    pub subject: String,     // file id or symbol id
    pub relation: Relation,  // Named | Calls | Imports | CoChanged | TestedBy | DeclaredIn
    pub depth: u8,           // 0 = named by the change itself
    pub reason: String,      // human-readable, cites the edge that produced it
}
```

Rules that make it trustworthy:

1. **No item without a reason.** `reason` names the edge (`imports`, `co-changed 4/6 commits`,
   `test asserts should_exclude`), not the score.
2. **Strength is visible.** A regex-indexed `DEFINITION_INDEXED` edge is weaker than a resolved
   import; the relation says which it is.
3. **Unknowns are mandatory.** Empty `unknowns` on a real query is itself a smell; the CLI prints it.
4. **Depth is bounded and stated.** Transitive reachability stops at a stated depth; nothing claims
   "all callers" when the index only has definitions.

---

## 3. Entry points

- **CLI:** `zylcode impact --file <path>` and `zylcode impact --symbol <name>` (Phase 3B's mission
  executor will consume the same call).
- **Agent:** an `impact` tool intent that returns the same `ImpactReport`, so a mission can cite it.
- **UI:** a panel on the existing Repository Intelligence surface; read-only, evidence shown per row.

The rung target is **R3**: reachable from a product surface, with a captured transcript showing the
report for a change that matters.

---

## 4. Where the data comes from (and where it lies)

| Edge              | Source today                              | Honest limitation                                     |
| ----------------- | ----------------------------------------- | ----------------------------------------------------- |
| Named symbol/file | regex `DEFINITION_INDEXED`                | a macro-generated symbol is invisible                 |
| `Imports`         | regex import scan + package graph         | unresolved re-exports and dynamic imports             |
| `Calls`           | **not implemented** (C6)                  | no reference resolution; today only "symbols in file" |
| `CoChanged`       | git history co-change mining              | correlation, not causation — must be labelled         |
| `TestedBy`        | test-file classification + symbol overlap | a test that exercises code indirectly is missed       |
| `Contracts`       | public visibility + CLI/config scan       | trait-object reachability is heuristic                |

This table is the reason the report carries confidence and unknowns. C6 exists because "Calls" is
currently absent, and a caller list built only from definitions is a partial answer.

---

## 5. Conditions from the Phase 2A verdict, folded in

### C1 — TypeScript symbols in the pipeline (enabling slice, implemented here)

`build_repo_query()` indexed Rust symbols only, while `extract_symbols()` already understood
TypeScript/JavaScript. The extractor was unreachable from any product surface, so
`typescript_symbol_index` was R2. This change indexes all parsed languages and adds a test that
builds a real fixture repository and asserts a TypeScript class **and** interface appear in
`RepoQuery::symbols()`. Impact analysis over a repo whose half is TypeScript cannot be honest while
that is missing.

### C4 — independent benchmark questions

The 28-question retrieval set is builder-authored; 8 were added to raise the average. For impact
analysis the equivalent risk is worse: a builder-authored change-scenario set would test exactly the
cases the builder already handles.

- The acceptance corpus for 2B must be **authored outside the implementation** — owner, auditor, or a
  held-out set written before the feature exists.
- This plan commits to publishing the scenario format and the scoring script **first**, so the
  corpus can be written against a stable contract without seeing the implementation.
- Builder-authored scenarios may appear only as a separate, clearly labelled development set.

### C6 — semantic resolution

Regex extraction yields `DEFINITION_INDEXED` only; `IMPORT_RESOLVED` and `REFERENCE_RESOLVED` are not
implemented, so real callers are unknown.

- Stage 1: resolve module paths within the repo (imports → file ids), which is enough for accurate
  import-closure impact. No parser needed.
- Stage 2: reference resolution per language (tree-sitter or LSP), starting with Rust, then
  TypeScript. This is the only way "who calls this" stops being a guess.
- Until stage 2 lands, the report must say `Calls: not resolved` in `unknowns` rather than imply
  completeness.

---

## 6. Staged delivery

| Slice  | Deliverable                                                                | Evidence it must carry                                                   |
| ------ | -------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| **B1** | `ChangeSpec` parsing (file path, symbol, or diff hunk) + direct lookup     | transcript: named file/symbol resolved, or explicit "not in index"       |
| **B2** | Import-closure impact from the existing graph, depth-bounded, with reasons | fixture repo where a change to a leaf breaks a known dependent           |
| **B3** | `tests_at_risk` from test classification + symbol overlap                  | a change that must flag an existing failing test, verified by running it |
| **B4** | Confidence + unknowns discipline; co-change labelled as correlation        | a query where the honest answer is "unknown"                             |
| **B5** | CLI `zylcode impact`, agent tool, UI panel                                 | captured transcript + UI screenshot                                      |
| **B6** | C6 stage 1 (import resolution), then stage 2 (references)                  | precision/recall on a labelled set                                       |

Each slice is independently shippable, and each must include a negative test: a case where the tool
declines to answer rather than guessing.

---

## 7. Acceptance benchmark

Scenario format (published before implementation):

```json
{
  "id": "impact-001",
  "repo_commit": "<sha>",
  "change": { "kind": "symbol", "symbol": "should_exclude" },
  "expected": {
    "directly_affected": ["crates/zylcode-core/src/intelligence/classifier.rs"],
    "transitively_affected": ["crates/zylcode-core/src/intelligence/scanner.rs"],
    "tests_at_risk": ["crates/zylcode-core/src/intelligence/classifier.rs"],
    "must_not_claim": ["crates/zylcode-core/src/router.rs"]
  }
}
```

Metrics: file-level precision and recall on `directly_affected`, plus a **false-claim rate** on
`must_not_claim` that must be **zero** — a confident wrong file is worse than a missing one. The
corpus is C4's, authored independently; the builder may not tune against it.

---

## 8. Non-goals

- Not a build graph or a packaging tool.
- Not a replacement for running the tests; it prioritises them.
- No cross-repository impact (monorepo boundaries only) and no runtime call graphs.
- No "impact score" without a reason string. A number alone is not an answer.

---

## 9. Implementation status

| Piece                                                    | State                                                                                     |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| C1 — TypeScript/JavaScript symbols in `build_repo_query` | **Implemented** in this change; asserted by `build_repo_query_indexes_typescript_symbols` |
| Import closure (B2)                                      | Not implemented; the package graph and per-file imports already exist to build on         |
| Test-at-risk selection (B3)                              | Not implemented                                                                           |
| Confidence / unknowns discipline (B4)                    | Specified here; not implemented                                                           |
| CLI / agent / UI surface (B5)                            | Not implemented                                                                           |
| C6 semantic resolution                                   | Not implemented; staged in §5                                                             |
| C4 independent corpus                                    | Blocked on an external author; format above is the contract                               |
| Impact engine itself                                     | **Not started.** No `ImpactReport`, no scoring, no acceptance run                         |

Phase 2B is therefore **not** complete, and this document does not claim otherwise. What it does is
fix the contract, the honesty rules, and the three audit conditions to specific slices, so the next
increment cannot quietly drop them.
