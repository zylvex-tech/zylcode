# REPOSITORY INTELLIGENCE WAVE — 2026-09-28

**Program:** ZYLVEX TECHNOLOGIES LTD / ZYLCODE
**Authority:** Next Execution Wave work order — _Repository Intelligence: symbol references,
import graph, dependency graph & evidence-backed code navigation_.
**Repository:** `C:\Projects\zylcode`
**Status vocabulary (Wave §15):** `NOT_IMPLEMENTED` · `IMPLEMENTED` · `TESTED` · `UI_WIRED` ·
`RUNTIME_VERIFIED` · `EVIDENCE_RECORDED` · `ACCEPTED` · `BLOCKED` · `UNVERIFIED`

> **Deterministic first.** Every graph fact in this wave must be produced by a parser, an AST, a
> language service, an index, compiler metadata, package metadata, or an explicit source
> relationship. Text search may be used only as a fallback and must be **labelled** as such.
> **The AI may explain a graph. It must not invent the graph.**

**Commit / no-commit rule:** this document and the wave's code are **NOT COMMITTED**. The work
tree is left for owner review.

**Remediation pass — 2026-09-29.** A single narrow authorised pass fixed §16's **G11**, **G3**
and **G2**, in that order, and commissioned each one. Read with: §14 (gates re-run from
scratch), §15.4 (safety re-verification, incl. the one extra modified file), §15.5b (G11 +
journey K), §15.6 journey L (G3), §15.7 (G2 + journey M), then §16 and §17. Nothing else was
touched: G4 stays `HISTORICAL_RECORD` and G6 stays closed.

---

## PART 1 — BASELINE (recorded before implementation)

### 1.1 Safety envelope

Checkpoint observed at wave start (§0):

```
$ git log --oneline -1
406753f docs(audit): record the git checkpoint in the live commissioning report
```

Tracked state:

```
$ git status --porcelain=v1 -b
## main...origin/main
```

The tracked tree is **clean** and `main` is level with `origin/main`. Nothing in this wave performs
`reset`, `clean`, `rebase`, force-checkout, stash drop/pop, history rewrite, `commit`, `push`, or
force-push.

Deliberately preserved untracked files — **left untouched by this wave** (§0):

| Path                                                   | Disposition                          |
| ------------------------------------------------------ | ------------------------------------ |
| `.git-msg.txt`                                         | preserved, not absorbed, not deleted |
| `COMPLETE_AUDIT_REPORT.html`                           | preserved                            |
| `crates/zylcode-core/src/project/filesystem.rs.backup` | preserved                            |
| `docs/governance/RECONCILIATION_FORENSIC_REPORT.md`    | preserved                            |
| `tasks/` (`plan.md`, `todo.md`)                        | preserved                            |

Prior integrated work referenced by §0 and present in history: `6b50a8b`, `93eb3d2`, `58dce7`,
`f9ba68b`.

### 1.2 Repository shape (measured)

```
$ git members (Cargo.toml)
crates/zylcode-core, crates/zylcode-mcp, crates/zylcode-cli, apps/zylcode-desktop/src-tauri

$ source file counts (excluding target/ and node_modules/)
rs: 111
ts/tsx: 80
Cargo.toml: 25
package.json: 10
```

Representative repository size for performance work: **~111 Rust + ~80 TS/TSX files.**

### 1.3 Dependency direction (this drives the whole design)

```
zylcode-core ──depends on──▶ zylcode-mcp
zylcode-mcp   ──does NOT depend on──▶ zylcode-core
```

Evidence: `crates/zylcode-mcp/Cargo.toml` lists no `zylcode-core` dependency;
`crates/zylcode-core/Cargo.toml` lists `zylcode-mcp = { workspace = true }`.

Consequence: **any tool that must run through the existing executor architecture
(`real_tools::dispatch` → permission gate → evidence record) and that needs repository-intelligence
data cannot be implemented inside `zylcode-mcp` against `zylcode-core`.** §12 of the work order
requires following the existing tool/executor architecture, so the deterministic navigation engine
must live in a crate that `zylcode-mcp` can depend on. See §2.1.

### 1.4 Existing repository-intelligence work

All of it lives in `crates/zylcode-core/src/intelligence/` (16 modules):

| Module                                                                                | What it does today                                                                            |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| `scanner.rs`                                                                          | gitignore-aware walk, classification, per-file SHA-256 content hash                           |
| `classifier.rs`                                                                       | language + file-role classification, secret exclusion                                         |
| `manifest.rs`                                                                         | Cargo.toml / package.json / pnpm-workspace parsing, internal dep resolution                   |
| `symbols.rs`                                                                          | **regex-based** symbol extraction for Rust and TS/JS                                          |
| `dependency.rs`                                                                       | package→package graph (bidirectional, transitive) + **file-level import maps**                |
| `entry_points.rs`, `architecture.rs`, `git.rs`, `canvas.rs`, `context.rs`, `query.rs` | entry points, architectural fingerprint, git history, ranked retrieval                        |
| `store.rs`, `persisted.rs`                                                            | in-memory store; content-hash-keyed persisted index cache                                     |
| `types.rs`                                                                            | canonical types incl. `Provenance`, `LanguageSupport`                                         |
| `api.rs`                                                                              | `repo_intel_payload()` — one payload, two consumers (CLI `serve-intel`, Tauri `repo_context`) |

**Honesty note carried forward from the existing code.** `symbols.rs` states in its own module
docs:

> Phase 2A uses regex-based extraction … Architecture MUST NOT pretend regex extraction is
> semantic indexing.

Every symbol it produces carries `support_level: LanguageSupport::Parsed`.

### 1.5 Baseline gaps (what the wave must close)

Recorded by source inspection, before any implementation:

| #   | Capability                                                                   | Baseline state                                                                                                                                                                                                                                                         | Status                           |
| --- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| G1  | **Symbol identity**                                                          | `Symbol { id, name, kind, file, line, end_line, visibility, parent, language, support_level }`. `id` is `file_id.replace('/', "::") + "::" + name`. No workspace, no container/module path, no signature, no byte range, `end_line` always `None`.                     | `NOT_IMPLEMENTED`                |
| G2  | **Find References**                                                          | No implementation. `RepoQuery::find_symbol()` returns _definitions_ by name substring only.                                                                                                                                                                            | `NOT_IMPLEMENTED`                |
| G3  | **Callers / callees**                                                        | No implementation anywhere. No call-site extraction.                                                                                                                                                                                                                   | `NOT_IMPLEMENTED`                |
| G4  | **Imports (file → file)**                                                    | `DependencyGraph::{add_file_import, files_imported_by, files_that_import}` **exist but are never called** — `grep add_file_import` returns only their definitions in `dependency.rs`. The file-level maps are always empty.                                            | `NOT_IMPLEMENTED`                |
| G5  | **Exports**                                                                  | No export extraction in either language.                                                                                                                                                                                                                               | `NOT_IMPLEMENTED`                |
| G6  | **Module dependencies**                                                      | Package→package edges only (from `Cargo.toml`/`package.json`). No module/file edge ever populated.                                                                                                                                                                     | `NOT_IMPLEMENTED`                |
| G7  | **Provenance-carrying edges**                                                | `Provenance` enum exists (`Observed/Parsed/Derived/Inferred`) and is used for _query results_, but no graph **edge** carries provenance — `DependencyGraph` stores bare `HashSet<String>`.                                                                             | `NOT_IMPLEMENTED`                |
| G8  | **Impact analysis**                                                          | No implementation. `transitive_dependents_of()` exists for **packages** only, with no `DIRECT/TRANSITIVE/POSSIBLE_TEXTUAL/UNRESOLVED` classification.                                                                                                                  | `NOT_IMPLEMENTED`                |
| G9  | **TS/JS symbol indexing**                                                    | `build_repo_query()` extracts symbols only `if file.language == Language::Rust`. TypeScript/JavaScript symbols are never indexed despite `extract_ts_symbols()` existing.                                                                                              | `NOT_IMPLEMENTED`                |
| G10 | **AST / parser**                                                             | No `tree-sitter`, `tower-lsp`, `lsp-types` or equivalent in any `Cargo.toml` (`grep` over `*.toml` → no matches). Regex only.                                                                                                                                          | `NOT_IMPLEMENTED`                |
| G11 | **Incremental indexing**                                                     | Whole-index cache keyed on a repo-wide content fingerprint (`persisted.rs`, `SCHEMA_VERSION = 1`). One changed file invalidates the entire index and re-parses everything. No per-file reuse, no per-file invalidation, no rename/delete handling beyond full rebuild. | `IMPLEMENTED` (whole-index only) |
| G12 | **References panel / dependency graph / call hierarchy / impact view in UI** | No such surfaces. `ResourceGraph.tsx` is a runtime-memory chart, not a dependency graph. The `intel` surface renders `RepoIntelPanel` (ranked retrieval) only.                                                                                                         | `NOT_IMPLEMENTED`                |
| G13 | **Agent tools for navigation**                                               | None. Executable catalogue has exactly 12 tools: `fs.*`, `shell.*`, `npm.run`, `cargo.test`, `git.*`, `search.*` (pinned by `metrics_match_the_governance_record`).                                                                                                    | `NOT_IMPLEMENTED`                |
| G14 | **AI integration citing graph evidence**                                     | `ContextBuilder` feeds ranked _files_ to the agent. No symbol/reference/impact evidence, and no deterministic-vs-interpretation separation.                                                                                                                            | `NOT_IMPLEMENTED`                |

### 1.6 What already exists and is reusable

| Asset                                                                                                                    | Reusable for                                                                   |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
| `Provenance`, `LanguageSupport` enums (`types.rs`)                                                                       | provenance on graph edges (§8), deterministic-vs-heuristic labelling (§6)      |
| `scanner.rs` + per-file content hash                                                                                     | file inventory, change detection (§13)                                         |
| `persisted.rs` fingerprint + atomic write-then-rename cache pattern                                                      | persistent index pattern (§13) — extend to per-file granularity                |
| `manifest.rs` package/crate discovery + internal dep resolution                                                          | package/crate dependency graph (§7, §8)                                        |
| `DependencyGraph` package-level bidirectional + transitive BFS                                                           | `DIRECT_DEPENDENT`/`TRANSITIVE_DEPENDENT` (§9)                                 |
| `real_tools::dispatch` permission gate + `ToolEvidence` + `JsonlEvidenceSink`                                            | read-only agent tools with evidence (§12)                                      |
| `tool_catalogue` `EXECUTABLE_SPECS` / `mcp.tools.yaml` invariant / metrics pinned by test                                | registering the 8 nav tools honestly (§12)                                     |
| `intelligence::api::repo_intel_payload` (one function, three consumers)                                                  | the pattern for nav payloads: CLI `serve-intel`, Tauri command, direct callers |
| Four-zone shell (`ActivityRail`, `SurfaceHost`, `AgentDock`, `BottomPanel`), themes, `Panel`/`StatusBadge` design system | §10 UI without replacing the approved shell                                    |
| `ExplorerTree.onOpenFile` → `App.tsx:openFile(path)`                                                                     | click-a-result-to-navigate (§5)                                                |

### 1.7 Tool-executor architecture (unchanged, must be followed)

```
request
  └─▶ real_tools::dispatch(tool_id, params, context, runtime)      [the ONLY path]
        ├─ 1. get_real_tool(tool_id)   → None ⇒ Deny "has no executor" (no execution)
        ├─ 2. PermissionGate           → Deny ⇒ no execution, evidence still recorded
        └─ 3. executor                 → ToolResult + ToolEvidence
```

Invariants a new tool must satisfy:

1. an executor in `get_real_tool`;
2. a spec in `EXECUTABLE_SPECS` (the catalogue `debug_assert`s the correspondence);
3. an entry under `tools:` in `mcp.tools.yaml` (a committed test asserts every listed id has an
   executor);
4. metrics pinned by `metrics_match_the_governance_record` and recorded in
   `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md` §11.5 — **updated in the same change**.

### 1.8 Baseline test battery (current numbers, this session)

Run in this session on the primary working tree, _before_ implementation.

| Suite          | Command                                | Result                                          |
| -------------- | -------------------------------------- | ----------------------------------------------- |
| Frontend       | `pnpm --filter zylcode-desktop test`   | **67 passed / 0 failed**, 11 test files, exit 0 |
| Rust workspace | `cargo test --workspace --all-targets` | **584 passed / 0 failed / 1 ignored**, exit 0   |

Both rows were produced by running the suites in this session; no figure carried over from
an earlier checkpoint is used as evidence. The same two commands after implementation are
recorded in §14 so the delta is visible against this table rather than against memory.

> **Session note.** The first baseline attempt was blocked by a stale
> `zylcode.exe serve-intel --workspace C:\Projects\zylcode` process (PID 16556, started 11:19)
> holding a file lock on `target\debug\zylcode.exe`. The owner authorised terminating it. No git
> state was involved.

### 1.9 Parser feasibility (measured, not assumed)

Because the wave requires a real parser (§3), tree-sitter was **prototyped out-of-tree first**
before any repository change. A scratch crate outside the repository built and ran:

```
tree-sitter 0.25.10 + tree-sitter-rust 0.23.3
          + tree-sitter-typescript 0.23.2 + tree-sitter-javascript 0.23.1
$ cargo run --manifest-path <scratch>\Cargo.toml --quiet
ok rust grammar
```

Node kinds for Rust, TypeScript and JavaScript were then **dumped empirically from real parses**
(see §3.1) rather than asserted from memory.

---

## PART 2 — ARCHITECTURE (as implemented)

Everything in Part 2 describes the working tree as it stands at the end of this wave, and
every claim is attached to the evidence that supports it. §15 is the consolidated state
table. Nothing is marked `IMPLEMENTED` because an interface exists, and nothing is marked
`RUNTIME_VERIFIED` because a unit test passed.

---

### 2.0 Change inventory

**New crate — `crates/zylcode-nav`** (untracked; a leaf crate, so `zylcode-core` and
`zylcode-mcp` can both depend on it without either depending on the other).

| File                  | Lines | Role                                                                                      |
| --------------------- | ----: | ----------------------------------------------------------------------------------------- |
| `src/lib.rs`          |    47 | public surface + the crate's evidence policy                                              |
| `src/scan.rs`         |   185 | repository walk, content hashing, language selection                                      |
| `src/parse.rs`        |  1629 | tree-sitter → per-file facts (symbols, imports, exports, call sites, occurrences, scopes) |
| `src/resolve.rs`      |   739 | module placement, layout discovery, import bindings, globs                                |
| `src/model.rs`        |   813 | symbols, evidence labels, impact/graph query types                                        |
| `src/index.rs`        |   873 | `RepoIndex::build`, warm path, schema-versioned persistence, non-directory guard          |
| `src/queries.rs`      |  2085 | definitions, references, callers/callees, dependency/impact/graph queries                 |
| `src/cache.rs`        |   190 | one process-wide index slot (`query_repository`)                                          |
| `src/payload.rs`      |   478 | single `dispatch(&RepoIndex, &str, Value) -> Result<Value, String>` + goal citations      |
| `tests/navigation.rs` |  1168 | 23 end-to-end tests over deterministic fixture repositories                               |

**Modified — tracked:** workspace `Cargo.toml` (`zylcode-nav` member + 4 tree-sitter
workspace deps); `crates/zylcode-core` (`Cargo.toml`, `intelligence/mod.rs`,
`intelligence/nav_api.rs` **new**, `agent_protocol.rs`, `context_builder.rs`, `agent.rs`);
`crates/zylcode-mcp` (`Cargo.toml`, `lib.rs`, `real_tools.rs`, `registry.rs`,
`tool_catalogue.rs`, `nav_tools.rs` **new**, `tests/nav_tools.rs` **new`);
`crates/zylcode-cli/src/main.rs`; `apps/zylcode-desktop/src-tauri/src/main.rs`;
`mcp.tools.yaml`; `MCP_TOOLS_YAML_DISPOSITION.md`; `TOOL_CATALOGUE_TRUTH_TABLE.md`;
frontend `App.tsx`, `SurfaceHost.tsx`+ test,`ActivityRail.tsx`, `ContextSidebar.tsx`.

**Modified — generated, not authored:** the three files under
`apps/zylcode-desktop/src-tauri/gen/schemas/` (`acl-manifests`, `desktop-schema`,
`windows-schema`) were rewritten by `tauri-build` when the `src-tauri` workspace member
compiled. Each was checked by deep-key-sorted canonical JSON comparison of `HEAD` against
the working tree; all three returned **`KEY-ORDER-ONLY (content identical)`**. They are
recorded rather than reverted, because reverting generated output only produces the same
diff at the next build.

**New — frontend:** `lib/navIntel.ts` (+ `navIntel.test.ts`), `components/RepoNavPanel.tsx`
(+ `RepoNavPanel.test.tsx`), `components/shell/ActivityRail.test.tsx`.

**Deliberately untouched:** the five owner-preserved untracked paths (`.git-msg.txt`,
`COMPLETE_AUDIT_REPORT.html`, `crates/zylcode-core/src/project/filesystem.rs.backup`,
`docs/governance/RECONCILIATION_FORENSIC_REPORT.md`, `tasks/`) were read for reference
only. One consequence is recorded in §12.6.

---

### 2.1 Deterministic first — the evidence chain

Every answer travels one path, and a hop can only _lose_ confidence, never create it:

```
source bytes
  → scan    repository walk + content hash + language      (read-only)
  → parse   tree-sitter; failure ⇒ facts = none            (file counted unsupported)
  → index   facts + module placement + bindings + globs
  → query   attribution, always with a `via` explanation
  → payload one dispatch function; errors are errors       (never an empty success)
  → surface HTTP / Tauri IPC / MCP / agent prompt / React
```

Enforced in code rather than by convention:

1. **A parse failure yields no facts, not guesses.** Unreadable and unsupported files are
   counted (`files_unsupported`) and skipped; they never contribute a name-only hit.
2. **Attribution is a strict ordered fall-through** (§4.2) whose only name-only step is
   labelled `HEURISTIC` and carries the reason it degraded.
3. **A query that cannot run is an error, not an empty result.** `NavError::Unavailable`
   (no index) and `NavError::Rejected` (bad or ambiguous request) are distinct states, and
   the HTTP route renders them with HTTP 200 **only** because the body carries
   `{"error": …, "engine": "zylcode-nav"}` — a client must not be able to read "the index is
   down" as "this symbol is not referenced anywhere" (§15.3, probe 2).
4. **An unresolved selector returns `unresolved_subject` / `unresolved`, never a lookalike.**
   `an_unknown_selector_is_unresolved_never_fabricated` and
   `a_symbol_with_no_references_reports_none_rather_than_guessing` pin this.
5. **LLM output is never a source of symbol, reference, call or dependency evidence.** The
   only thing a model is given is retrieved graph citations, plus an instruction to label
   everything not covered by them as interpretation (§11).

---

### 2.2 Parser and language coverage

`tree-sitter 0.25.10` with `tree-sitter-rust 0.23.3`, `tree-sitter-typescript 0.23.2` and
`tree-sitter-javascript 0.23.1`, chosen only after an out-of-tree feasibility probe (§1.9).
Rust and TypeScript/JavaScript are the first two languages the work order requires; both
ship full facts. Node kinds were dumped from real parses rather than assumed.

Per-file facts persisted and reused: `content_hash`, `language`, `module`, `crate_key`,
symbols, imports, exports, call sites, occurrences and scopes.

**Module placement** (`resolve::place`) is decided from the _complete_ file set — a crate
root is a crate root because `src/lib.rs` exists, not because of walk order. That matters
for the warm path (§13.4): placement is recomputed every build and compared against the
cached facts, so a manifest appearing or disappearing invalidates the affected files instead
of silently reusing the old module path.

---

### 2.3 What the index refuses to do

- It never indexes a path that is not a directory. `RepoIndex::build` begins with
  `anyhow::ensure!(root.is_dir(), …)` (§13.5) — an empty index answers every query with
  "nothing found", which is a fabricated negative.
- It never writes to source files. The only write is the warm-start cache (§13.3).
- It never reports a relationship it cannot point at. Every impact item and every reference
  carries `via` — the edge or rule that produced it — and an empty `via` fails a test.

---

### 3. Deterministic first (continued)

The work order lists "deterministic first" as §3. In this implementation it is one
constraint expressed three ways: §2.1 (the evidence chain and the five rules it enforces),
§2.2 (the parser and what a parse failure yields) and §2.3 (what the index refuses to do).
The deterministic _contract_ those rules protect — what a symbol is and how a name becomes
one — is §4. No separate §3 artifact exists, and none is being claimed.

---

### 4. Symbol identity model

#### 4.1 Identity

A symbol is identified by the tuple:

| Field                    | Notes                                                                                                                                                                        |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| workspace root           | one index per repository root (§13.1)                                                                                                                                        |
| `file`                   | repository-relative path — the disambiguator of record                                                                                                                       |
| `language`               | rust / typescript / javascript                                                                                                                                               |
| `qualified_name`         | e.g. `crate::intelligence::api::repo_intel_payload`                                                                                                                          |
| `kind`                   | `module`, `struct`, `enum`, `trait`, `impl`, `function`, `method`, `constant`, `type_alias`, `macro`, `class`, `interface`, `type`, `component`, `variable`, plus `other(…)` |
| `range` (+ `name_range`) | byte/line range of the declaration and of its name                                                                                                                           |
| container / `module`     | enclosing module path                                                                                                                                                        |
| `crate_key`              | owning crate for Rust, `None` elsewhere                                                                                                                                      |
| `signature`              | parameter/return text where the grammar exposes it                                                                                                                           |

**Duplicates across files stay distinct** because `file` is part of identity and every
lookup is scoped: `duplicate_names_do_not_bleed_across_modules` asserts that two symbols
with the same name in different modules resolve independently, and
`call_hierarchy_is_labelled_and_never_text_search` asserts that a _different_ `helper` in
`other.rs` is not returned as a caller of `crate::util::helper` — the exact line a text
search would have produced.

#### 4.2 Selectors and attribution order

`SymbolSelector::{Qualified, At{file,line}, Name{name,file?}}`. An ambiguous selector is
**rejected**, not resolved by luck: `an_ambiguous_selector_refuses_to_search`.

Once an occurrence is located, attribution walks a fixed order and stops at the first step
that answers. Each step returns an `Attribution` with `Evidence` and a human-readable `via`:

1. the occurrence _is_ a declaration → `DETERMINISTIC · declaration`
2. it sits in an `use`/`import` statement → the statement says where the name came from →
   `DETERMINISTIC · import`
3. it has a module path prefix → resolved through the module table →
   `DETERMINISTIC`, or `did not resolve to a symbol` when the path is well-formed but empty
4. it has a member receiver → **`HEURISTIC`** — _"member access `recv.name` — name
   correspondence only, no receiver type is available to the parser"_
5. an import binding exists in this file → symbol, glob target, external package, or an
   honest `import unresolved: …`
6. the name is shadowed by an inner declaration → `DETERMINISTIC` but **no symbol**, with
   the reason `shadowed by an inner declaration of x` (a deliberate refusal: the outer
   symbol is not what was referenced)
7. the module scope chain of the reference → `DETERMINISTIC · in scope at the reference site`
   7b. glob/namespace imports, checked **after** local scope because a local declaration always
   wins over a glob
8. nothing in scope → **`HEURISTIC`** name-only correspondence, _or_
   `no symbol named x exists` when no such symbol exists at all

Steps 4 and 8 are the only paths to `HEURISTIC`, and both state why in the note. There is no
step that silently degrades to text search.

#### 4.3 Scope discipline (module depth floor)

`scope_candidate` refuses to reach _upward_. A bare name may only resolve to an item in an
enclosing module — `crate::util::helper` is not in scope inside `crate::api`, and a
root-level symbol is not visible from a sub-module without an explicit path (Rust 2018) or
an import (ESM). The floor is
`module_depth = len(chain_of(module)); min_depth = module_depth.max(scope_chain.len() - 1)`
— one level of slack for the trailing function/impl block, and no more. Without this floor
the scope chain would happily attach `helper` in `api.rs` to `util::helper` purely because
the names match.

---

### 5. References and navigation

`find_references` returns, in one payload:

- the **definition** — qualified name, kind, file, line, range, signature;
- **every location** — file, range, and the _reference kind_, one of
  `definition` / `import` / `export` / `call` / `type` / `read` / `write`;
- per-location **`evidence`** (`DETERMINISTIC`/`HEURISTIC`) and **`via`** (the attribution
  step that produced it);
- `deterministic_count` / `heuristic_count`, so a panel can never present a mixed set as if
  it were uniform;
- `unresolved` when the selector did not resolve — an empty list with a reason, never a
  fabricated list.

Required cases, each with a test in `crates/zylcode-nav/tests/navigation.rs`:

| Requirement                                                    | Test                                                            |
| -------------------------------------------------------------- | --------------------------------------------------------------- |
| same-file references                                           | `same_file_references_are_found`                                |
| cross-file references, deterministic and complete              | `cross_file_references_are_deterministic_and_complete`          |
| duplicate names do not bleed                                   | `duplicate_names_do_not_bleed_across_modules`                   |
| renamed import followed to the original symbol                 | `a_renamed_import_is_followed_to_the_original_symbol`           |
| no references ⇒ none reported, not guessed                     | `a_symbol_with_no_references_reports_none_rather_than_guessing` |
| unresolved selector ⇒ unresolved, never fabricated             | `an_unknown_selector_is_unresolved_never_fabricated`            |
| definition resolvable three ways (name / qualified / location) | `definitions_resolve_by_name_by_qualified_name_and_by_location` |
| ambiguity is refused                                           | `an_ambiguous_selector_refuses_to_search`                       |

**Navigation** means the result is clickable: `RepoNavPanel` passes
`onNavigateToFile(file, line)` through `SurfaceHost` into `App.navigateToFile`, which opens
the file, activates the explorer and switches to the code surface (§10.3).

**Runtime evidence** (§15.3): `find_references` for `repo_intel_payload` returned
`deterministic = 7`, `heuristic = 0`, definition at
`crates/zylcode-core/src/intelligence/api.rs:29`, with references at
`src-tauri/main.rs:217`, `cli/main.rs:524` and `api.rs:147` — each carrying
`evidence: DETERMINISTIC` and a `via` string.

---

### 6. Call hierarchy (callers / callees)

Every relation is one of exactly three labels, and the label is data, not prose:

- **`DETERMINISTIC`** — resolved through declaration, import, module path, binding or scope.
- **`HEURISTIC`** — the only deterministic-looking cases that cannot be proved from a parse:
  a **member call** (`recv.name`) has no receiver type in the syntax tree, so it is reported
  as a name correspondence, and a bare name with no binding or scope hit (§4.2 steps 4 and 8).
- **`UNSUPPORTED`** — the call site exists but there is no identifier node to attribute. The
  fixture case is TypeScript's `super()`: reporting nothing would hide a real call, and
  inventing a callee out of the call's _text_ is exactly what the label forbids. Pinned by
  `a_callee_with_no_identifier_to_attribute_is_labelled_unsupported`.

`call_hierarchy_is_labelled_and_never_text_search` is the guard rail for the work order's
"never call text search semantic callers" rule. It asserts, on one fixture:

- `find_callers(crate::util::helper)` → exactly 1 relation, `DETERMINISTIC`, in
  `crate::entry`, with a non-empty `note`, and **not** `other.rs` — because `other.rs` calls
  a _different_ `helper`, and "text search would have added it";
- `find_callees(crate::entry)` → resolves to `crate::util::helper`;
- `find_callees(add in ledger.rs)` → `deterministic_count == 0`,
  `heuristic_count == 1`, `evidence == HEURISTIC`, note contains _"name correspondence"_;
- an unknown target → no relations and `unresolved.is_some()`.

---

### 7. Import / export graph

#### 7.1 Languages and edge content

Rust and TypeScript/JavaScript first. Every import edge carries: the importing file, the
importing statement's range, the specifier text as written, the resolved target (file or
symbol), and the attribution rule that resolved it — i.e. the edge is **traceable back to a
line of source**, which is what "provenance" means here.

#### 7.2 Cases handled explicitly

- **Wildcards (`use_wildcard`).** `use crate::util::*;` nests the path _inside_ the wildcard
  node (`use_wildcard(scoped_identifier)`), so the path has to be lifted out of the node
  before resolution; doing so is what lets `use super::*;` resolve at all. The same handling
  covers `export * from …` (`export_wildcard`) on the TypeScript side.
- **Aliases.** `use x as y` / `import { a as b }` are modelled as `UseTree::As` with an alias
  stack, and each entry records `local` (the name usable _inside_ the importing file) versus
  `imported` (the name in the target), with `alias: local != imported`. A renamed import is
  followed back to the original symbol — `a_renamed_import_is_followed_to_the_original_symbol`.
- **External packages.** An import that resolves to a package outside the workspace binds to
  `BindingTarget::External(package)` and reports _"`x` is provided by external package `p`"_
  rather than pretending the symbol lives in this repository.
- **Broken imports.** `import { missing } from "./does-not-exist"` resolves to
  `BindingTarget::Unresolved(reason)` and stays visible as a broken edge —
  `typescript_graph_captures_imports_cycles_and_broken_imports`.
- **Cycles.** Detected and returned as such; the graph query can include them
  (`include_cycles`), and the panel shows a cycles disclosure (§10.2).

#### 7.3 Placement in the workspace

`file_dependencies` / `file_dependents` are the two MCP- and HTTP-facing queries over this
graph, each returning edges with `via` and `evidence`.

---

### 8. Dependency graph with provenance

`repository_graph_query(GraphQuery)` returns `nodes`, `edges`, `total`, and `truncated`:

| Control                     | Behaviour                                            |
| --------------------------- | ---------------------------------------------------- |
| `edge_kinds` / `node_kinds` | filter by kind                                       |
| `focus`                     | only edges touching this node — scoped exploration   |
| `depth`                     | expansion from `focus`; default 1, **clamped 1..=6** |
| `limit`                     | default 400, **clamped 1..=4000**                    |
| `include_cycles`            | surface detected import cycles                       |

`GraphQuery::normalised()` applies the clamps, so an absurd `depth` cannot be turned into a
hang and `limit: 0` means "default", not "nothing".

Every edge carries `evidence` and a `provenance` from
`{observed, parsed, derived, inferred}`: `observed` for something read off the filesystem,
`parsed` for something extracted from an AST or manifest, `derived` for something computed
from other facts (transitive closure, cycle detection), and `inferred` for a heuristic —
which, per the enum's own contract, is _never_ presented as deterministic evidence. `graph_scoping_trims_to_the_requested_neighbourhood`
asserts that a focused query returns exactly the neighbourhood asked for — the "large graphs
need filtering/collapsing/scoping" requirement is enforced server-side, not only in the UI.

**Runtime evidence** (§15.3): `repository_graph_query {focus: …, depth: 1, limit: 10}` →
`nodes = 10`, `edges = 10`, `total = 44`, `truncated = true`; every returned edge touches
the focus node; every edge `DETERMINISTIC` / `parsed`.

---

### 9. Impact analysis

#### 9.1 Classes

`ImpactClass::{DirectDependent, TransitiveDependent, PossibleTextualReference, Unresolved}`
— exactly the four the work order names.

- **DIRECT** = a file that declares or references the subject (symbol case) or imports it
  (file case).
- **TRANSITIVE** = reverse import closure up to `depth`, each hop labelled
  `imports <via> (hop n)`.
- **POSSIBLE_TEXTUAL_REFERENCE** = a mention with no binding. By construction
  `evidence == HEURISTIC` and `provenance == Inferred` — asserted, not assumed.
- **UNRESOLVED** = the subject could not be resolved; `items` is empty and the summary says
  _"no relationships are claimed"_.

#### 9.2 Relationships, not predictions

`ImpactReport.summary` contains _"not predictions"_ and the tests assert the summary never
contains `will break` (case-insensitive). `impact_reports_relationships_and_never_predictions`
also asserts that **every** item carries a non-empty `label`, a non-empty `via`, an allowed
evidence value, and a non-empty excerpt whenever an excerpt is present.

#### 9.3 A defect found by running the engine, and fixed

Probing `file_impact` on `crates/zylcode-nav/src/cache.rs` reported `cache.rs` as its own
**DIRECT_DEPENDENT**. The cause is correct behaviour in the parser: the
`use super::*;` inside `cache.rs`'s own `#[cfg(test)] mod tests` resolves back to `cache.rs`
— a real edge, and a useless claim (a file is never a dependent of itself).

Two changes, both narrow:

- `importers_of` now excludes `file` itself, with the reason recorded in the doc comment;
- `referencing_files` now actually excludes `target.file`, which its doc comment had claimed
  but its body did not do (the caller filtered instead).

`file_dependencies` still reports the self-import, because it _is_ an import statement in
that file — the fact stays visible where facts belong and is dropped only where it would be
misread as an impact relationship. Regression test:
`a_file_is_never_reported_as_its_own_direct_dependent`.

---

### 10. UI

#### 10.1 What was built

| File                               | Role                                                                                                                                                                                                                                                                                |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lib/navIntel.ts`                  | two-transport data path mirroring `repoIntel.ts`: Tauri `nav_query` on desktop, `POST /api/nav` in browser preview; `NavState` distinguishes `rejected` (engine refused — caller-fixable) from `unavailable` (no engine); `classifyNavFailure` keys off the engine's own vocabulary |
| `components/RepoNavPanel.tsx`      | the four views                                                                                                                                                                                                                                                                      |
| `components/shell/SurfaceHost.tsx` | hosts `RepoNavPanel` on the `intel` surface, above the existing repository-intelligence grid; new `onNavigateToFile` prop                                                                                                                                                           |
| `App.tsx`                          | `navigateToFile` = open file + activate explorer + switch to code surface                                                                                                                                                                                                           |

`NavState` never converts a failure into an empty result — an engine refusal is rendered as
a refusal, which is the UI half of §2.1 rule 3.

#### 10.2 The four views and their controls

1. **References** — definition header, then every location with evidence badge and `via`.
2. **Call hierarchy** — callers/callees with the three-way label and its hint text.
3. **Dependency graph** — nodes/edges with `focus` + `depth` re-scoping, edge-kind filter,
   and a cycles disclosure.
4. **Impact** — the four classes as sections, with the non-prediction summary shown as text.

Shared: a selector form (symbol / file / line + **Analyze**), an evidence filter, `details`-
based collapsing for long sections, `PAGE = 60` with **Show more** (so a 400-edge graph does
not render 400 rows at once), and `QueryStatus` rendering `idle` / `loading` / `rejected` /
`unavailable` as four distinct states.

Large results are also scoped server-side (§8), so the panel never receives the whole
repository graph by default.

#### 10.3 Shell integration and a gap that had to be closed

`intel` was already declared as a surface and already rendered `RepoIntelPanel` — but
**nothing in the application could ever activate it.** `ACTIVITY_TO_SURFACE` mapped nine
activities and none of them produced `"intel"`, so the surface (and anything on it) was
dead UI. Closed this wave:

- new **Code Navigation** entry in the activity rail (between Source Control and Missions),
  with a graph icon and an explicit `intel → intel` mapping in `ACTIVITY_TO_SURFACE`;
- **View → Code Navigation** menu entry;
- a `ContextSidebar` panel explaining the four views and where to run them.

Because `ACTIVITY_TO_SURFACE` is `Record<ActivityId, SurfaceType>`, adding the rail entry
forced the mapping at compile time — the type is the guard; the new
`ActivityRail.test.tsx` and the added `SurfaceHost` test are the runtime ones.

Theme, four-zone shell, Agent Dock and bottom panel are untouched; `RepoNavPanel` uses the
existing design tokens and panel conventions.

---

### 11. AI integration — citations, not vibes

#### 11.1 Retrieval vs citation are separate steps

- `payload::goal_tokens(goal)` — **retrieval**. Splits the goal into candidate symbol names.
  This is a _heuristic_ and is documented as one; it is never shown as an answer.
- `payload::goal_citations(index, goal, limit)` — **evidence**. Resolves the tokens through
  the normal definition/reference machinery (which drops ambiguity and drops heuristic
  references), returning only citations whose own `evidence` is `DETERMINISTIC`.
- `goal_citation_lines(…)` renders them one per line for a prompt. When nothing resolves it
  returns **exactly one** line saying so, so the prompt never contains a bare empty list.

#### 11.2 The agent prompt

`nav_api::agent_citation_block(root, goal, limit)` returns `Err` when the index cannot be
built, rather than an empty list. `ContextBuilder::build` turns that into an explicit line:

> `(repository graph unavailable: … — treat every structural claim as interpretation)`

`AgentContext.navigation_citations` is `#[serde(default)]` so an older context serialises
safely, and `agent::gather_context` renders the block under the heading
**"Repository graph evidence (deterministic, resolved from the parsed repository index)"**
with the instruction that anything not covered by it must be labelled interpretation.

The design rule: _"no citations"_ and _"we cannot read this repository"_ are different facts,
and a prompt that renders both as silence teaches the model that silence means **no
relationships exist**.

#### 11.3 Tests

| Test                                                     | Asserts                                                                                                                                                    |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `goal_citation_lines` ×3 (`payload.rs`)                  | retrieval→citation separation; ambiguity dropped; heuristic refs excluded; empty ⇒ exactly one explanatory line                                            |
| `nav_api` ×8                                             | bare and `nav.`-prefixed ids; `agent_citation_block` success and refusal                                                                                   |
| `agent_context_carries_deterministic_graph_citations`    | a goal naming a real symbol produces a non-empty citation block, **every** line starts with `DETERMINISTIC`, and each line contains a `file:line` location |
| `an_unindexable_workspace_says_the_graph_is_unavailable` | unindexable root ⇒ exactly one line, containing both _"repository graph unavailable"_ and _"interpretation"_                                               |

The second of those needed a root that is a **file**, not a missing path: the Repository
Intelligence warm-start writes `<root>/.zylcode/…`, and `create_dir_all` on that path
silently brings a non-existent workspace root into existence _before_ the navigation index
ever sees it. That side effect pre-dates this wave.

**G6 decided this session — (B) defect fixed now, fail-closed.** `persisted::ensure_root_exists()`
runs at the top of `PersistedIndex::build` and `rebuild` and returns
`Err("workspace root '…' does not exist (refusing to index a missing directory)")` _before_
anything is written, so a bad root can no longer be laundered into "indexed, empty, no error".
(A) was rejected because documenting a fail-open mutation as acceptable leaves the input error
hidden for every future caller; (C) was rejected because the change is two `ensure!` calls in a
module this wave already owns, guarded by tests that run in this wave's own battery:

| Test                                                                             | Asserts                                                                                               |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `build_on_a_missing_root_fails_and_creates_nothing` (persisted.rs)               | `build` on a non-existent root is `Err`, and neither `<root>` nor `<root>/.zylcode` exists afterwards |
| `rebuild_on_a_missing_root_fails_and_creates_nothing` (persisted.rs)             | the same for `rebuild`                                                                                |
| `a_missing_workspace_root_is_never_created_by_a_warm_start` (context_builder.rs) | the citation path cannot create the workspace it was asked about                                      |

---

### 12. Agent tools

#### 12.1 The eight tools

`nav.find_definition`, `nav.find_references`, `nav.find_callers`, `nav.find_callees`,
`nav.file_dependencies`, `nav.file_dependents`, `nav.symbol_impact`,
`nav.repository_graph_query` — read-only, no side effects beyond the warm-start cache.

One entry point serves all of them:
`zylcode_nav::payload::dispatch(&RepoIndex, &str, Value) -> Result<Value, String>`, which
injects `"engine": "zylcode-nav"` into every response and returns an error body (not an
empty success) for an unknown tool. `nav_api::nav_payload` accepts bare and `nav.`-prefixed
ids, and `payload_id()` strips the prefix.

#### 12.2 Wiring

- **MCP**: `nav_tools.rs` + `tests/nav_tools.rs` (all 8 executed end-to-end), registered in
  `lib.rs` / `registry.rs` / `real_tools.rs`, catalogued in `tool_catalogue.rs`, declared in
  `mcp.tools.yaml` (+8).
- **HTTP**: `POST /api/nav` in `zylcode-cli` (§15.3).
- **Tauri**: `nav_query` command in `src-tauri/src/main.rs`, registered in `invoke_handler`.

#### 12.3 Catalogue metrics (pinned)

| Metric                    | Value                                                                         |
| ------------------------- | ----------------------------------------------------------------------------- |
| `definition_count`        | **46** (20 executable + 27 bridge definitions − 1 overlap, `git.commit`)      |
| `executable_count`        | **20** (12 general-purpose + 8 `nav.*`)                                       |
| `tested_execution_count`  | **10** (`fs.read`, `shell.execute`, + the 8 `nav.*` via `tests/nav_tools.rs`) |
| `product_reachable_count` | **19** (`git.commit` is false)                                                |
| `r3_verified_count`       | **0**                                                                         |

Pinned by `tool_catalogue::tests::metrics_match_the_governance_record`, and mirrored in
`TOOL_CATALOGUE_TRUTH_TABLE.md` §11.5 and `MCP_TOOLS_YAML_DISPOSITION.md` (20 ids under
`tools:`, 21 retained under `proposed:`) — updated in the same change so the three cannot
drift apart without a test failure.

#### 12.4 What `r3_verified_count = 0` means here

`RUNTIME_VERIFIED` in §15 is a claim about _this wave's_ surfaces (HTTP + engine). The
catalogue's `r3` column is a different, stricter instrument — evidence captured through a
product surface for that specific tool — and it is honestly `0`. No nav tool is counted as
r3-verified by this wave's probes.

#### 12.5 Failure vocabulary

`NavError::Unavailable` displays _"repository index unavailable: …"_;
`NavError::Rejected` displays _"request rejected: …"_; the Tauri command wraps both with
_"repository navigation failed: …"_. `classifyNavFailure` in the frontend keys off exactly
that vocabulary (`"request rejected"` ⇒ `rejected`, otherwise `unavailable`), so the client
does not have to guess.

#### 12.6 An owner record this wave partly invalidated — recorded, not edited

`docs/governance/RECONCILIATION_FORENSIC_REPORT.md` is one of the five owner-preserved
untracked files and was **not** modified. Its §6 table (lines 144–150) now reads:

| Line | Says                                                       | Status now                                                                                                                                                                                           |
| ---- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 144  | _"Canonical? **YES** — 12 executable + 21 proposed"_       | **STALE.** The counts are `20` executable + `21` proposed; the wave added 8 `nav.*` entries.                                                                                                         |
| 146  | _"Stale? **NO** — matches `get_real_tool` exactly"_        | **STILL TRUE.** `real_tools.rs` gained the matching executors, and `shipped_config_has_no_definition_only_tools` still asserts `{configured} == {executable}` — it passes in the 166-test run above. |
| 148  | _"Tested? YES — … asserts `{configured} == {executable}`"_ | **STILL TRUE**, same test.                                                                                                                                                                           |
| 150  | _"**Disposition:** KEEP as-is. No action required."_       | **SUPERSEDED.** Correct for the file as it then stood; the file has since changed by +8 entries.                                                                                                     |

Only the line-144 counts are wrong; the rest of the record still holds, which is why it is
quoted rather than summarised as "stale". The document is a point-in-time forensic record and
this wave is what invalidated one row of it. It is flagged here for the owner to refresh or
retire deliberately — editing someone else's preserved artifact silently would be exactly the
kind of unstamped change this wave forbids.

**Classification decision (this session): `HISTORICAL_RECORD` — line 144 left byte-for-byte
unchanged.** The report is dated 2026-09-21 and pins SHAs `4cd63b8` and `865c142`, and it is
already read as history rather than as live truth (`ENGINEERING_TRUTH.md:70` lists it under
"Diverged Gate-0 history"). Rewriting line 144 in place would silently move a dated,
SHA-pinned measurement to fit a later change: the report would then contradict the SHAs it
names, and no reader could tell which numbers were observed when. `LIVE_GOVERNANCE` was
therefore rejected — the _live_ counts already live in `TOOL_CATALOGUE_TRUTH_TABLE.md`,
`MCP_TOOLS_YAML_DISPOSITION.md` and `mcp.tools.yaml`, all of which this wave updated. A fresh
snapshot would be a new dated report issued by the owner, not an edit to this one.

---

### 13. Performance and incremental indexing

#### 13.1 One index per process, not per query

`zylcode_nav::cache::query_repository(root, f)` holds one process-wide slot. `f` receives the
index with the slot's guard held, so concurrent callers cannot build twice. The error path
**never** caches — a transient failure is not stored as an answer.

#### 13.2 The staleness window is a stated trade-off

`REUSE_FOR = 1s`. Inside the window a query is answered from memory and may miss an edit
that landed a moment ago. That is deliberate: the alternative (stat every file on every
request) reintroduces O(files) work per query. The window is bounded at one second so the
exposure is short, and a rebuild is cheap because it re-reads bytes without re-parsing.

**Index-debounce trade-off:** no debounce was added on the UI side. Debouncing per panel
would bound _one_ consumer while MCP, HTTP, Tauri and the agent prompt each built their own
index. Bounding the _index_ instead bounds all of them with one clock, at the cost of the
1-second staleness above. Correctness first: the window is explicit in the module docs and
in the tests, not an emergent race.

#### 13.3 Full facts are persisted, not a summary

`<root>/.zylcode/nav-index.json` (gitignored via `.zylcode/`) stores
`Persisted { schema, files: BTreeMap<path, FileFacts> }` — the **complete** per-file facts,
so a warm start needs no parsing at all. `SCHEMA_VERSION = 1`; a version mismatch discards
the cache rather than mis-reading it. Writes are write-then-rename, so a crash mid-write
leaves the previous cache or none — never a half-written file that fails to parse forever.

**Write trade-off:** building the index writes this one file. It is a side effect of a
read-only query, which is why it is documented at the top of `cache.rs`, why it is confined
to `.zylcode/` (already gitignored), why source files are never touched, and why a failed
build is reported rather than cached. The alternative — no persistence — makes every process
start pay a full parse of the repository.

The cache is written only when something actually changed:
`!warm || files_parsed > 0 || files_removed > 0`.

#### 13.4 Warm path and reparse-on-rename

A cached file is reused **only** when all four match:

```
content_hash && language && module && crate_key
```

- **Rename → reparse.** The cache key is the repository-relative path, so a renamed file has
  no entry under its new name and is parsed afresh. Deliberate: the decision was _reparse, do
  not reuse-by-hash_, because a file's module path and crate membership are derived from its
  path — reusing facts by content alone would attach the old module identity to the new
  location. The same four-way check also invalidates facts when a manifest appears or
  disappears (`module` / `crate_key` change without any byte changing).
- **Edit → reparse of one file.** `editing_one_file_reparses_exactly_one_file` measures it;
  `an_unchanged_repository_rebuilds_from_the_cache` measures the zero-parse case.

#### 13.5 Non-directory root guard

`RepoIndex::build` starts with `anyhow::ensure!(root.is_dir(), …)`. Without it a typo'd or
deleted root would build an _empty_ index that answers every query with "nothing found" —
a fabricated negative, which is the single most dangerous failure mode for a tool whose
promise is "we tell you when something is not referenced". Refusing turns the failure into
`NavError::Unavailable`, which every surface renders as an explicit failure.

---

### 14. Tests — current numbers

All figures below were produced by running the commands in this session, after every change
in this wave. Baseline for comparison is §1.8 (584 Rust / 67 frontend).

> **Re-run 2026-09-29 (remediation pass — §16's G11, G3 and G2).** Every gate in this section was
> re-executed from scratch after those three fixes. The figures below are that re-run, not a
> carry-over.

#### 14.1 Rust

| Command                                                           | Result                                                   |
| ----------------------------------------------------------------- | -------------------------------------------------------- |
| `cargo test --workspace --all-targets --offline`                  | **671 passed / 0 failed / 1 ignored**, no errors, exit 0 |
| `cargo fmt --all --check`                                         | exit 0                                                   |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 0                                                   |

Per-crate, from the same run:

| Crate                                  | Result                                                                                                                                                                                                                            |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `zylcode-nav`                          | **51 passed / 0 failed** (28 unit + 23 integration)                                                                                                                                                                               |
| `zylcode-core` (lib)                   | **412 passed / 0 failed** — +8 over the previous session, the G3 citation-delivery tests (§15.6)                                                                                                                                  |
| `zylcode-core` (integration + benches) | **41 passed / 0 failed / 1 ignored** (agent_loop 4, commissioning 1, crash_recovery 3, decision_proptest 21, e2e_crash_recovery 2, first_mission_e2e 2, pipeline_proptest 6, repo_intelligence_benchmark 2, live_commissioning 0) |
| `zylcode-mcp`                          | **166 passed / 0 failed** (106 lib + 16 + 10 + 11 + 23)                                                                                                                                                                           |
| `zylcode-cli`                          | **1 passed / 0 failed**                                                                                                                                                                                                           |
| `zylcode-desktop` (bin)                | 0 unit tests (the desktop's tests are the frontend ones, below)                                                                                                                                                                   |
| **Total**                              | **671**                                                                                                                                                                                                                           |

The workspace run's single ignored test is the pre-existing live-provider commissioning test
(`crates/zylcode-core/tests/live_commissioning.rs`,
`#[ignore = "makes real provider calls; run explicitly with --ignored"]`) — ignored before
this wave and unchanged by it. Running `cargo test -p zylcode-mcp` _without_ `--all-targets`
additionally surfaces one ignored **doctest** of that crate; it is not part of the workspace
figure above.

Delta against §1.8: **+87 passing tests, 0 failures, 0 new ignores.** The +8 over the previous
session's 663 are the eight G3 tests added for `repository_context_block` /
`with_repository_context` / `provider_request` (§15.6); `fmt`, `clippy` and all frontend gates
were re-run after that change too.

#### 14.2 Frontend

| Command                               | Result                                          |
| ------------------------------------- | ----------------------------------------------- |
| `pnpm typecheck`                      | exit 0                                          |
| `pnpm --filter zylcode-desktop test`  | **98 passed / 0 failed**, 14 test files, exit 0 |
| `pnpm --filter zylcode-desktop build` | exit 0 (466 modules transformed)                |

Delta against §1.8: **+31 passing tests, 0 failures** (baseline 67 in 11 files → 98 in 14).
The +8 over the previous session's 90 are the five G11 race tests in
`RepoNavPanel.test.tsx` (§15.5b) and the three transport-selection tests in
`navIntel.test.ts` (§15.7).

#### 14.3 Fixture discipline

Every navigation test builds a deterministic fixture repository in a temp directory —
`rust_repo()` (a small Rust package with deliberate edge cases: duplicate names, a renamed
import, a member call, a file that mentions a symbol without binding it) and `ts_repo()`
(a TypeScript package with an import cycle and a broken import). No test depends on this
repository's own contents, so none of them can pass for the wrong reason.

---

### 15. Evidence states

#### 15.1 The states

`NOT_IMPLEMENTED` → `IMPLEMENTED` → `TESTED` → `UI_WIRED` → `RUNTIME_VERIFIED` →
`EVIDENCE_RECORDED` → (`ACCEPTED` owner-only) / `BLOCKED` / `UNVERIFIED`.

#### 15.2 State table

| Deliverable                                          | State                                            | Evidence                                                                                                                                                                                                                             |
| ---------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Deterministic evidence chain (§2.1)                  | **TESTED**                                       | attribution + refusal tests in `navigation.rs`                                                                                                                                                                                       |
| Parser: Rust / TS / JS (§2.2)                        | **TESTED**                                       | `rust_repo` + `ts_repo` fixtures, `cargo test -p zylcode-nav`                                                                                                                                                                        |
| Non-directory guard (§13.5)                          | **TESTED**                                       | `cache.rs` unit tests (bad root ⇒ `Unavailable`, not cached)                                                                                                                                                                         |
| Symbol identity model (§4)                           | **TESTED**                                       | `definitions_resolve…`, `duplicate_names…`, `an_ambiguous_selector…`                                                                                                                                                                 |
| References + navigation (§5)                         | **TESTED**                                       | 8 tests, table in §5                                                                                                                                                                                                                 |
| References over HTTP                                 | **RUNTIME_VERIFIED**                             | §15.3 probe 1                                                                                                                                                                                                                        |
| Call hierarchy (§6)                                  | **TESTED**                                       | `call_hierarchy_is_labelled_and_never_text_search`, `…_labelled_unsupported`                                                                                                                                                         |
| Import/export graph (§7)                             | **TESTED**                                       | `rust_module_and_import_edges_resolve`, `typescript_graph_captures_imports_cycles_and_broken_imports`                                                                                                                                |
| Dependency graph + provenance (§8)                   | **RUNTIME_VERIFIED**                             | §15.3 probe 3                                                                                                                                                                                                                        |
| Impact analysis (§9)                                 | **TESTED** + **RUNTIME_VERIFIED**                | `impact_reports_relationships_and_never_predictions`, `file_impact_reports_importers_without_predicting`, `a_file_is_never_reported_as_its_own_direct_dependent`, §15.3 probe 4                                                      |
| Index reuse / incremental reparse (§13.4)            | **TESTED**                                       | `an_unchanged_repository_rebuilds_from_the_cache`, `editing_one_file_reparses_exactly_one_file`                                                                                                                                      |
| Shared index slot (§13.1)                            | **TESTED**                                       | `cache.rs` concurrency/error-caching tests                                                                                                                                                                                           |
| MCP `nav.*` ×8                                       | **TESTED**                                       | `crates/zylcode-mcp/tests/nav_tools.rs` (all 8 executed)                                                                                                                                                                             |
| MCP catalogue metrics 46/20/10/19/0                  | **TESTED**                                       | `metrics_match_the_governance_record`                                                                                                                                                                                                |
| HTTP `POST /api/nav`                                 | **RUNTIME_VERIFIED**                             | §15.3                                                                                                                                                                                                                                |
| Tauri `nav_query` command                            | **RUNTIME_VERIFIED**                             | §15.5 — the running window's `queryNav` takes the `TAURI_DESKTOP` branch, so `invoke("nav_query")` really executed: `find_references` / `find_callers` / `symbol_impact` all returned real data in ~3 s                              |
| Frontend `lib/navIntel.ts`                           | **TESTED**                                       | 14 tests, incl. `queryNav (transport selection)` covering the desktop and browser branches (§15.7)                                                                                                                                   |
| `RepoNavPanel` four views                            | **TESTED**                                       | 14 tests — 9 base + 5 for overlapping queries (§15.5b)                                                                                                                                                                               |
| Shell wiring (rail, menu, sidebar, `SurfaceHost`)    | **UI_WIRED** + **TESTED** + **RUNTIME_VERIFIED** | `ActivityRail.test.tsx` (2), `SurfaceHost.test.tsx` (2), `pnpm typecheck`, §15.5 journeys A–D                                                                                                                                        |
| UI in a running desktop window (journeys A–J)        | **RUNTIME_VERIFIED**                             | §15.5 — real window, real repo, all ten journeys; re-run after remediation **16/16 PASS**, 0 exceptions                                                                                                                              |
| `RepoNavPanel` under overlapping queries             | **RUNTIME_VERIFIED**                             | §15.5b — G11 fixed (ownership token + per-tab typing + error boundary), 5 regression tests, 2 mutation proofs, **journey K 10/10 clean**                                                                                             |
| Browser-preview `queryNav` → `fetch("/api/nav")`     | **RUNTIME_VERIFIED**                             | §15.7 — journey M **6/6 PASS** on a real browser against the Vite proxy → `serve-intel`, HTTP 200, same 9-row answer the desktop gives                                                                                               |
| AI citations in agent context (§11)                  | **TESTED**                                       | `goal_citation_lines` ×3, `nav_api` ×8, `context_builder` ×2                                                                                                                                                                         |
| Citations computed inside a live agent turn          | **RUNTIME_VERIFIED**                             | §15.6 — `ContextBuilder::build` really ran: `.zylcode/intelligence-index.json` was rewritten 21 s into the turn                                                                                                                      |
| Citations delivered into a live provider request     | **RUNTIME_VERIFIED**                             | §15.6 journey L — `prompt_hash` = `sha256(prompt_text)` for a 1778-char prompt that opens with `REPOSITORY INTELLIGENCE CONTEXT` and carries 4 `DETERMINISTIC file:line` rows; the model's plan then named two of those files        |
| AI citations delivered to a model (`AGENT_VERIFIED`) | **BLOCKED**                                      | §15.6 — delivery is now runtime-proven, but the response carried no `DETERMINISTIC` `file:line` of its own (§17.2's criterion) and every cloud provider is unusable here (OpenRouter/Anthropic `401`, DeepSeek `402`). Not promoted. |
| Governance doc §2–§15                                | **EVIDENCE_RECORDED**                            | this document                                                                                                                                                                                                                        |
| Owner acceptance                                     | **NOT_STARTED**                                  | awaiting review — see safety envelope, §1.1                                                                                                                                                                                          |

The two rows left **`UNVERIFIED`** on purpose last session have now been commissioned in the
direction their evidence actually supports: the desktop UI was driven through journeys A–J and
re-driven after the fixes (**16/16**, §15.5), and G11's crash row moved from **FAILS** to
**`RUNTIME_VERIFIED`** on ten clean races (§15.5b). The browser transport reached
**`RUNTIME_VERIFIED`** for the first time (§15.7). Citations are now proven to reach a real
provider request, which is why that row exists as its own line — but the `AGENT_VERIFIED` row
stays **`BLOCKED`**, and neither the delivery evidence nor the passing test suites were allowed
to promote it. §16 now shows **G2, G3 and G11 all closed**, with G5 the only gap still
awaiting a decision (G7–G10 are accepted trade-offs). Gap numbers here are §16's namespace;
§1.5's separate G1–G14 numbering is unchanged.

#### 15.3 Runtime evidence (captured this session)

Probes ran against `target\debug\zylcode.exe serve-intel --port <n>` with
`POST /api/nav`, bodies `{"tool": …, "args": …}`. Logs:
`%TEMP%\opencode\serve-intel-nav{,2,3,4}.log` (stderr files empty). **Every server was
stopped after its probe; no `zylcode` process remains.**

| #   | Request                                                                            | Observed                                                                                                                                                                                                                                                                                    |
| --- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `find_references {name: "repo_intel_payload"}`                                     | `engine: zylcode-nav`; definition `crate::intelligence::api::repo_intel_payload @ crates/zylcode-core/src/intelligence/api.rs:29`; `deterministic 7`, `heuristic 0`; references at `src-tauri/main.rs:217`, `cli/main.rs:524`, `intelligence/api.rs:147`, each `DETERMINISTIC` with a `via` |
| 2   | `find_references {}` (no selector)                                                 | `{"error": "repository navigation failed: request rejected: a selector is required: `target`, `id`, `qualified_name`, `name`, or `file`+`line`", "engine": "zylcode-nav"}` — a refusal, not an empty answer                                                                                 |
| 3   | `repository_graph_query {focus, depth: 1, limit: 10}`                              | `nodes 10`, `edges 10`, `total 44`, `truncated true`; all edges touch the focus; all `DETERMINISTIC` / `parsed`                                                                                                                                                                             |
| 4   | `symbol_impact {subject: "file", file: crates/zylcode-nav/src/cache.rs, depth: 2}` | `DIRECT 2`, `TRANSITIVE 2`, `POSSIBLE 0`, `UNRESOLVED 0`; summary carries the "relationships, not predictions" caveat                                                                                                                                                                       |
| 5   | `file_dependencies` on `cache.rs`                                                  | surfaced `cache.rs --imports--> cache.rs` — the finding behind the fix in §9.3                                                                                                                                                                                                              |

Probe 5 is the reason §9.3 exists: the defect was not found by reading the code, it was found
by querying the engine and being surprised by the answer.

#### 15.4 What "not committed" means

Per §1.1 nothing was committed, pushed, rebased, stashed or reset this wave. The working tree
holds every change described above as uncommitted edits for owner review. HEAD is still
`406753f`; `main...origin/main` is unchanged; the stash is empty; `Cargo.lock` remains
gitignored (it gained 6 packages — `streaming-iterator`, `tree-sitter`,
`tree-sitter-language`, `tree-sitter-rust`, `tree-sitter-typescript`,
`tree-sitter-javascript`).

**Re-verified at the end of the remediation pass (2026-09-29):**

```
$ git rev-parse HEAD            406753f498b97137d515eaafd99e7b33164fdee8
$ git rev-parse origin/main     406753f498b97137d515eaafd99e7b33164fdee8
$ git rev-list --left-right --count HEAD...origin/main
0	0
$ git stash list
(empty)
$ git status --porcelain | wc   41   (26 modified + 15 untracked)
```

The count is **one larger than at the previous checkpoint (40 = 25 M + 15 U)**, and the single
delta is `apps/zylcode-desktop/vite.config.ts` — the G2 proxy entry described in §15.7. No
other file entered or left `git status`; no preserved path was touched
(`.git-msg.txt`, `COMPLETE_AUDIT_REPORT.html`,
`crates/zylcode-core/src/project/filesystem.rs.backup`,
`docs/governance/RECONCILIATION_FORENSIC_REPORT.md` — including line 144's
`HISTORICAL_RECORD`, never edited — and `tasks/` all still present).
`apps/zylcode-desktop/src-tauri/tauri.conf.json` still hashes to
`E0886F8897122B0F9AA364FB9E9E56C61D38E443866F52F3EC7A36295EC52C6E` and contains no
`additionalBrowserArgs` (§29).

#### 15.5 Desktop commissioning — journeys A–J on the real window

**Launch path (repository-supported, no mocks).** A debug build of `zylcode-desktop` loads
`build.devUrl`, so the app was started exactly as `tauri dev` would: `pnpm --filter
zylcode-desktop dev` (Vite, `port: 1420`, `strictPort`) plus
`target\debug\zylcode-desktop.exe` with the **process CWD set to `C:\Projects\zylcode`**
(the workspace root is the literal `"."`, so CWD _is_ the root). Engine log on start:
`initializing ZylCodeEngine workspace_root=.` and `loaded MCP tools from default location
tools=20`; stderr empty. Nothing was mocked, stubbed or served from a fixture.

**Driving the window.** `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` is a dead route: wry 0.55.1
unconditionally calls `options.set_additional_browser_arguments(...)` with its own defaults
(`wry-0.55.1\src\webview2\mod.rs:294–327`), which overrides the environment variable.
Tauri's per-window `additionalBrowserArgs` (`tauri-utils-2.9.3\config.rs:2083`, serde alias
`additional-browser-args`) _does_ reach `with_additional_browser_args`, so it was set
**temporarily** to

```
--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required --remote-debugging-port=9333
```

the desktop was rebuilt, the journeys were driven over CDP, and `tauri.conf.json` was then
restored **byte-for-byte** (SHA-256 `E0886F8897122B0F9AA364FB9E9E56C61D38E443866F52F3EC7A36295EC52C6E`
both before and after), so the file is clean in `git status`. This was a commissioning-only,
fully reverted change; the window's own browser process confirmed `--remote-debugging-port=9333`
on its command line, and the debug port binds `::1` only.

| #   | Journey                                                   | Result                 | Observed in the real window                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| --- | --------------------------------------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A   | activity-rail entry                                       | **PASS**               | exactly one `button[aria-label="Code Navigation"]`; click sets the active state and opens `main [role=tablist]` with References / Call hierarchy / Dependency graph / Impact, while the ContextSidebar switches to `CODE NAVIGATION`                                                                                                                                                                                                                                                                                                                                                                                                                  |
| B   | View → Code Navigation                                    | **PASS**               | control first: leaving the surface removes the tablist; `role=menu[aria-label="View"]` contains `Code Navigation`; clicking that item re-opens the surface with all four tabs                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| C   | surface opens                                             | **PASS**               | `main` headings `CODE NAVIGATION`, `REPOSITORY INTELLIGENCE`, `WHAT AM I LOOKING AT`, `CAPABILITIES`; Analyze present; badge `LIMITED` → `AVAILABLE` once a query lands                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| D   | ContextSidebar reachable                                  | **PASS**               | `aside`, 256 px wide, visible, title `CODE NAVIGATION`, copy naming `DETERMINISTIC / HEURISTIC / UNSUPPORTED`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| E   | loads against a real repo                                 | **PASS**               | workspace banner `C:\Projects\zylcode`; `find_references {name: query_repository}` settled in **3 s** with `8 deterministic`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| F   | real data in results                                      | **PASS**               | definition `crate::cache::query_repository · pub fn query_repository<T>(…)` at `crates/zylcode-nav/src/cache.rs:L93:C8`; groups `crates/zylcode-nav/src/cache.rs (8)` and `crates/zylcode-nav/src/lib.rs (1)`; 9 location buttons labelled `L93:C8`, `L142:C21`, `L144:C21`, `L149:C21`, `L156:C19`, `L170:C20`, `L172:C13`, `L183:C19`, `L44:C17`; `8 deterministic / 1 heuristic / 1 unattributed`; each row carries a `via` justification (`via in scope at the reference site`, `via declaration`)                                                                                                                                                |
| F′  | **every shown `file:line` independently checked on disk** | **PASS 9/9**           | each of the nine paths opened from the filesystem and each line re-read: `:93` `pub fn query_repository<T>(`; `:142/:144/:149/:156/:170/:172/:183` each contain a `query_repository` call; `lib.rs:44` `pub use cache::{query_repository, NavError, REUSE_FOR};` — no missing file, no line that failed to support the claim (`%TEMP%\opencode\cdp\file_line_verification.txt`)                                                                                                                                                                                                                                                                       |
| G   | result click navigates to the file                        | **PASS**               | clicking `Open crates/zylcode-nav/src/cache.rs` sets the Explorer rail active, opens editor tab `div[title="crates/zylcode-nav/src/cache.rs"]`, renders breadcrumbs `crates › zylcode-nav › src › cache.rs`, and removes the nav panel. (The first harness reported G false because it looked for editor tabs among `main button[title$=".rs"]`; editor tabs are `div[title=…]`. `probe3_g.mjs` re-ran the journey with the correct selector — PASS, and `railActive('Code Navigation')` is also `true` there only because the active-state class match is not exclusive; the decisive evidence is the tab + breadcrumbs + the nav panel being gone.) |
| H   | references / call hierarchy / impact displayed            | **PASS ×3**            | call hierarchy counts line read `7` deterministic relations with `DETERMINISTIC` / `HEURISTIC` / `UNSUPPORTED` labels on the rows; impact reached its `direct / transitive / possible / unresolved` counts (`0 direct dependents…` for this subject); references returned to `8 deterministic`                                                                                                                                                                                                                                                                                                                                                        |
| I   | switch away and back does not corrupt                     | **PASS**               | leaving removes the tablist; returning restores the four tabs, no error text, Analyze enabled, and a re-run produces references again                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| J   | no material console/runtime errors                        | **PASS (walkthrough)** | 4 events captured across the whole boot-and-walk: `[vite] connecting/connected` ×2 (debug), the React DevTools notice (info), one `sandbox` iframe warning on `?embed=1` (warning). **0 exceptions, 0 error-level entries**                                                                                                                                                                                                                                                                                                                                                                                                                           |

Independent cross-check of the same engine over the other transport: `serve-intel`
`POST /api/nav {"tool":"find_references","args":{"name":"query_repository","limit":500}}`
→ HTTP 200 in **3.07 s**, `engine=zylcode-nav`, `refs=8`, `possible=1`, `deterministic=8`,
`heuristic=1`, first row `crates/zylcode-nav/src/cache.rs:93 [DETERMINISTIC] pub fn query_repository<T>(`.

Evidence: `%TEMP%\opencode\cdp\journeys_evidence.json`, `probe_evidence.json`,
`file_line_verification.txt`, and the drivers `journeys.mjs`, `probe3_g.mjs`,
`probe4_race.mjs`, `probe5_postcrash.mjs` (re-run with `node journeys.mjs` while the window and
CDP are up). The 16 recorded checks are 15 `true` / 1 `false`, the single `false` being the
journey-G selector bug described above, which `probe3_g.mjs` re-verified as a pass.

**Re-run after the remediation pass (2026-09-29).** All ten journeys were re-driven against
the same real window by `journeys_aj_v2.mjs`, with the corrected journey-G selector folded
into the main harness: **16/16 PASS**, no `false`. Journey J on that pass recorded **0
uncaught exceptions and 0 error-level console entries** across the whole boot-and-walk (4
informational events: two `[vite]` connect lines and the React DevTools notice). Boot
assertion: `{href: "http://localhost:1420/", tauri: true, env: "TAURI_DESKTOP",
workspace: "C:\Projects\zylcode"}`.

The nine journey-F `file:line` claims were re-verified against disk on that pass too:
**9/9** — all eight `crates/zylcode-nav/src/cache.rs` lines (`:93`, `:142`, `:144`, `:149`,
`:156`, `:170`, `:172`, `:183`) contain `query_repository`, and
`crates/zylcode-nav/src/lib.rs:44` re-exports it.

Evidence: `journeys_aj_v2.mjs`, `journeys_aj_v2_evidence.json`, `file_line_verification_v2.txt`.

#### 15.5b G11 — the failure the journeys did not cover (recorded, then fixed)

Driving journeys A–J in order passes. Deliberately _not_ waiting for an in-flight query does
not:

- **Trigger:** click **Analyze**, then while the query is still running click the **Impact** tab
  (i.e. two `run()` calls in flight — a normal impatient-user action).
- **Error:** `TypeError: Cannot read properties of undefined (reading 'filter')` thrown three
  times from `ImpactView` (`RepoNavPanel.tsx:777`, `data.items.filter(...)`).
- **Aftermath:** `document.getElementById("root").childElementCount === 0`,
  `document.body.innerText.length === 0`, no rail, no sidebar — the whole React tree unmounts
  because the app has no error boundary. The window is blank until the app is restarted.
- **Root cause:** `run()` (`RepoNavPanel.tsx:171–212`) ends with
  `setState(await queryNav(tool, args))` and carries no request token, so whichever query
  resolves _last_ wins. The view is chosen by `tab` (`:395–430`) while the payload is cast
  unconditionally (`state.data as NavImpactResult`), so a `find_references` / `find_callers`
  payload rendered under `tab === "impact"` reaches `ImpactView`, whose only guard is
  `data.unresolved_subject` — absent on a references payload, so it passes straight through to
  `data.items`, which is also absent.
- **Reproducible:** attempt 1 of 4 in `probe4_race.mjs`, 3 exceptions; `probe5_postcrash.mjs`
  confirms the destroyed root.

> The line numbers in that diagnosis are as they stood before the fix; `RepoNavPanel.tsx` has
> changed since. The diagnosis itself is unchanged and is kept verbatim as the record of what
> was found.

Journey J is reported **PASS for the A–J walkthrough** and _not_ as a blanket "the surface
cannot crash" — that stronger claim was false at the time, and G11 is why.

##### The fix (remediation pass, 2026-09-29)

The defect was fixed at its cause, not papered over. Three things changed:

1. **Request ownership.** `run()` now stamps every query with a monotonic token held in a
   `useRef` counter. A response is applied only while its token is still current
   (`if (!isCurrent()) return;`); a stale response is discarded, never written into state.
   Whichever query resolves _last_ no longer wins — the query that was _asked for last_ wins.
2. **Per-tab result typing.** The untyped `state.data as NavImpactResult` casts are gone.
   `RepoNavState` is now a discriminated union — `AnswerlessNavState | ReadyNavState` — where
   `ReadyNavState` is keyed by tab, and `readyStateFor(tab, data, via)` applies a structural
   payload guard before anything is accepted. The compiler, not an `as` cast, is what blocks a
   mode-A response being consumed as a mode-B result. Rendering is additionally gated on
   `state.kind === "ready" && state.tab === <view> && tab === <view>`, with a defensive
   warning if that invariant is ever violated.
3. **Containment.** `NavErrorBoundary` wraps `case "intel"` in `SurfaceHost.tsx`, so even a
   render failure inside the surface cannot unmount the rest of ZylCode. This is the second
   line of defence; it is not the fix — (1) and (2) are.

`RepoNavPanel.test.tsx` gained a `RepoNavPanel — G11 overlapping queries` block with **5
tests** (file total 9 → 14): _discards an answer that resolves after the view has moved on_,
_keeps the newest request authoritative when an older one resolves later_, _survives
A → B → A switching, drawing only the answer for the view asked last_, _refuses an engine
payload that does not answer the requested mode_, _contains a rendering failure instead of
unmounting the rest of ZylCode_.

**Mutation proof 1 — break the ownership check.** Bypass `if (!isCurrent()) return;` so a stale
response is applied again: **3 tests failed** (the discard, the out-of-order, and the A→B→A
case). Restored.

**Mutation proof 2 — break the payload guard.** Make `isNavReferenceResult` `return true;` so
any payload is accepted as a references answer: **1 test failed** (the mode-mismatch refusal).
Restored.

Both mutations were reverted; `navIntel.ts` and `RepoNavPanel.tsx` were re-read afterwards and
are byte-identical to the fixed state.

##### Journey K — 10 races on the real desktop window

`journey_k.mjs` drove the real `zylcode-desktop` window (CDP on `::1:9333`) through ten
deliberately overlapping query sequences — `References→Impact`, `Impact→Call hierarchy`,
`References→Dependency graph→References→Dependency graph`, `Impact→References`, and further
shuffles — clicking the next tab the moment the previous query was still in flight.

| Measure                                                     | Result         |
| ----------------------------------------------------------- | -------------- |
| Attempts                                                    | **10**         |
| Uncaught exceptions                                         | **0**          |
| Blank window (`#root` empty / `body` empty)                 | **0**          |
| Cross-mode payload rendered (mode-A answer shown as mode B) | **0**          |
| Result                                                      | **10/10 PASS** |

View markers were asserted inside the CODE NAVIGATION panel only, so a stale number could not
be read off an unrelated part of the UI: references `unattributed name matches`, call hierarchy
`/\d+ unsupported/`, impact `/\d+ direct/`, dependency graph `/of \d+ edges/`.

Evidence: `%TEMP%\opencode\cdp\journey_k.mjs`, `journey_k_evidence.json`.

#### 15.6 Agent citation commissioning — `AGENT_VERIFIED = BLOCKED`

**The turn that was run** (real product path, `handle_build` → `process_intent` →
`AgentLoop::run`), from `C:\Projects\zylcode`:

```powershell
$env:ZYLCODE_FALLBACK_MODEL = "qwen3:0.6b"
$env:ZYLCODE_VECTOR_CACHE_PATH = "$env:TEMP\opencode\vc_agent_a.db"   # deleted first
.\target\debug\zylcode.exe --workspace C:\Projects\zylcode build `
  --prompt "What code activates the repository intelligence surface and where is that activity mapped?" `
  --output json
```

Exit `1` (the loop failed at the tool step with `error: Tool not found: fs.read`).
Log: `telemetry:fallback provider=openrouter … status=401` →
`telemetry:provider_failover from=openrouter to=ollama`.

**1. Citations really were computed.** `.zylcode/intelligence-index.json` was rewritten at
04:39:07Z — 21 s into a turn that started 04:38:46Z — which only happens inside
`ContextBuilder::build`, and `agent_citation_block` runs unconditionally immediately after it.
The citations existed in `session.messages` for that exact goal in that exact turn.

**2. The model never saw them.** The vector cache holds exactly one row — the prompt that was
actually dispatched:

```
prompt_len = 519
sha256(prompt) = 346cef973c84c1fc5e6453406bf207f4c26e817c07db0d2d8b29bdf749238141
```

```
You are an expert software engineer. Your task is to: What code activates the repository
intelligence surface and where is that activity mapped?

Based on the context provided, create a detailed plan to accomplish this task.
Return a JSON object with action 'Plan' and a list of steps.
… (the plan template)
```

Scanned against both prompt and response: `DETERMINISTIC` **false**, `HEURISTIC` **false**,
`Repository graph evidence` **false**, `interpretation` **false**, `.rs:` **false**. The
response is a bare Plan JSON with `expected_files: ["src/main.rs"]` and no citations.

**3. Why — the delivery gap, exhaustively.** `gather_context` (`agent.rs:693`) appends the
block as a system message at `session.messages[1..]`, but every model-facing call builds its
prompt only from `messages[0]` (the raw user task, pushed at `AgentLoop::new`): `generate_plan`
(`:740`), `get_next_tool_call` (`:1310`), `verify_results` (`:964`), `repair_errors` (`:1148`).
`execute_plan` (`:797`) is `#[allow(dead_code)]` and unreferenced. `RealModelClient::call` →
`router.dispatch_prompt(prompt, system)` takes two strings; no transcript is serialised
anywhere outside tests. Grep over all `.rs`: `agent_citation_block` has exactly one consumer,
`ContextBuilder::build` (`context_builder.rs:43`); the only production consumers of
`navigation_citations` are `agent.rs:714` and `:721`, which write to `session.messages`.

So this is **not** a computation failure and **not** a heuristic-vs-deterministic labelling
failure — nothing was laundered, because nothing was shown at all. It is a delivery failure.
`AGENT_VERIFIED` is recorded **`BLOCKED`**, not `UNVERIFIED` and not `TESTED`: the blocker is
identified to the line, and closing it means changing which history a model request reads —
an agent-loop behaviour change, i.e. next wave (G3).

> **Items 1–3 above are the pre-fix record, kept as written.** The delivery gap they describe
> was closed in the 2026-09-29 remediation pass; the turn is repeated as **Journey L** below
> with the read-back. The _level_ `AGENT_VERIFIED` nevertheless remains `BLOCKED` — for the two
> reasons stated in Journey L, neither of which is waived.

Evidence: `%TEMP%\opencode\vc_agent_a.db`, `agent_turn_a_out.txt`, `agent_turn_a_err.txt`,
`dump_vc.py` / `read_cache.py`.

##### Journey L — the same turn repeated after the G3 fix (2026-09-29)

**The fix.** Citations used to be written into a session _message_ that no model request ever
read (§15.6 item 3). Now:

- `SessionContext.navigation_citations` carries them (`#[serde(default)]`), and `gather_context`
  replaces it on every turn.
- `repository_context_block()` renders them into a prompt block: `REPOSITORY INTELLIGENCE
CONTEXT` header, `EVIDENCE · symbol @ file:line — relation — excerpt` lines,
  `REPO_CONTEXT_MAX_CITATIONS = 12`, `REPO_CONTEXT_MAX_CHARS = 4_096`, **whole-line**
  truncation (never a partial citation), the drop is announced when it happens, `None` when
  empty, and the header switches to `No graph evidence is available for this task:` when no
  kept line starts with `DETERMINISTIC`.
- `with_repository_context()` prepends that block to the task — the protocol instruction stays
  last — and `AgentLoop::provider_request()` is the **single dispatch boundary** every
  model-facing call goes through (`ProviderRequest { prompt, system }`).

**Unit + mutation proof.** 8 new G3 tests; `cargo test -p zylcode-core --offline --lib --
agent::tests` → **15 passed / 0 failed** (7 pre-existing + 8 new). Mutation proof: make
`with_repository_context` return `prompt` unchanged → **4 tests failed**; original restored.

**The turn.** Real product path, same command shape as §15.6, fresh vector cache:

```powershell
$env:ZYLCODE_FALLBACK_MODEL  = "qwen3:0.6b"
$env:ZYLCODE_VECTOR_CACHE_PATH = "$env:TEMP\opencode\vc_g3_journey_l.db"   # deleted first
.\target\debug\zylcode.exe build --workspace . --verbose `
  --prompt "What code activates the repository intelligence surface and where is that activity mapped?" `
  --output text
```

Exit `1` in 53.8 s with `error: Tool not found: fs.read` — the **same pre-existing CLI
tool-registry gap as §15.6** (the plan template's own example step carries
`"expected_tools": ["fs.read"]`, and no `fs.read` tool is registered). It is unrelated to G3
and is recorded here rather than suppressed.

Provider path: `telemetry:fallback provider=openrouter … status=401` →
`telemetry:provider_failover from=openrouter to=ollama` → a real generation on local `ollama`
with `ZYLCODE_FALLBACK_MODEL=qwen3:0.6b`, which wrote exactly one row to the vector cache.

**Read-back — the dispatched prompt, from disk:**

```
prompt_len            = 1778
stored prompt_hash    = 32c565020dfeff617bfd5a72235f808f33b228a4b38367b5c4872135a9e11bff
sha256(prompt_text)   = 32c565020dfeff617bfd5a72235f808f33b228a4b38367b5c4872135a9e11bff  (identical)
```

`REPOSITORY INTELLIGENCE CONTEXT` **true**, `DETERMINISTIC` **true**, a `file:line` **true**,
`Instructions:` **true**, the task **after** the block, **4** `DETERMINISTIC` citation lines:

```
REPOSITORY INTELLIGENCE CONTEXT

Deterministic evidence resolved from the parsed repository index:
DETERMINISTIC · src::lib::useArtifactStream::parseFenceBlock::code @ apps/zylcode-desktop/src/lib/useArtifactStream.ts:148 — declared — code = lines.slice(1).join("\n")
DETERMINISTIC · crate::intelligence @ crates/zylcode-core/src/lib.rs:22 — declared — pub mod intelligence;
DETERMINISTIC · crate::intelligence @ apps/zylcode-desktop/src-tauri/src/main.rs:217 — call via module path `zylcode_core` -> `crates/zylcode-core/src/lib.rs` — zylcode_core::intelligence::api::repo_intel_payload(&root, &task)
DETERMINISTIC · crate::intelligence @ apps/zylcode-desktop/src-tauri/src/main.rs:240 — call via module path `zylcode_core` -> `crates/zylcode-core/src/lib.rs` — zylcode_core::intelligence::nav_api::nav_payload(&root, &tool, args)
… (protocol instructions) …
You are an expert software engineer. Your task is to: What code activates the repository
intelligence surface and where is that activity mapped?
```

**Delivery is therefore proven at runtime, not inferred:** the row keyed by `sha256(prompt)`
_is_ the prompt handed to the provider, and it carries the block.

**All four `file:line` claims re-read from disk — 4/4 OK:**

| Citation                                                | Line on disk                                                           |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| `apps/zylcode-desktop/src/lib/useArtifactStream.ts:148` | `const code = lines.slice(1).join("\n");`                              |
| `crates/zylcode-core/src/lib.rs:22`                     | `pub mod intelligence;`                                                |
| `apps/zylcode-desktop/src-tauri/src/main.rs:217`        | `zylcode_core::intelligence::api::repo_intel_payload(&root, &task)`    |
| `apps/zylcode-desktop/src-tauri/src/main.rs:240`        | `zylcode_core::intelligence::nav_api::nav_payload(&root, &tool, args)` |

**Consumption is proven too:** the model's plan named `apps/zylcode-desktop/src/lib/useArtifactStream.ts`
and `crates/zylcode-core/src/lib.rs` in `expected_files` — neither string appears anywhere in
the task — and step 2's `verification` reads _"Check line references in the provided context"_.
The citations reached the model and were read.

**`AGENT_VERIFIED` remains `BLOCKED`, and is not promoted.** Two independent reasons, neither
waived:

1. **§17.2's own criterion is not met.** The recorded condition for `AGENT_VERIFIED` was "the
   response actually cites `DETERMINISTIC` `file:line` rows that were independently checked on
   disk." The response has `DETERMINISTIC` **false** and any `:<digits>` **false** — it is a
   bare Plan JSON. Delivery is proven; self-citation is not.
2. **Provider outage.** Every cloud path is unusable in this environment (OpenRouter `401`
   placeholder key, Anthropic `401`, DeepSeek `402`), so the intended provider never served a
   turn and only the local fallback did. The gate is not weakened to accommodate that.

Promotion now needs a turn from a healthy provider whose response carries `DETERMINISTIC`
`file:line` rows. Everything up to that point is recorded as `RUNTIME_VERIFIED`, not higher.

Evidence: `%TEMP%\opencode\vc_g3_journey_l.db`, `journey_l_out.txt`, `journey_l_err.txt`,
`dumpvc.mjs`, `journey_l_verify.mjs`.

#### 15.7 Browser-preview commissioning — journey M (G2)

**The defect, confirmed before anything was changed.** `queryNav` branches on
`env === "TAURI_DESKTOP"`: desktop → `invoke("nav_query")`, everything else → the service
fetch. The browser-preview side of that branch was never exercised. With `serve-intel` up on
17630:

| Request                                                                             | Result                      |
| ----------------------------------------------------------------------------------- | --------------------------- |
| `POST http://127.0.0.1:17630/api/nav`                                               | **200**, `application/json` |
| `POST http://localhost:1420/api/nav` (what `queryNav` actually issues in a browser) | **404**                     |

So the browser path was broken in the dev preview: `apps/zylcode-desktop/vite.config.ts` had
no proxy entry for `/api/nav`.

**The fix.** One line in `vite.config.ts`:

```
"/api/nav": "http://127.0.0.1:17630"
```

This is the **only new file added to `git status` by this remediation pass** — 25 modified →
26 modified (41 status lines total, untracked count unchanged at 15). It is reported here
rather than folded into an earlier count.

**Unit tests — transport selection.** `navIntel.test.ts` gained
`describe("queryNav (transport selection)")` with **3 tests** (file total 11 → 14): _routes a
Tauri desktop query through `nav_query` IPC and never through HTTP_, _routes a non-desktop
query through `fetch("/api/nav")` and never through IPC_, _classifies an IPC refusal as
`rejected` rather than an empty answer_.

**Mutation proof 1 — delete the branch condition.** `if (env === "TAURI_DESKTOP")` →
`if (env === "TAURI_DESKTOP" && false)`: **2 tests failed**. Restored.

**Mutation proof 2 — collapse the branch.** `if (true)`, forcing every query through IPC:
**5 tests failed**. Restored; `navIntel.ts` re-read afterwards and is byte-identical to
`if (env === "TAURI_DESKTOP") {`.

**Journey M — the real browser branch, network + runtime evidence.** A dedicated headless
Chrome on `:9444` (profile `%TEMP%\opencode\journey_m_prof`, stopped afterwards per §30) drove
the Vite dev server:

- `navigator` shows **no** `__TAURI_INTERNALS__` → the app took the browser path, not desktop.
- `POST http://localhost:1420/api/nav` with body
  `{"tool":"find_references","args":{"name":"query_repository","limit":500}}` →
  **200**, `application/json`, through the Vite proxy to `serve-intel`.
- The panel rendered `8 deterministic`, **9** location buttons, and the file groups
  `crates/zylcode-nav/src/cache.rs (8)` / `crates/zylcode-nav/src/lib.rs (1)` — the same
  answer the desktop transport gives (§15.5 journey F), reached over a different transport.
- **0** material console errors (no uncaught exception, no failed request beyond the known
  informational `[vite]` chatter).
- Result: **6/6 PASS**.

`tauri.conf.json` was **not** touched for this (§29): SHA-256 still
`E0886F8897122B0F9AA364FB9E9E56C61D38E443866F52F3EC7A36295EC52C6E`, no `additionalBrowserArgs`.

Evidence: `%TEMP%\opencode\cdp\journey_m.mjs`, `journey_m_evidence.json`.

---

### 16. Gaps and known-stale records

| #   | Gap                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Class                                                                               |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| G1  | ~~Tauri desktop window never launched~~ — closed this session: launched via `pnpm dev` + `zylcode-desktop.exe` and driven through journeys A–J (§15.5)                                                                                                                                                                                                                                                                                                                                                                                                                                        | **CLOSED — RUNTIME_VERIFIED**                                                       |
| G2  | ~~Browser preview never run against a live `serve-intel`~~ — closed this pass: journey M drove `queryNav`'s `fetch("/api/nav")` branch in a real browser against the Vite proxy → `serve-intel`, HTTP 200, 6/6; the underlying 404 was a missing proxy entry in `vite.config.ts`, fixed, with 3 transport tests and 2 mutation proofs (§15.7)                                                                                                                                                                                                                                                 | **CLOSED — RUNTIME_VERIFIED**                                                       |
| G3  | ~~Citations computed but excluded from every model request~~ — closed this pass: `AgentLoop::provider_request()` is now the single dispatch boundary and prepends `repository_context_block()`; journey L read the dispatched prompt back from disk (1778 chars, `prompt_hash` = `sha256(prompt_text)`, 4 `DETERMINISTIC file:line` rows), the 4 citations re-read from disk 4/4, and the model's plan named two of them (§15.6). **The `AGENT_VERIFIED` level itself stays BLOCKED**: the response carried no `DETERMINISTIC file:line` of its own and every cloud provider is unusable here | **CLOSED for delivery — `RUNTIME_VERIFIED`; `AGENT_VERIFIED` still BLOCKED**        |
| G4  | `RECONCILIATION_FORENSIC_REPORT.md` line 144 counts ("12 executable + 21 proposed") are stale after the +8 `nav.*` entries; lines 146/148 still hold (§12.6)                                                                                                                                                                                                                                                                                                                                                                                                                                  | **DECIDED — `HISTORICAL_RECORD`, left byte-for-byte unchanged; reasoning in §12.6** |
| G5  | `gen/schemas/*.json` regenerate key order on every build of `src-tauri`; content-identical but dirty in `git status`                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | **INCIDENTAL / generated**                                                          |
| G6  | The intelligence warm-start `create_dir_all(<root>/.zylcode)` creates a missing workspace root (§11.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | **CLOSED — decision (B), fixed fail-closed with 3 regression tests (§11.3)**        |
| G7  | 1-second staleness window means a query immediately after an edit may be answered from the previous index (§13.2)                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | **ACCEPTED TRADE-OFF**                                                              |
| G8  | Rust/TS/JS only; other languages fall through to "unsupported file", counted but never guessed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | **ACCEPTED SCOPE**                                                                  |
| G9  | `member call` attribution cannot go deterministic without type information the parser does not have                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | **ACCEPTED LIMIT — reported as HEURISTIC**                                          |
| G10 | `r3_verified_count` remains 0 (§12.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | **ACCEPTED**                                                                        |
| G11 | ~~`RepoNavPanel` crashes fatally when a tab is switched while a query is in flight: `ImpactView` reads `data.items` on a payload that belongs to another tool, throws, and the app has no error boundary — the window goes blank~~ — closed this pass: request-ownership token (stale responses are discarded, never applied), per-tab discriminated payload typing so the compiler blocks cross-mode misuse, and `NavErrorBoundary` as containment; 5 regression tests, 2 mutation proofs, and journey K **10/10 races clean** (§15.5b)                                                      | **CLOSED — RUNTIME_VERIFIED**                                                       |

---

### 17. Next wave

1. ~~**Fix G11 first**~~ — **done 2026-09-29.** `run()` carries a monotonic request token so a
   stale response can never be applied after `tab` has moved on; `RepoNavState` is a
   discriminated per-tab union so a mode-A payload cannot be consumed as a mode-B result; and
   `NavErrorBoundary` wraps `case "intel"` so no render error can unmount the app. 5 regression
   tests, 2 mutation proofs, journey K **10/10 races clean** (§15.5b).
2. ~~**Close G3 (citations reaching a model)**~~ — **done 2026-09-29.** `AgentLoop::provider_request()`
   is the single dispatch boundary and prepends `repository_context_block()`; journey L read the
   dispatched prompt back from disk and all four `file:line` rows re-read from disk (§15.6).
   **The `AGENT_VERIFIED` criterion above was deliberately _not_ claimed:** the response did not
   carry `DETERMINISTIC` `file:line` rows of its own, and every cloud provider path is unusable
   in this environment. That row stays `BLOCKED` until a healthy provider answers with self-cited
   deterministic rows.
3. ~~**Close G2**~~ — **done 2026-09-29.** Journey M drove `queryNav`'s browser branch through
   the Vite proxy to a live `serve-intel`: HTTP 200, 6/6, plus 3 transport tests and 2 mutation
   proofs (§15.7).
4. **Decide G5** — commit the generated-schema reorder once so builds stop dirtying the tree.
   This is now the only gap still awaiting a decision; G7–G10 are accepted trade-offs.
5. **Done across the sessions, for the record**: G1 → `RUNTIME_VERIFIED` (§15.5); G2 →
   `RUNTIME_VERIFIED` (§15.7); G3 → delivery `RUNTIME_VERIFIED`, level `BLOCKED` (§15.6); G4 →
   `HISTORICAL_RECORD`, left byte-for-byte unchanged (§12.6); G6 → decision (B), fixed
   fail-closed (§11.3); G11 → `RUNTIME_VERIFIED` (§15.5b). No action remains on those six.
6. **Owner review, then commit** — the accepted scope was staged path-by-path (never
   `git add -A`) and committed locally as a single checkpoint commit on 2026-09-29.
   **Nothing was pushed.** The working tree is _deliberately_ still dirty afterwards: the
   three `gen/schemas/*.json` files (G5) and the five preserved untracked paths (§1.1) were
   left unstaged on purpose. The tree grew by exactly one modified file during the remediation
   pass — `apps/zylcode-desktop/vite.config.ts`, the G2 proxy entry (§15.7) — 25 → 26 modified
   files, 41 status lines, untracked unchanged at 15.

#### 17.1 Carry-forward items — explicitly NOT part of the 2026-09-29 checkpoint

The owner's acceptance of 2026-09-29 covers the remediation scope (G11, G2, and G3's
delivery/consumption path) only. It does **not** waive any evidence gate. Two items are
carried forward; neither is fixed, waived or claimed by the checkpoint commit:

| ID           | Item                                                                                                                                                                                                                                                                                                                                                                                                                                 | State                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **AGENT-01** | Complete genuine `AGENT_VERIFIED` commissioning on a successful **real provider** turn that satisfies the deterministic output-citation criterion quoted in §15.6: the response must itself cite `DETERMINISTIC` `file:line` rows that were independently checked on disk. Today the response does not self-cite, and no usable cloud provider exists in the commissioning environment (OpenRouter/Anthropic `401`, DeepSeek `402`). | **OPEN — `AGENT_VERIFIED` stays `BLOCKED`**                                                                                                                                                                                                                                                                                                                                                                                                                |
| **TOOL-01**  | Resolve the pre-existing `fs.read` tool-name/dispatch mismatch so the agent's own planned tool invocation is actually executable. Both §15.6 and journey L ended `error: Tool not found: fs.read` _after_ repository-context delivery and consumption had already been proven; the CLI registers no `fs.read`, and the plan template's own example step names it.                                                                    | **CLOSED — commit `1a0c79d`: `process_intent` registers every catalogue-executable built-in before the loop is built (idempotent, definition-only ids excluded); `fs.read` is workspace-contained (refuses `..`/absolute/symlink escapes), bounded (1 MiB, enforced at read time), and every call — success or refusal — is persisted as evidence; e2e proves the first planned tool step now executes (crates/zylcode-core/tests/executable_tools_e2e.rs) |

Canonical wording is unchanged: §15.6 calls it "the pre-existing CLI tool-registry gap" and
records `AGENT_VERIFIED` as `BLOCKED`. The two IDs above are the owner's; no parallel tracking
system is introduced.

---

## Appendix A — Wave requirement → section map

| Work-order § | Deliverable                             | Section |
| ------------ | --------------------------------------- | ------- |
| 2            | baseline recorded before implementation | §1      |
| 3            | deterministic first                     | §2, §3  |
| 4            | symbol identity model                   | §4      |
| 5            | find references + navigation            | §5      |
| 6            | callers / callees                       | §6      |
| 7            | import / export graph                   | §7      |
| 8            | dependency graph with provenance        | §8      |
| 9            | impact analysis                         | §9      |
| 10           | UI                                      | §10     |
| 11           | AI integration                          | §11     |
| 12           | agent tools                             | §12     |
| 13           | performance / incremental index         | §13     |
| 14           | tests                                   | §14     |
| 15           | evidence states                         | §15     |
