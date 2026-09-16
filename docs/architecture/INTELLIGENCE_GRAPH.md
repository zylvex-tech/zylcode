# INTELLIGENCE_GRAPH.md — ZylCode Intelligence Graph

**Status: PARTIAL — implemented at R2, benchmark failing, not integrated**
**Rung: R2 (EXECUTED)**
**Phases: 2A (re-opened) · 2B (blocked)**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §3 · `PHASE2A_INDEPENDENT_AUDIT.md`

> ⚠️ **Read the Phase 2A audit before this document.**
> The module exists and is unit-tested. It is **not** integrated into any product surface, its
> headline metric was mis-scoped, and its benchmark fails on independent re-execution.
> It is **R2**: a library, not a capability.

---

## 1. Responsibility

Repository, symbol, dependency, architecture, history and runtime knowledge — expressed as a
structured, queryable model so that an agent can determine:

- what the repository **is**
- how it is **structured**
- what **owns** what
- what **depends** on what
- where execution **begins**
- where **tests** live
- how components **build / run / test**
- which symbols and files are **relevant** to a task
- what **changed** recently
- **why** retrieved context is relevant

---

## 2. What Exists Today

```
crates/zylcode-core/src/intelligence/
├── mod.rs             module root
├── types.rs           canonical types
├── classifier.rs      language + role classification
├── scanner.rs         deterministic repository scanner
├── manifest.rs        Cargo.toml / package.json / pnpm-workspace.yaml
├── symbols.rs         regex symbol extraction (Rust, TS/JS)
├── dependency.rs      bidirectional dependency graph
├── entry_points.rs    entry-point discovery from manifest evidence
├── architecture.rs    architectural fingerprint with provenance
├── git.rs             git / change intelligence
├── store.rs           SQLite persistence
├── query.rs           typed query API (14 methods)
└── context.rs         context retrieval with relevance ranking
```

**79 unit tests pass.** Symbol count (1,444), package count (5), dependency edges (4) and entry
points (15) all reproduce.

---

## 3. Defects (from the audit)

### 3.1 The index measures build output

```
indexed files:            15,064
files under target/:      14,900   (98.9%)
repository source files:     254
git-tracked files:           193
```

**Root cause:** `should_exclude()` matches forward-slash patterns (`/target/`, `node_modules/`)
against `path.to_string_lossy()`. On Windows this yields backslashes, so the exclusion list is
**dead code on the primary target platform**. The `.gitignore` fallback does not rescue it
because the real patterns are `/target` and `**/target`, neither of which the hand-rolled matcher
handles.

**Why tests were green:** the scanner's own test writes a fixture `.gitignore` containing
`target/` — a bare directory pattern the naive matcher *does* handle. **The fixture differed from
reality, so a green suite certified a broken scanner.**

### 3.2 The benchmark does not reproduce

```
Scan should complete in < 120s, took 276.1043943s   → FAILED
```

Reported: 12.7s. Measured: 276.1s. 22× slower, and the assertion fails.

Additionally, the suite asserts only `file_count() > 50`, `symbol_count() > 20`,
`package_count() >= 3`. **A scan indexing 51 files would print "PASS".** None of the headline
numbers are asserted, so none of them are protected against regression.

### 3.3 Zero integration

```
grep -rn "intelligence::" --include=*.rs crates/ apps/ \
  | grep -v "src/intelligence/" \
  | grep -v "tests/repo_intelligence_benchmark.rs"
→ (no output)
```

No CLI command. No UI surface. No agent tool. `AgentLoop` still uses the older `ContextBuilder`.

**By the reachability test (`ZYLCODE_PROOF_GRAPH.md` §2.1), this is R2.**

---

## 4. What Must Be Preserved

The audit is a rejection of the *evidence*, not the *design*. These are sound:

1. **Module decomposition** — clean layering, no circular coupling:
   `types → classifier → scanner → manifest → symbols → dependency → entry_points → architecture → git → store → query → context`
2. **Canonical types** — `Repository`, `Workspace`, `Package`, `FileNode`, `Symbol`,
   `Dependency`, `EntryPoint`, `ArchitecturalFingerprint`, `GitCommit`, `ContextResult`.
3. **Provenance modelling** — `Provenance::{Observed, Parsed, Inferred}` plus
   `ContextResult.reason`. **Every retrieval can explain why.** This is the seed of the Project
   Knowledge Graph and should be protected.
4. **Honest support levels** — `DEFINITION_INDEXED` vs `IMPORT_RESOLVED` vs `REFERENCE_RESOLVED`.
   `symbols.rs` states outright: *"Architecture MUST NOT pretend regex extraction is semantic
   indexing."* Keep that discipline.
5. **Security boundaries** — secret-file exclusion, `follow_symlinks: false`, repo-root-relative
   paths, local-first with no egress.

---

## 5. Required Fixes (Phase 2A remediation)

### P0.1 — Path exclusion

```rust
// WRONG — dead on Windows
let excluded = ["/target/", "/node_modules/", ...];
if path_str.contains(exc) { return true; }

// RIGHT — normalise first, then match on components
let normalised = path.to_string_lossy().replace('\\', "/");
if normalised.split('/').any(|c| EXCLUDED_DIRS.contains(&c)) { return true; }
```

Replace the hand-rolled gitignore matcher with the **`ignore`** crate (same author as `walkdir`,
used by ripgrep). Then **test against the real repository `.gitignore`**, not a synthetic one.

### P0.2 — Re-baseline every metric

Expected genuine index size: **~254 files**. Every figure in the completion report and the
capability registry must be corrected, and the corrections must be **visible as corrections**.

### P0.3 — Deterministic, self-enforcing benchmark

- Assert `files_indexed` within a tolerance band of the real count.
- Assert symbol count.
- Assert index scope: **no indexed path may contain `target/`, `node_modules/`, or `.git/`**.
- Split **performance** from **quality** so a slow machine does not red-flag correctness.

### P0.4 — Machine-independent timing

Express the budget as **files/second**, and parallelise the scan. Hashing 15K files serially is
the bottleneck. A threshold that passes on the author's machine and fails on a reviewer's is not
a threshold.

### P0.5 — Real acceptance demonstration

A committed transcript: command, environment, raw stdout, commit SHA. No "Expected Output".

---

## 6. Phase 2B — Reasoning Layer

Once 2A is re-accepted, the graph becomes actionable.

### 6.1 Impact analysis

```
impact(symbol | file) → ranked affected set
```

Traversal over the dependency graph with ranking by:
- distance
- reference strength (definition vs. mention)
- test relationships
- recency of change

**Every result carries a `reason`.** A ranked list without reasons is a guess with formatting.

### 6.2 Definition / reference traversal

Requires semantic resolution beyond regex:

| Level | Method | Phase |
|---|---|---|
| `DEFINITION_INDEXED` | regex | 2A ✔ |
| `IMPORT_RESOLVED` | tree-sitter / compiler-assisted | 2B |
| `REFERENCE_RESOLVED` | tree-sitter / LSP | 2B |

**Do not claim semantic resolution before it exists.** The existing honesty in `symbols.rs`
about this must be preserved.

### 6.3 Test-to-code relationships

Bidirectional mapping. This is what makes Q5 (test file discovery) and Q6 (reverse dependency)
non-zero in the benchmark — both currently score 0.00.

### 6.4 Change risk

A risk signal per change, **with the factors that produced it**. Not a bare number.

### 6.5 Architectural boundary detection

Identify intended module/layer boundaries and flag violations. Boundaries are inferred from
manifest structure plus observed dependency direction, and recorded with provenance.

### 6.6 Task → context retrieval

Given a task, assemble the **minimum sufficient** working set:

- token-aware
- ranked with reasons
- incremental (updated as the task evolves, without a full re-scan)

### 6.7 Repository diagnostics

Health of the index itself: coverage, staleness, unsupported files, parse failures. Fail closed —
unsupported input returns `UNSUPPORTED_SEMANTIC_INDEX`, parser failure returns `PARSE_FAILED`,
stale data returns `STALE`.

### 6.8 Agent integration — **mandatory**

Wire into `AgentLoop::assemble_context()` and expose a CLI entry point. Without this, 2B
reproduces the exact defect the audit identified.

---

## 7. Benchmark (Phase 2B)

| Metric | Phase 2A reported | Phase 2B required |
|---|---|---|
| Precision@10 | 0.30 | **≥ 0.60** |
| Recall@10 | 0.53 | **≥ 0.60** |
| Questions scoring 0.00 | 3 of 10 (Q5, Q6, Q10) | **0 of 20** |
| Questions | 10 | **≥ 20** |
| Index scope assertion | none | **asserted** |
| Reproducibility | not reproducible | **deterministic** |

**Rationale:** the 2B bar must be one a **keyword search would fail**. Otherwise the graph adds
nothing over `grep`.

---

## 8. Relationship to Other Systems

| System | Relationship |
|---|---|
| **Agent Kernel** | primary consumer; must assemble context through the graph |
| **Project System** | scopes the graph; owns project identity |
| **Project Knowledge Graph** | extends the graph with decisions and runtime evidence |
| **Vision Studio** | consumes symbols/dependencies to map design↔code |
| **Proof Engine** | consumes test-to-code relationships to select verification |

### 8.1 The index is not the ledger

> **An index may be rebuilt. A ledger may not.**

The Intelligence Graph index is a *derived* artifact. Rebuilding it must not appear to rewrite
history. Index and Ledger are separate stores with separate APIs. This distinction is
architectural and must not be collapsed for convenience.

---

## 9. Anti-Requirements

- Do not claim semantic resolution that does not exist.
- Do not publish an index size without naming its scope.
- Do not treat an index as a ledger.
- Do not build reasoning on an index whose correctness is unverified.
- Do not accept a green benchmark that has not been re-run.
