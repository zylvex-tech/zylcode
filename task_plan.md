# Phase 2A — Repository Intelligence Foundation
## Execution Plan

### Status: IN PROGRESS
### Current Phase: 0 — Forensic Audit (COMPLETE)

---

## Forensic Audit Summary

### Existing Capabilities Found

| Capability | Location | Quality | Reusable? |
|---|---|---|---|
| **File tree walker** | `context_builder.rs` — `WalkDir`, max_depth 3, basic exclusions | Shallow — no classification, no persistence | YES, as base for scanner |
| **File classification** | `pipeline.rs` — `validate_artifact_kind`, `infer_kind_from_path/content` | Extension-based + content heuristic; covers .rs, .ts/.tsx, .json, .md | YES, extend |
| **Source inference** | `cli.rs` — `infer_source` | Filename/path based only | YES, extend |
| **Workspace benchmark** | `cli.rs` — benchmark subcommand | Walks workspace, classifies files, reports counts | YES, as performance baseline |
| **Git tool** | `real_tools.rs` — `GitTool` | Shells out to `git status/diff/log`; parses changed files | YES, wrap for Change Graph |
| **Search tool** | `real_tools.rs` — `SearchTool` | Shells out to `find`/`grep` (Linux-only, won't work on Windows) | LIMITED — needs Windows support |
| **File system tool** | `real_tools.rs` — `FileSystemTool` | read/write/list via tokio fs | YES |
| **Evidence Ledger** | `ledger.rs` + `sqlite_ledger.rs` + `memory_ledger.rs` | Full CRUD, SQLite persistence, hash chain | YES — but REPO INDEX ≠ LEDGER |
| **Vector cache** | `cache.rs` — `VectorCacheStore` + `mock_embed` | SQLite-backed, cosine similarity, deterministic mock embeddings | AVAILABLE but not primary retrieval |
| **Config watcher** | `mcp/config.rs` — `ConfigWatcher` | `notify` crate, watches config files | YES, pattern for file watching |
| **Hot reload watcher** | `mcp/hot_reload.rs` | `notify` crate, watches plugin files | YES |
| **Agent context** | `agent_protocol.rs` — `AgentContext` | file_tree, relevant_files, git_status, user_goal | EXTEND with repo intelligence |
| **Planner** | `planner.rs` — `IntentPlanner` | Assembles execution plans from intents | WILL CONSUME repo intel |

### Existing Dependencies Available

| Dependency | Version | Purpose |
|---|---|---|
| `walkdir` | 2 | Recursive directory walking |
| `regex` | 1.10 | Pattern matching |
| `serde_json` | 1.0 | JSON serialization |
| `serde_yaml` | 0.9 | YAML parsing (for pnpm-workspace.yaml) |
| `toml` | NOT PRESENT | Need to add for Cargo.toml parsing |
| `rusqlite` | 0.31 | SQLite persistence |
| `notify` | 6.1 | File system watching |
| `sha2` | 0.10 | Hashing |
| `chrono` | 0.4 | Timestamps |
| `uuid` | 1 | Identifiers |
| `git2` | NOT PRESENT | Need to add for native git operations |
| `tree-sitter` | NOT PRESENT | Need to add for AST parsing |
| `tree-sitter-rust` | NOT PRESENT | Need to add |
| `tree-sitter-typescript` | NOT PRESENT | Need to add |

### What Does NOT Exist

1. **No repository model types** — no Repository, Workspace, Package, Symbol, etc.
2. **No manifest parsing** — no Cargo.toml/package.json structured parsing
3. **No symbol extraction** — no AST parsing at all
4. **No dependency graph** — no graph data structure
5. **No git history integration** — only `git status` via shell
6. **No incremental indexing** — no file fingerprinting or invalidation
7. **No structured query API** — only keyword matching in ContextBuilder
8. **No relevance ranking** — only basic keyword intersection
9. **No token-aware assembly** — no budget management
10. **No benchmark suite** — no retrieval quality measurement

---

## Implementation Plan

### Phase 2A.1: Repository Model + Scanner (Sections 1-3)
**Files**: `crates/zylcode-core/src/intelligence/mod.rs`, `types.rs`, `scanner.rs`, `classifier.rs`
**Dependencies to add**: `toml` crate
**Tests**: Unit tests for classification, path handling, exclusion

### Phase 2A.2: Manifest Intelligence (Section 4)
**Files**: `crates/zylcode-core/src/intelligence/manifest.rs`
**Dependencies**: `toml` (for Cargo.toml), `serde_json` (already present for package.json), `serde_yaml` (already present for pnpm-workspace.yaml)
**Tests**: Parse real ZylCode manifests, verify workspace/package discovery

### Phase 2A.3: Symbol Graph (Section 5)
**Files**: `crates/zylcode-core/src/intelligence/symbols.rs`, `rust_symbols.rs`, `ts_symbols.rs`
**Dependencies to add**: `tree-sitter`, `tree-sitter-rust`, `tree-sitter-typescript`
**Tests**: Extract symbols from real ZylCode source files

### Phase 2A.4: Dependency Graph (Section 6)
**Files**: `crates/zylcode-core/src/intelligence/dependency.rs`
**Tests**: Build graph from ZylCode workspace, verify bidirectional queries

### Phase 2A.5: Entry Points + Architecture (Sections 7-8)
**Files**: `crates/zylcode-core/src/intelligence/entry_points.rs`, `architecture.rs`
**Tests**: Detect all ZylCode entry points, verify architectural fingerprint

### Phase 2A.6: Git/Change Intelligence (Section 9)
**Files**: `crates/zylcode-core/src/intelligence/git.rs`
**Dependencies to add**: `git2` (or shell-out initially)
**Tests**: Extract commits, changed files, build change graph

### Phase 2A.7: Persistence + Invalidation (Sections 10-12)
**Files**: `crates/zylcode-core/src/intelligence/store.rs`
**Tests**: Persist, restart, invalidate, verify freshness

### Phase 2A.8: Query API + Context Retrieval (Sections 13-15)
**Files**: `crates/zylcode-core/src/intelligence/query.rs`, `context.rs`
**Tests**: All query methods, relevance ranking, token budgeting

### Phase 2A.9: Benchmark + Performance (Sections 16-17)
**Files**: `crates/zylcode-core/tests/repo_intelligence_benchmark.rs`
**Tests**: Known-answer benchmark against ZylCode itself

### Phase 2A.10: Security, Registry, Docs (Sections 18-22)
**Files**: Tests, capability-registry.json, completion report

### Phase 2A.11: Acceptance Demo + Delivery (Sections 23-28)
**Verification**: Full test suite, fmt, clippy, commit, push

---

## Key Design Decisions

1. **Repository Index ≠ Evidence Ledger**: Large structural data goes in the repo index (SQLite). Only evidence-bearing discoveries go in the Evidence Ledger.

2. **Incremental by default**: Every indexed item gets a content hash. On re-scan, only changed items are reindexed.

3. **Fail-closed semantics**: Unsupported language → UNSUPPORTED_SEMANTIC_INDEX. Parser failure → PARSE_FAILED. Stale → STALE.

4. **Provenance on every fact**: OBSERVED / PARSED / DERIVED / INFERRED classification.

5. **Benchmark is the feature**: Known-answer questions with Precision@K / Recall@K metrics.

---

## Next Step
Begin Phase 2A.1: Create the intelligence module structure and scanner.
