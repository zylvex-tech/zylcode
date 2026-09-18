# REPOSITORY INTEGRITY RECOVERY — FIX-1 through FIX-4

**Order:** `AUTHORIZE REPOSITORY INTEGRITY RECOVERY — FIX-1/FIX-2` (owner, 2026-09-17)
**Priority:** P0
**Executor:** architect/auditor session
**Status:** **PATCH COMPLETE AND VERIFIED — AWAITING OWNER ACCEPTANCE. NOT PUSHED.**
**Scope:** repository-integrity recovery only. No Phase 2A status change. No Phase 2B. No new capability.

---

## 0. Summary

A clean checkout of `origin/main` (`1338d0b`) could not compile. Investigation found **four**
independent integrity defects, not the two the order anticipated. Each is the same failure mode:
**a commit that landed the caller but withheld the callee.**

| ID | Defect | Root commit | Repair applied |
|---|---|---|---|
| FIX-1 | `pub mod performance;` / `pub mod system_integration;` declared, files never in git | `5f5a99e` | Removed 2 declarations + 2 re-exports |
| FIX-2 | 8 × `get_more_*_tools()` called, never defined | `29cc936` | Removed 8 call sites |
| **FIX-3** | `zylcode-core` imports `RiskLevel`/`ToolSchema`, calls `list_schemas()` — absent from `zylcode-mcp` | `158370f` | Added 2 types (22 lines) + 1 method (31 lines) |
| **FIX-4** | `agent_loop_e2e.rs` calls `AgentLoop::new` with 5 args; fn takes 6 | (same lineage) | Added the missing `ledger` argument |

**FIX-3 and FIX-4 were not in the order.** They exist, they block the stated acceptance criterion
(*"a clean-checkout buildable repository"*), and the order authorizes repairing *"performance module
… system_integration module … eight `get_more_*_tools()` methods"* — it does not forbid repairing
what a clean-checkout build additionally requires. They are recorded here separately so the owner
can accept or reject them on their own merits.

**Total patch: 5 files, +79 / −25 lines.** No new capability. No simulated execution added.

---

## 1. Provenance audit (mandated step 2)

Both missing modules were traced through full history:

```
$ git log --all --oneline --diff-filter=A -- crates/zylcode-mcp/src/performance.rs
(empty)
$ git log --all --oneline --diff-filter=A -- crates/zylcode-mcp/src/system_integration.rs
(empty)
```

**Neither file has ever existed in any commit on any ref.** This eliminates repair option **B**
(restore an earlier known-good implementation) — there is no earlier implementation to restore.

The defect was introduced by two commits, both partial:

```
$ git show 5f5a99e -- crates/zylcode-mcp/src/lib.rs | grep "^+pub mod"
+pub mod performance;
+pub mod real_tools;
+pub mod system_integration;

$ git show --name-only --format="" 5f5a99e | grep -E "performance|system_integration"
(empty — declared but the files were never added)

$ git show 29cc936 -- crates/zylcode-mcp/src/enhanced_bridge.rs | grep -c "^+.*self.get_more_"
8
```

- `5f5a99e` — *"feat(runtime): replace mocked tools with real execution"* (2026-09-15). Declared
  three modules; added only `real_tools.rs`. `performance` and `system_integration` were declared
  but withheld.
- `29cc936` — *"Add AI input, computer-use, and plugin marketplace modules"* (2026-09-14). Added all
  eight `self.get_more_*_tools()` **call sites**; the implementations stayed uncommitted.

`158370f` — *"feat(agent): complete Phase 1C commissioning"* — added `use zylcode_mcp::real_tools::RiskLevel;`
and `use zylcode_mcp::real_tools::{ToolSchema, RiskLevel};` to `zylcode-core`, against
`real_tools` symbols that were not committed until FIX-3.

---

## 2. Content audit (mandated step 3)

| Check | `performance.rs` | `system_integration.rs` | `get_more_*` block |
|---|---|---|---|
| Mocks | none | none | none |
| Fabricated success | none | none | **no** (definitions only) |
| Simulated execution | none | none | routed to a simulator — see §3 |
| Dead code | **yes — never referenced** | **yes — never referenced** | **yes** |
| Unsupported capability claims | none | none | **yes — see §3** |
| Unrelated refactors | none | none | none |
| Secrets | **none** | **none** | **none** |

Secret scan (all three sources): `api[_-]?key|secret|token|password|bearer|private[_-]?key|ghp_|sk-` → **0 hits.**

`performance.rs` (409 lines) implements a TTL cache, an execution pool, and a memory pool with
honest instrumentation (`total_created` / `total_reused` / `reuse_ratio`). Its three tests assert
real behaviour. It is competent, non-fabricated, and **completely unused**.

`system_integration.rs` (391 lines) is **structurally consistent** — it depends only on committed
modules (`enhanced_plugin_marketplace`, `enhanced_skills`, `hot_reload`), uses the committed
`ToolDefinition` shape verbatim (6 fields, exact match), and `uuid` with the `v4` feature is declared
in `Cargo.toml`. Its only weakness is an honest in-code admission at line 76:
*"Note: This would require adding a method to register tools dynamically / For now, we'll just cache
the tool definition"* — i.e. `register_skills_as_tools` does **not** register anything. It caches.

---

## 3. ⚠ The finding that decided FIX-2

Committing the 113 uncommitted tool definitions was considered and **rejected on evidence.**

Every tool the bridge registers is wrapped in `BuiltinTool`, whose `call()` is:

```rust
async fn call(&self, params: Value) -> Result<Value> {
    // Simulate tool execution
    // In real implementation, this would execute the actual tool
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    Ok(serde_json::json!({
        "result": { "success": true,
                    "message": format!("Tool {} executed successfully", self.definition.id) }
    }))
}
```

**This is committed at `HEAD`.** So committing the 113 definitions would have:

1. Added `113 × 10 ms` of sleep to `initialize_with_builtin_tools()`;
2. Taken the reported tool count from **28 → 141**;
3. Registered 113 tools that return `success: true` **without doing anything**.

That is fabricated execution — the exact anti-pattern the constitution forbids. **The uncommitted
definitions are not the lost half of a real feature; they are the missing half of a simulated one.**

Two further facts support repair option **C**:

- **The bridge is not reachable from the product.** `EnhancedMcpBridge` is referenced only inside
  `zylcode-mcp`. The desktop app uses `ZylCodeEngine` and the real `execute_with_recovery` path.
- **The 113 definitions are not load-bearing.** They are referenced by no re-export and no caller;
  nothing outside the bridge consumes them.

**Decision: option C — remove the eight call sites.** (Option D, "rewrite the minimum implementation
required by the real contract", has no real contract to satisfy: this surface is unreachable.)

---

## 4. ⚠ A contradiction that this patch exposes but does not resolve

`crates/zylcode-mcp/tests/integration_test.rs` is **committed** and **CI runs it**
(`cargo test --workspace --all-targets`). It asserts:

| Assertion | Committed implementation | Result |
|---|---|---|
| `assert!(tool_count >= 100)` | **28** | **FAILS** |
| `assert!(dev_tools.len() >= 25)` | **11** | **FAILS** |
| `assert!(categories.len() >= 8)` | **8** | passes |

Measured committed category sizes: `development` 11, `ai_ml` 3, `database` 3, `cloud` 2, `devops` 2,
`communication` 2, `productivity` 2, `security` 2 = **28**.

**These two assertions cannot pass on any clean checkout** — the only thing that could have satisfied
them is the `get_more_*` block, which was never in git. So either:

- **(i)** CI is red on `main` and this has not been noticed; or
- **(ii)** CI has never run (unobservable from this environment — `gh` returns 401).

**This is not repaired here.** Suppressing the assertions (changing the test to match the code) would
be a builder certifying its own work and would erase the evidence. The correct repair — a real tool
catalogue, or an honest re-scoping of the claim — is a **product decision**, recorded as an open item.

> **The `100+ tools` claim is not backed by the committed implementation.** It was retracted once
> already (Finding F) and has been re-entering. This is its third documented form.

---

## 5. The patch

```diff
 crates/zylcode-core/tests/agent_loop_e2e.rs | 39 +++++++++++++++++++----------
 crates/zylcode-mcp/src/enhanced_bridge.rs   |  8 ------
 crates/zylcode-mcp/src/lib.rs               |  4 ---
 crates/zylcode-mcp/src/real_tools.rs        | 22 ++++++++++++++++
 crates/zylcode-mcp/src/registry.rs          | 31 +++++++++++++++++++++++
 5 files changed, 79 insertions(+), 25 deletions(-)
```

- **`lib.rs`** — removed `pub mod performance;`, `pub mod system_integration;` and their two `pub use`
  re-export lines. (Note: the dirty tree's `lib.rs` is *rustfmt only*; the declaration lines are
  clean at `HEAD`.)
- **`enhanced_bridge.rs`** — removed the eight `self.get_more_*_tools(),` call sites.
- **`real_tools.rs`** — added `RiskLevel` (7 variants) and `ToolSchema` (5 fields), taken verbatim
  from the dirty tree. `ToolEvidence` and the other `real_tools` symbols were already committed.
- **`registry.rs`** — added `ToolRegistry::list_schemas()`, a 31-line additive method.
- **`agent_loop_e2e.rs`** — added `MemoryLedgerStore` and the sixth `ledger` argument at three call
  sites. `memory_ledger.rs` is committed at `HEAD`; only the test had not caught up.

**The 338-line `real_tools.rs` refactor in the dirty tree was NOT committed.** Only the 22 lines
`zylcode-core` structurally requires were extracted. Everything else in that file — and the other
51 modified paths — remains untouched.

---

## 6. Verification (clean worktree, detached from `origin/main` = `1338d0b`)

The dirty main working tree was **not** used for any verification, per the order.

**Committed:** `33f126ec553da3d77f781779440d7c7fe6905356`**"fix(repo): restore self-contained clean-checkout build"**
**Tree:** `21cd39f9cbfe18ce7b36118e72e4d823bdcd96a2`

### 6.0 Post-commit re-verification (mandated: fresh worktree from the repaired commit)

```
$ git worktree add .wt/verify 33f126e --detach
HEAD is now at 33f126e fix(repo): restore self-contained clean-checkout build
$ git status --porcelain
(empty — pristine, no dirty carryover)
```

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets` | **EXIT 0 — 0 errors** |
| `git diff --check` | **EXIT 0** |
| `git status --porcelain` | **empty** — no untracked required source |

This was run a second time, from scratch, in a directory created *after* the commit — so it could
not have inherited any artifact from the patch-under-development worktree.

### 6.1 Pre-commit verification (`.wt/fix1`)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets` | **EXIT 0 — 0 errors** |
| `git diff --check` | **EXIT 0** (no whitespace errors, no conflict markers) |
| `git status --porcelain \| grep '^??'` | **empty** — no required source exists only as untracked content |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 1 — 31 errors, all pre-existing warnings** |
| `cargo test --workspace --lib` | **EXIT 101 — 215 passed, 11 failed** |

### 6.1 Clippy — reported honestly as NOT GREEN

`-D warnings` fails on **31 pre-existing lint warnings** in `enhanced_plugin_marketplace.rs`,
`enhanced_skills.rs`, `real_tools.rs`, `hot_reload.rs` and others — dead struct fields, unused
imports, `unused mut`. **None originate in this patch.** Fixing them would mean a 338-line refactor
of the very file the order told me not to commit blindly. **Reported as FAILING, not waived.**

### 6.2 The 11 test failures — all pre-existing, none caused by this patch

`10 × cli::tests::run_benchmark_*` — `failed to execute benchmark binary: Os { code: 2, NotFound }`.
These tests shell out to `target/debug/zylcode-core-cli`, which `cargo test --lib` does not build.
They cannot pass under the mandated command regardless of this patch.

`1 × router::tests::synthetic_offline_dispatch_returns_parseable_payload` —
`called Result::unwrap() on an Err value: all providers failed — provider openrouter returned 401
Unauthorized … fallback (ollama): HTTP request failed`. This test's own comment claims
*"Hermetic: no persistent vector cache."* **It nonetheless makes a live network call and fails on a
missing credential.** The claim of hermeticity is falsified by its own failure. `router.rs` is not
in this patch.

**Because a clean `origin/main` does not compile, no test suite could be run against the unpatched
baseline for comparison.** The isolation argument is therefore structural, not empirical: the patch
touches 5 files, none of which are `cli.rs` or `router.rs`. This limitation is stated rather than
papered over.

---

## 7. What this patch does NOT do

- It does **not** make `cargo clippy -D warnings` green (§6.1).
- It does **not** make `cargo test --workspace` green (§6.2).
- It does **not** resolve the `100+ tools` assertion contradiction (§4) — **owner decision**.
- It does **not** commit `performance.rs`, `system_integration.rs`, or the 113 definitions.
- It does **not** touch the other 51 modified paths.
- It does **not** change Phase 2A status, claim Repository Intelligence repaired, begin Phase 2B, or
  implement any capability.
- It does **not** reach the remote (§8).

---

## 8. Remote status

**PUSHED: NO. REMOTE SHA VERIFIED: NO.**

`origin/main` remains `1338d0bee3e676a2f5d8c678ba32f8e6ad41237b` — i.e. **still broken.**

```
$ GIT_ASKPASS=echo git -c credential.helper= -c core.askPass= push origin main
remote: Invalid username or token. Password authentication is not supported for Git operations.
```

The local Git credential is expired. Per the owner's instruction, no PAT was requested, transmitted,
or written into any file. **The repair is COMMITTED locally and STOPPED at COMMITTED.**

**The repository is not remotely repaired until the remote SHA contains this patch.**

---

## 9. State at time of writing

| State | Value |
|---|---|
| IMPLEMENTED | **YES** — 5-file patch |
| CLEAN CHECKOUT BUILDS | **YES** — `cargo check --workspace --all-targets` exit 0 |
| TESTED | **PARTIAL** — 215 pass / 11 fail, all pre-existing; isolation structural (§6.2) |
| COMMITTED | **PENDING OWNER REVIEW** |
| PUSHED | **NO** |
| REMOTE SHA VERIFIED | **NO** — remote still `1338d0b` |
| CI VERIFIED | **NOT OBSERVABLE** — `gh` 401 |
