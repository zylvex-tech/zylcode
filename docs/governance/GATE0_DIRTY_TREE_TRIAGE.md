# GATE-0 DIRTY-TREE TRIAGE

**Question this document answers:** Gate-0 acceptance criterion 8 —
*"no required source in dirty/untracked."*

**Verdict: the criterion FAILS.** One required product asset is untracked, and the dirty
tree is not a superset of the candidate — it is a divergent, earlier state that would
revert two accepted repairs if committed.

**No action was taken.** This document characterises; it does not clean, commit, discard,
or move anything. The owner's Decision 4 stands: *the dirty tree needs preservation, not
cleanup.*

---

## 1. The measurement

At `639dbbb` (primary working tree, `main`):

| Class | Count |
|---|---|
| Modified tracked files | **56** |
| Untracked entries (`git status`, directory-collapsed) | **35** |
| Untracked files (individually) | **37** |
| Files modified by **both** the dirty tree and the candidate | **25** — and all 25 differ |

The 25-file overlap is the important number. It is not coincidence: the dirty tree
contains an *earlier, parallel* version of the same clippy remediation, produced by a
different session before Gate-0 was authorised.

---

## 2. `mcp.tools.yaml` — required, untracked, never added

This is the finding that fails the criterion.

**The file is the product's tool surface.** It is loaded by default at runtime:

| Site | Code |
|---|---|
| `crates/zylcode-mcp/src/lib.rs:73` | `std::env::var("ZYLCODE_MCP_CONFIG").unwrap_or_else(\|_\| "mcp.tools.yaml".to_string())` |
| `apps/zylcode-desktop/src-tauri/src/main.rs:425` | `// Auto-load tools from mcp.tools.yaml` |
| `apps/zylcode-desktop/src-tauri/src/main.rs:741` | `// Auto-load tools from mcp.tools.yaml if present (non-fatal).` |
| `apps/zylcode-desktop/src/components/McpInspector.tsx:87` | UI: *"No tools registered. Add `mcp.tools.yaml` → hot-reload."* |

Without it the desktop application registers **zero** tools and says so.

**It is not in git, and it is not ignored** — `git check-ignore -v mcp.tools.yaml` returns
nothing and `.gitignore` has no entry. It was simply never added.

### 2.1 What it contains

30 tool definitions, each with a real `command` + `args`, in contrast to the bridge's 27
simulated definitions:

```
git.commit  git.push  git.pull  git.status  git.diff
code.analyze  code.format  code.lint
test.run  test.single  build.debug  build.release
npm.install  npm.run
fs.read  fs.write  fs.list  search.grep  search.find
docker.build  docker.run  k8s.get  k8s.apply  db.query
ai.train  ai.evaluate  slack.send  security.scan  perf.bench  docs.generate
```

Commands are real executables: `cargo check`, `cargo audit`, `cargo bench`, `cargo doc`,
`kubectl get`, `sqlite3`, `docker`, `git`, `npm`.

**Two are not functional:**

| id | command | problem |
|---|---|---|
| `ai.train` | `python train.py --model {{model}}` | `train.py` **does not exist** in the repository |
| `ai.evaluate` | `python evaluate.py --model {{model}}` | `evaluate.py` **does not exist** in the repository |

These are the same class of defect as the bridge simulator: a tool that appears in the
catalogue, is enabled, and cannot work. They are the only two of the thirty.

### 2.2 Why this matters more than the bridge

`TOOL_CATALOGUE_TRUTH_TABLE.md` established that the 27 bridge definitions are unreachable
from the product. This file is the opposite: it **is** the product path, it is **live**, and
it is **not in the repository**. The earlier conclusion — "no `mcp.tools.yaml` is committed,
so the shipped tool set is currently empty" — is confirmed and now explained.

---

## 3. The dirty tree is divergent, not a superset

Measured on `crates/zylcode-mcp/src/enhanced_bridge.rs` and across the workspace:

| Axis | Dirty tree (`639dbbb` + WIP) | Candidate (`c3dedad`) |
|---|---|---|
| `ToolDefinition {` sites in the bridge | **113** | **28** |
| `fn get_more_*_tools` implementations | **8** | **0** |
| `pub mod performance;` / `pub mod system_integration;` | **present** (`lib.rs:10,15`) | **removed** (FIX-1) |
| `run_benchmark_binary` (subprocess CLI test) | **11 references** | **0** (Track 2) |
| `generate_benchmark_report` (library API) | **0** | present (Track 2) |
| `NETWORK_EGRESS_COUNT` (router spy) | **0** | present (Track 3) |
| `synthetic_config` (bench hermeticity) | **0** | present (Track 3b) |
| claim guard scans `git ls-files` | **0** | present (Track 5) |
| rustfmt applied | **yes** | no |
| `cargo clippy --workspace --all-targets -D warnings` | **passes** | **passes** |
| `cargo test -p zylcode-mcp --lib` | **35 passed / 0 failed** | 29 passed / **2 failed** |
| `cargo test --workspace --all-targets` | **FAILED** | **FAILED** |

The dirty tree is **ahead on exactly one axis**: it carries the 113-tool surface that
FIX-1 and FIX-2 deliberately removed. On every remediation axis it is **behind**.

### 3.1 The dirty tree fails its own test suite

```
$ cargo test --workspace --all-targets        # in the primary working tree
thread 'router::tests::synthetic_offline_dispatch_returns_parseable_payload'
  panicked at crates\zylcode-core\src\router.rs:1059:14
test result: FAILED. 225 passed; 1 failed
```

That is precisely the Track 3 defect: with `OPENROUTER_API_KEY=YOUR_OPENROUTER_API_KEY` in
the ambient environment, the router leaves the offline path and `.unwrap()` panics. The
dirty tree has the defect; the candidate has the repair.

### 3.2 Why the 113 tools exist

`enhanced_bridge.rs` in the dirty tree contains eight `get_more_*_tools()` implementations
and eight corresponding call sites. Their absence from the candidate is not an omission —
FIX-2 removed the call sites because the implementations were **never committed**, which is
why `origin/main` did not compile.

The dirty tree compiles only because the untracked `performance.rs` and
`system_integration.rs` satisfy the `pub mod` declarations that FIX-1 removed from
`lib.rs`. **The dirty tree is load-bearing on untracked files in two independent places.**

This also explains the four assertions in `TOOL_CATALOGUE_TRUTH_TABLE.md` §0: they were
calibrated to the dirty tree (113 > 100, and `hot_reload`'s count ≥ 150 passes there). They
have never been satisfiable from the repository.

---

## 4. Disposition — recommendation only, no action taken

| Group | Contents | Recommended disposition |
|---|---|---|
| **A. Required product asset** | `mcp.tools.yaml` (30 tools) | **Owner decision.** Either commit it (after fixing `ai.train` / `ai.evaluate`) or add it to `.gitignore` and commit a `mcp.tools.example.yaml`. Leaving it untracked means the product ships with no tools. |
| **B. Superseded remediation** | the 25 files modified by both trees | **Do not commit on top of the candidate.** The candidate reproduces this work. Committing the dirty versions would revert Tracks 2, 3, 3b and the guard fix. |
| **C. FIX-1/FIX-2 reversals** | `performance.rs`, `system_integration.rs`, the 8 `get_more_*` blocks | **Owner decision.** These are the 113 simulated tools. Re-instating them re-creates fabricated execution; see `TOOL_CATALOGUE_TRUTH_TABLE.md` §5. |
| **D. Legacy status reports** | `PHASE3_*`, `PHASE4_*`, `INSTALLATION_*`, `USER_GUIDE`, `DEVELOPER_GUIDE`, `*_SUMMARY.md` | Archive or delete. Already quarantined by `docs/README_INDEX.md`. Not published, not cited. |
| **E. This session's governance docs** | `STATUS_SWEEP`, `REPOSITORY_INTEGRITY_RECOVERY`, `GATE0_REMEDIATION_FORENSIC`, `PARALLEL_EXECUTION_MANIFEST`, `recovery-f1b2b36.patch` | **Commit.** They are the evidence trail for Gate-0 and are currently untracked. |
| **F. Scratch / local tooling** | `.dsh-vision-toolkit/`, `*.ps1`, `*.bat`, `diagnose*`, `test-*.ps1` | Local developer tooling. Add to `.gitignore` or delete. Not product source. |

---

## 5. Answer to the Gate-0 criterion, stated plainly

> *"no required source in dirty/untracked"*

**FAILS.** `mcp.tools.yaml` is required by the shipped product, is loaded by default at
runtime, and is untracked and unignored. It is the only required product asset in the
dirty set — the candidate builds and passes 267 `zylcode-core` tests from a clean checkout
with no untracked files present, so nothing else in the set is build-required.

**Additional hazard.** The 25-file overlap means the dirty tree cannot be committed
wholesale without reverting accepted repairs. Any future integration must reconcile file by
file, not tree by tree.

---

## 6. Reproduction

```bash
# counts
git diff --name-only | wc -l                        # 56 modified tracked
git status --porcelain | grep -v '^??' | wc -l      # 56
git status --porcelain | grep -c  '^??'             # 35 untracked entries
git ls-files --others --exclude-standard | wc -l    # 37 untracked files

# mcp.tools.yaml is untracked, unignored, and loaded by default
git check-ignore -v mcp.tools.yaml             # no output
grep -n 'mcp.tools.yaml' crates/zylcode-mcp/src/lib.rs apps/zylcode-desktop/src-tauri/src/main.rs

# the dirty tree is the 113-tool state
grep -c 'ToolDefinition {' crates/zylcode-mcp/src/enhanced_bridge.rs   # 113
git show c3dedad:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -c 'ToolDefinition {'   # 28
grep -c 'fn get_more_' crates/zylcode-mcp/src/enhanced_bridge.rs       # 8

# the dirty tree does not contain the candidate's repairs
grep -c run_benchmark_binary crates/zylcode-core/src/cli.rs            # 11
grep -c NETWORK_EGRESS_COUNT crates/zylcode-core/src/router.rs         # 0
grep -c synthetic_config     crates/zylcode-core/benches/router_cache.rs  # 0
grep -c 'ls-files'           scripts/check_retracted_claims.py         # 0

# the dirty tree fails its own suite
cargo test --workspace --all-targets   # FAILED: router.rs:1059

# two catalogue entries reference files that do not exist
grep -A2 'id: "ai.train"' mcp.tools.yaml ; ls train.py   # No such file
```

---

## 7. UPDATE — collisions introduced by the tool consolidation

Added after the Option C consolidation (owner decision). Per the owner's
instruction: *"Separately update `docs/governance/GATE0_DIRTY_TREE_TRIAGE.md`
with any new collision discovered."*

### 7.1 Nine further modified-file collisions

The consolidation touched ten files. **Nine of them are also modified in the
primary working tree:**

```
crates/zylcode-mcp/src/enhanced_bridge.rs
crates/zylcode-mcp/src/executor.rs
crates/zylcode-mcp/src/hot_reload.rs
crates/zylcode-mcp/src/lib.rs
crates/zylcode-mcp/src/real_tools.rs
crates/zylcode-mcp/src/tool.rs
crates/zylcode-mcp/tests/fault_injection.rs
crates/zylcode-mcp/tests/integration_test.rs
crates/zylcode-mcp/tests/telemetry_audit_tests.rs
```

Only `crates/zylcode-mcp/src/tool_catalogue.rs` is new and therefore
collision-free.

Combined with the 25 collisions recorded in §1, the two trees now overlap on
**34 tracked files**, all of them differing.

### 7.2 A new kind of collision: `mcp.tools.yaml`

The consolidation commits a **regenerated** `mcp.tools.yaml` at the repository
root (disposition: REGENERATE —
`docs/governance/MCP_TOOLS_YAML_DISPOSITION.md`).

The primary working tree holds a different, **untracked** file at the same path.
This is a collision between a tracked file and an untracked one, which `git`
will report as an overwrite-on-checkout rather than a merge conflict. It is
therefore easy to miss, and worth stating explicitly.

**The two files are not equivalent:**

| | primary working tree (untracked) | committed (regenerated) |
|---|---|---|
| entries | 30 | **12** under `tools:`, 21 under `proposed:` |
| entries with a real executor | 9 | **12** (all of them) |
| broken script references | 2 (`ai.train`, `ai.evaluate`) | 0 (recorded under `proposed:`) |
| effect if used as-is | 21 entries fail closed at call time | every entry executes |

Resolution is an owner decision. The recommendation is to adopt the regenerated
file and move the 21 unsupported entries to `proposed:` — but the dirty tree is
preserved, not cleaned, so nothing was overwritten.

### 7.3 Why the collision count keeps growing

Each round of remediation adds collisions, because the dirty tree contains a
**parallel, unreviewed version of the same subsystem**. This is the expected
consequence of the owner's Decision 4 (preserve the dirty tree) combined with
continued work on the candidate.

It is manageable while the overlap is enumerated. It stops being manageable the
moment anyone attempts a tree-level merge. **Reconcile file by file, never tree
by tree.**

### 7.4 Current totals

| Measure | Count |
|---|---|
| Files modified in the primary working tree | 56 |
| Untracked entries in the primary working tree | 35 (37 files) |
| Files modified by **both** the dirty tree and the candidate | **34** |
| Untracked-in-main / tracked-in-candidate path collisions | **1** (`mcp.tools.yaml`) |
