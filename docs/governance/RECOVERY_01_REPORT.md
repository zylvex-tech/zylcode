# RECOVERY-01 — Current-State Reconnaissance and Preservation

> **STOP FOR INDEPENDENT REVIEW.** This is a reconnaissance report. No destructive commands were executed. No commits were created. The user will independently challenge SHA, collision map, tool counts, dirty-tree conclusions, and test evidence before authorizing RECOVERY-02.

---

## UNIT
RECOVERY-01 — Current-State Reconnaissance and Preservation

## BASE SHA
`639dbbb397d9052d9aa0376b3ef6a734f176c52c` (HEAD, branch `main`)

## FINAL SHA
`639dbbb397d9052d9aa0376b3ef6a734f176c52c` — **no change**. This is reconnaissance only. No files were modified, no commits were created, no branches were pushed.

## OBJECTIVE

Establish the ground truth of the repository as it exists right now, without trusting any prior document, audit, or completion report. Specifically:

1. Pin the exact SHA of HEAD, origin/main, and the Gate-0 remediation candidate.
2. Map the git topology — is the candidate a descendant of HEAD, or do they diverge?
3. Produce a collision map: files modified in BOTH the dirty working tree AND the candidate.
4. Re-measure the forensic findings from the Gate-0 audit against the actual candidate code.
5. Verify the candidate compiles, passes tests, and passes clippy.
6. Determine which findings the candidate already addresses and which remain open.
7. Preserve everything — zero destructive operations.

## FILES CHANGED

**None.** No files were created, modified, or deleted by this reconnaissance unit. (A temporary `collision_check.sh` script was created and removed during analysis; it is not tracked.)

## WHAT WAS ACTUALLY WRONG (Forensic findings re-measured against the candidate)

### Finding 1: Fabricated tool-count assertions — **FIXED in candidate, LIVE in HEAD**

| Assertion | HEAD (live code) | Candidate (removed) |
|---|---|---|
| `assert!(count > 100)` | `enhanced_bridge.rs:1495` | Comment at `:820`: "Replaces `assert!(count > 100)`" |
| `assert!(count >= 150)` | `hot_reload.rs:739` | Comment at `:594`: "Replaces `assert!(count >= 150)`" |

These assertions fire against an actual **27** bridge definitions / **11** development tools. They block `cargo test --workspace --lib` / `--all-targets` on HEAD. The candidate replaced them with semantic invariants.

### Finding 2: Simulated-success tool paths — **FIXED in candidate, LIVE in HEAD**

Three paths fabricated success instead of executing real work:
- `BuiltinTool::call` — `tokio::time::sleep` + `{"success": true}` → **REMOVED** in candidate
- `DynamicTool::call` — `Ok({"status": "unsupported_mock"})` → **REMOVED** in candidate
- `initialize_with_enhanced_tools` — `count + 50` (made `count >= 150` pass) → **REMOVED** in candidate

### Finding 3: `mcp.tools.yaml` — the product's ACTUAL tool surface — **TRACKED in candidate, UNTRACKED in HEAD**

| State | HEAD | Candidate (`34d29a9`) |
|---|---|---|
| `git ls-files mcp.tools.yaml` | (empty — untracked) | `mcp.tools.yaml` (tracked) |
| Status | Untracked, unignored | Tracked, regenerated from canonical catalogue |

Without this file, the shipped desktop app registers **zero** tools. It contains 30 tools with real commands (`cargo check`, `kubectl get`, `sqlite3`, `docker`, `git`, `npm`). Two tools (`ai.train`, `ai.evaluate`) invoke scripts that do not exist.

### Finding 4: Tool catalogue modules — **PRESENT in candidate, ABSENT in HEAD**

| Module | Candidate lines | HEAD |
|---|---|---|
| `tool_catalogue.rs` | 881 | Does not exist |
| `permission.rs` | 371 | Does not exist |
| `evidence.rs` | 366 | Does not exist |
| `actor.rs` | 106 | Does not exist |

The candidate consolidated tool definitions into one canonical catalogue. HEAD has none of these modules.

### Finding 5: Retracted-claim guard — **PASSES in candidate, FAILS in HEAD**

| Guard result | HEAD working tree | Candidate worktree |
|---|---|---|
| Exit code | 1 (FAIL) | 0 (PASS) |
| New occurrences | 135 | 0 |
| Baseline occurrences | — | 11 (11 baseline lines) |

HEAD's guard fails because `.wt/` worktree directories (`.wt/audit-34d29a9/`, `.wt/remediation/`, `.wt/verify/`, `.wt/fix1/`) are being scanned by the filesystem-based guard. The candidate's guard was fixed in commit `d1c0a15` ("scan tracked files only") to only scan git-tracked files.

### Finding 6: Computer-Use Engine simulation facade — **UNCHANGED in candidate (correct)**

The simulation facade at `crates/zylcode-core/src/computer_use/` is byte-identical in both HEAD and candidate. This is **correct per governance**: "Replace, do not extend." The candidate did not touch it. It remains R0.

Fabricated confidences (`0.85`, `0.9`, `0.95`), `vec![0; 1920 * 1080 * 4]` screen captures, and `tokio::time::sleep` actions are present in both trees.

### Finding 7: Legacy root .md file chaos — **NOT FIXED in candidate**

| Root .md files | HEAD | Candidate |
|---|---|---|
| Tracked | 48 | 47 |
| Untracked | 16 | 0 (clean tree) |
| Total | 64 | 47 |

The candidate has fewer root .md files because untracked legacy files do not exist in its clean tree. However, the tracked legacy completion docs (`FINAL_SUMMARY.md`, `PHASE3_COMPLETE.md`, etc.) are **still present** in the candidate. The quarantine called for in `docs/README_INDEX.md` has **not been done**.

## WHAT CHANGED

**Nothing.** This unit performed reconnaissance only. No destructive commands were executed. The working tree, HEAD, and all worktrees are byte-for-byte identical to their pre-reconnaissance state.

## WHAT WAS DELIBERATELY NOT CHANGED

**Everything.** No files were modified, staged, committed, pushed, or deleted. No worktrees were created or removed. No `.gitignore` entries were changed. The retracted-claim guard was run in read-only mode on both trees. The candidate was inspected via `git show` and worktree reads — no writes to the candidate worktree.

## TESTS EXECUTED

### Candidate worktree (`.wt/remediation`, commit `34d29a9`)

| Command | Result | Detail |
|---|---|---|
| `cargo check --workspace --lib --offline` | ✅ PASS | 1m53s, zero errors |
| `cargo test -p zylcode-mcp --lib --offline` | ✅ PASS | 100 passed, 0 failed |
| `cargo test -p zylcode-core --lib --offline` | ✅ PASS | 229 passed, 0 failed |
| `cargo clippy --workspace --all-targets -D warnings --offline` | ✅ PASS | 5m08s, zero warnings |

**Total: 329 tests passed, 0 failed. Clippy clean.**

### HEAD working tree (commit `639dbbb`)

| Command | Result | Detail |
|---|---|---|
| `cargo check --workspace --lib` | ✅ PASS | Compiles (with dirty-tree modifications) |
| `cargo test --workspace --lib` | ❌ BLOCKED | `enhanced_bridge.rs:1495` fires `assert!(count > 100)` against actual 27; `hot_reload.rs:739` fires `assert!(count >= 150)` |

### Retracted-claim guard

| Tree | Result | Detail |
|---|---|---|
| HEAD working tree | ❌ FAIL | Exit 1, 135 new occurrences (all from `.wt/` worktree scanning) |
| Candidate worktree | ✅ PASS | Exit 0, 11 baselined occurrences |

## CAPABILITY CHANGES

**None.** No capabilities were added, removed, or changed. The capability registry is identical in both HEAD and candidate:

| Status | Count |
|---|---|
| GREEN | 20 |
| DISPUTED | 12 |
| PARTIAL | 1 |
| r3_verified | **0** |

**Reconciliation note:** The prior brutal audit labeled parts of the Agent Kernel as "R3 (real)." The Gate-0 consolidation report says `r3_verified_count = 0`. The reconciliation: the audit's "R3" referred to code that compiles, has tests, and dispatches real commands — but it never verified R3 per the governance definition ("reachable through a named product surface with captured evidence"). **R3 requires product-surface reachability with captured evidence, which has never been demonstrated.** The consolidation report's `r3_verified = 0` is correct per the governance definition. The audit's "R3 (real)" was loose language for "R2 with real execution paths, not simulation." The governance definition governs.

**Tool metrics (candidate, no aggregate "tool count"):**

| Metric | Value |
|---|---|
| definition | 38 |
| executable | 12 |
| tested | 2 |
| product_reachable | 11 |
| r3_verified | **0** |

## CLAIMS CORRECTED

No claims were corrected by this unit (reconnaissance only). The following claims from prior documents are **re-verified as false or misleading**:

1. **"156 tools"** — actual executable count is **12**, definition count is **38**. No single aggregate "tool count" is published in the candidate (by design — the truth table says "every metric names and scopes what it counts").
2. **"R3 verified" (Agent Kernel)** — `r3_verified = 0` per governance definition. Prior "R3" labels were loose.
3. **"Phase 2A complete"** — re-opened by independent audit. Not complete.
4. **"SOC 2 / ISO 27001 certified"** — no certification is held. Retracted-claim guard enforces this.
5. **"Clean working tree"** — HEAD has 92 dirty entries (56 modified, 36 untracked). The dirty tree is a **divergent fork**, not a superset of HEAD or the candidate.

## DIRTY-TREE COLLISIONS

### Topology

```
origin/main (1338d0b) ── common ancestor
    ├── HEAD (639dbbb) — 2 unique commits (governance preflight docs)
    │       └── dirty working tree: 92 entries (56 M, 36 ??)
    └── Candidate (34d29a9) — 25 unique commits (Gate-0 remediation chain)
            └── clean worktree: 0 dirty, 258 tracked files
```

**HEAD and candidate DIVERGE.** Neither is a descendant of the other. The candidate is NOT a superset of HEAD. HEAD is NOT a superset of the candidate.

### Collision map

**32 files** are modified in BOTH the dirty working tree AND the candidate (changed between HEAD and `34d29a9`).

Of those 32:
- **31 files** differ in content between the dirty tree and the candidate (divergent edits)
- **1 file** matches: `crates/zylcode-core/tests/agent_loop_e2e.rs` (both trees have the same version)

### Full collision file list

```
PARALLEL_EXECUTION_MANIFEST.md
crates/zylcode-core/benches/router_cache.rs
crates/zylcode-core/src/agent_protocol.rs
crates/zylcode-core/src/ai_input/context_manager.rs
crates/zylcode-core/src/ai_input/file_processor.rs
crates/zylcode-core/src/ai_input/intent_engine.rs
crates/zylcode-core/src/ai_input/text_processor.rs
crates/zylcode-core/src/ai_input/voice_processor.rs
crates/zylcode-core/src/computer_use/gui_automation.rs
crates/zylcode-core/src/computer_use/input_controller.rs
crates/zylcode-core/src/computer_use/screen_capture.rs
crates/zylcode-core/src/computer_use/vision_ai.rs
crates/zylcode-core/src/computer_use/workflow_engine.rs
crates/zylcode-core/src/context_builder.rs
crates/zylcode-core/src/pipeline.rs
crates/zylcode-core/tests/agent_loop_e2e.rs          ← ONLY MATCHING FILE
crates/zylcode-core/tests/decision_proptest.rs
crates/zylcode-core/tests/pipeline_proptest.rs
crates/zylcode-mcp/src/enhanced_bridge.rs
crates/zylcode-mcp/src/enhanced_plugin_marketplace.rs
crates/zylcode-mcp/src/enhanced_skills.rs
crates/zylcode-mcp/src/executor.rs
crates/zylcode-mcp/src/hot_reload.rs
crates/zylcode-mcp/src/lib.rs
crates/zylcode-mcp/src/plugin_marketplace.rs
crates/zylcode-mcp/src/real_tools.rs
crates/zylcode-mcp/src/registry.rs
crates/zylcode-mcp/src/skills_system.rs
crates/zylcode-mcp/src/tool.rs
crates/zylcode-mcp/tests/fault_injection.rs
crates/zylcode-mcp/tests/integration_test.rs
crates/zylcode-mcp/tests/telemetry_audit_tests.rs
```

### Dirty tree summary (HEAD working tree)

| Category | Count |
|---|---|
| Modified tracked files | 56 |
| Untracked entries | 36 |
| **Total dirty entries** | **92** |

### Candidate summary

| Category | Count |
|---|---|
| Tracked files | 258 |
| Dirty entries | 0 |
| Untracked entries | 0 |

## LOCAL VERIFICATION

### Reproduction block

```bash
# Environment
# OS: Windows 11, Git Bash (POSIX sh)
# rustc 1.97.1, cargo 1.97.1
# Working directory: C:\Projects\zylcode

# 1. Pin SHAs
git rev-parse HEAD
# → 639dbbb397d9052d9aa0376b3ef6a734f176c52c

git rev-parse origin/main
# → 1338d0bee3e676a2f5d8c678ba32f8e6ad41237b

git rev-list --all | grep "^34d29a9"
# → 34d29a90bfe3f6e404ac3bbe8131958b33475a1b

# 2. Topology
git merge-base HEAD 34d29a9
# → 1338d0bee3e676a2f5d8c678ba32f8e6ad41237b (= origin/main)

git log --oneline HEAD --not origin/main
# → 639dbbb docs(governance): preflight re-verification ...
# → 29a9654 docs(governance): parallel preflight ...

git log --oneline 34d29a9 --not origin/main | wc -l
# → 25

# 3. Dirty tree
git status --porcelain | wc -l
# → 92

git status --porcelain | awk '{print $1}' | sort | uniq -c
# → 56 M
# → 36 ??

# 4. Collision map
git diff --name-only HEAD | sort > /tmp/head_dirty.txt
git diff --name-only HEAD..34d29a9 | sort > /tmp/candidate_diff.txt
comm -12 /tmp/head_dirty.txt /tmp/candidate_diff.txt | wc -l
# → 32

git diff --stat 34d29a9 -- $(cat /tmp/collisions.txt | tr '\n' ' ') | tail -1
# → 31 files changed (1 matches: agent_loop_e2e.rs)

# 5. Candidate cleanliness
git -C .wt/remediation status --porcelain | wc -l
# → 0

git -C .wt/remediation ls-files | wc -l
# → 258

# 6. Candidate build/test (run in .wt/remediation)
cargo check --workspace --lib --offline     # PASS, 1m53s
cargo test -p zylcode-mcp --lib --offline    # PASS, 100 passed, 0 failed
cargo test -p zylcode-core --lib --offline  # PASS, 229 passed, 0 failed
cargo clippy --workspace --all-targets -D warnings --offline  # PASS, 5m08s, 0 warnings

# 7. Retracted-claim guard
python scripts/check_retracted_claims.py           # EXIT 1 (HEAD, 135 new from .wt/)
cd .wt/remediation && python scripts/check_retracted_claims.py  # EXIT 0 (11 baselined)

# 8. Fabricated assertions
grep -n "count > 100" crates/zylcode-mcp/src/enhanced_bridge.rs     # HEAD: line 1495 (live)
grep -n "count > 100" .wt/remediation/crates/zylcode-mcp/src/enhanced_bridge.rs  # Candidate: line 820 (comment only)
grep -n "count >= 150" crates/zylcode-mcp/src/hot_reload.rs        # HEAD: line 739 (live)
grep -n "count >= 150" .wt/remediation/crates/zylcode-mcp/src/hot_reload.rs     # Candidate: line 594 (comment only)

# 9. mcp.tools.yaml tracking
git ls-files mcp.tools.yaml                    # HEAD: (empty = untracked)
git -C .wt/remediation ls-files mcp.tools.yaml  # Candidate: mcp.tools.yaml (tracked)

# 10. Tool catalogue modules
wc -l .wt/remediation/crates/zylcode-mcp/src/tool_catalogue.rs  # 881
wc -l .wt/remediation/crates/zylcode-mcp/src/permission.rs      # 371
wc -l .wt/remediation/crates/zylcode-mcp/src/evidence.rs        # 366
wc -l .wt/remediation/crates/zylcode-mcp/src/actor.rs           # 106
# None of these exist in HEAD: crates/zylcode-mcp/src/
```

## REMOTE PUSH

**Not pushed.** No push was attempted. This is reconnaissance only.

## CI

**BLOCKED_EXTERNAL.** `gh` CLI is not authenticated in this environment. No CI runs were triggered or inspected. Remote CI state was last verified via authenticated GitHub API on 2026-09-17: `origin/main` (`1338d0b`) is red at CI step 8 (missing sixth `ledger` arg in `agent_loop_e2e.rs`). No PR exists for the candidate.

## OPEN BLOCKERS

1. **Divergent fork.** HEAD and candidate diverge from `1338d0b`. 32 collision files, 31 with divergent content. **Cannot fast-forward or merge without reconciliation.** Each collision file must be individually adjudicated: take the candidate version, take the dirty-tree version, or take neither.

2. **Dirty tree carries foreign edits.** The 92 dirty entries include pre-existing uncommitted modifications from another session (the implementation agent's work). These are NOT this auditor's changes. Any `git add` on this tree risks committing someone else's work. `git diff --cached --stat` must be checked before any staging.

3. **Fabricated assertions still live on HEAD.** `enhanced_bridge.rs:1495` and `hot_reload.rs:739` block `cargo test --workspace --lib` on HEAD. The candidate fixed these, but the fix is on a divergent branch.

4. **Retracted-claim guard fails on HEAD.** The guard scans the filesystem (including `.wt/` worktrees) rather than only tracked files. The candidate fixed this (`d1c0a15`), but HEAD's guard is still broken. Running the guard from HEAD's working tree produces 135 false positives.

5. **`mcp.tools.yaml` untracked on HEAD.** The product's actual tool surface is untracked and unignored on HEAD. Without it, the shipped app registers zero tools. The candidate tracks it, but on a divergent branch.

6. **Computer-Use Engine remains R0.** Both HEAD and candidate carry the simulation facade (1,763 lines, fabricated confidences, `vec![0]` captures). Governance says "replace, do not extend." No replacement exists.

7. **R3 verified = 0.** No capability has been verified through a product surface with captured evidence. The capability registry claims 20 GREEN, but GREEN requires R3 per governance. These 20 GREEN claims are unverified.

8. **Legacy root .md quarantine not done.** 47-64 root .md files (depending on tree) include legacy completion docs with retracted claims ("156 tools", "SOC 2", "ISO 27001"). The quarantine called for in `docs/README_INDEX.md` has not been executed in either tree.

9. **Candidate `.gitignore` differs from HEAD.** Candidate adds `/.zylcode/` and `**/.zylcode/` entries; HEAD has `.wt/` entry. This must be reconciled during any merge.

## NEXT RECOMMENDED UNIT

**RECOVERY-02 — Repository Hygiene Reconciliation.**

Before any code changes, the divergent fork must be reconciled:

1. **Triage the 92 dirty entries.** Categorize each as: (a) foreign edits to preserve, (b) candidate's changes to take, (c) stale work to discard. The implementation agent's work must be preserved or explicitly rejected, not silently overwritten.

2. **Adjudicate the 31 divergent collision files.** For each, decide: take the candidate version, take the dirty-tree version, or take neither. This is a file-by-file decision, not a bulk operation.

3. **Rebase the candidate onto HEAD (or HEAD onto candidate).** After collision adjudication, produce a linear history that incorporates both the candidate's remediation and HEAD's governance preflight docs.

4. **Fix the retracted-claim guard on HEAD.** Apply the candidate's `d1c0a15` fix (scan tracked files only) to HEAD.

5. **Stage and commit the reconciled tree.** Verify `cargo test --workspace --lib` passes and the retracted-claim guard passes before committing.

**OR** if the user prefers:

**RECOVERY-02A — Candidate Acceptance Path.**

1. Accept the candidate's remediation chain (25 commits) as the new base.
2. Cherry-pick HEAD's 2 governance preflight commits on top.
3. Handle the dirty tree as a separate workstream (triage and reapply selectively).

This path is simpler but discards the dirty tree's divergent edits. The user must decide which path to take.
