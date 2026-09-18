# GATE-0 REMEDIATION FORENSIC

**Order:** Owner review result, 2026-09-17
**Status:** FORENSIC COMPLETE — no fixes implemented; report presented for owner review
**Scope:** A (clippy), B (CLI benchmark tests), C (synthetic offline router test), D (tool catalogue truth table)

---

## A. CLIPPY — Root-cause matrix

### A.1 Summary

`cargo clippy --workspace --all-targets -- -D warnings` produces **31 errors** in `zylcode-mcp`.
**None are introduced by `f1b2b36`.** All are pre-existing at `1338d0b`.

### A.2 Classification

| # | Error (condensed) | File | Lint | At `1338d0b`? | Touched by `f1b2b36`? | Min safe correction | Runtime impact |
|---|---|---|---|---|---|---|---|
| 1 | unused imports `ExecutionContext`, `ExecutionRecord` | `lib.rs` | `unused_imports` | **yes** | **yes** (re-export removal) | Remove 2 `pub use` lines | None |
| 2 | unused import `ToolCategory` | `hot_reload.rs:10` | `unused_imports` | **yes** | no | Remove import | None |
| 3 | unused imports `error`, `info`, `warn` | `real_tools.rs:11` | `unused_imports` | **yes** | **yes** (FIX-3 added 2 types) | Remove 3 imports | None |
| 4 | variable does not need to be mutable | `real_tools.rs:16` | `unused_mut` | **yes** | **yes** (FIX-3) | Remove `mut` | None |
| 5 | unused variable `user_id` | `real_tools.rs` | `unused_variables` | **yes** | **yes** (FIX-3) | Prefix with `_` | None |
| 6 | fields `payment_gateway`, `payout_manager` never read | `enhanced_plugin_marketplace.rs:19` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 7 | fields `provider`, `api_key`, `sandbox_mode` never read | `enhanced_plugin_marketplace.rs:28` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 8 | field `plans` never read | `enhanced_plugin_marketplace.rs:36` | `dead_code` | **yes** | no | Remove field OR prefix with `_` | None |
| 9 | field `revenue_by_period` never read | `enhanced_plugin_marketplace.rs:83` | `dead_code` | **yes** | no | Remove field OR prefix with `_` | None |
| 10 | fields `payouts`, `payout_schedule` never read | `enhanced_skills.rs:20` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 11 | fields `user_preferences`, `plugin_similarities`, `collaborative_filtering` never read | `enhanced_bridge.rs:45` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 12 | fields `user_item_matrix`, `item_similarity` never read | `skills_system.rs:91` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 13 | fields `rules`, `templates` never read | `enhanced_skills.rs:20` | `dead_code` | **yes** | no | Remove fields OR prefix with `_` | None |
| 14 | field `payouts` never read | `enhanced_skills.rs:117` | `dead_code` | **yes** | no | Remove field OR prefix with `_` | None |
| 15–31 | missing `Default` impl for 17 structs | multiple | `new_without_default` | **yes** | no | Add `#[derive(Default)]` or manual impl | None |

### A.3 Analysis

**Errors 1–5** touch files modified by `f1b2b36`, but the warnings themselves **pre-exist**:
- `lib.rs` error #1 is the **old** `pub use` lines for `performance`/`system_integration` re-exports — already present at `1338d0b`.
- `real_tools.rs` errors #3–5 are in the **existing** code; FIX-3 only added `RiskLevel` + `ToolSchema` at the top of the file, which do not use `error`/`info`/`warn` or a mutable variable.

**Errors 6–14** are **dead struct fields** across `enhanced_plugin_marketplace.rs`, `enhanced_skills.rs`, `enhanced_bridge.rs`, and `skills_system.rs`. These are structural stubs — fields declared but never referenced. They are the residue of a partially-implemented marketplace/skills system.

**Errors 15–31** are **missing `Default` implementations** for 17 structs. These are lint suggestions, not failures. Adding `#[derive(Default)]` or `impl Default` satisfies clippy without changing behaviour.

### A.4 Proposed remediation (not implemented)

| Commit | Files | Description |
|---|---|---|
| `clippy-fix-a` | `lib.rs`, `hot_reload.rs`, `real_tools.rs` | Remove unused imports + `mut` + prefix unused var |
| `clippy-fix-b` | `enhanced_plugin_marketplace.rs`, `enhanced_skills.rs`, `enhanced_bridge.rs`, `skills_system.rs` | Prefix dead fields with `_` or remove them |
| `clippy-fix-c` | 17 struct files | Add `#[derive(Default)]` where semantically valid |

**Dependency:** `clippy-fix-a` can be independent; `clippy-fix-b` and `clippy-fix-c` are independent of each other.

---

## B. CLI BENCHMARK TESTS — Root cause

### B.1 Symptom

10 tests in `cli::tests` fail with:
```
failed to execute benchmark binary: Os { code: 2, kind: NotFound, ... }
```

### B.2 Root cause

The test helper `run_benchmark_binary()` shells out to:
```rust
let exe = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("../../target/debug/zylcode-core-cli");
```

**`cargo test --workspace --lib` does NOT build binary targets.** It builds library targets and their unit tests only. The `zylcode-core-cli` binary (defined in `Cargo.toml` as `[[bin]]` with `path = "src/bin/zylcode-core-cli/main.rs"`) is not produced.

The binary **does exist** in the main working tree (`target/debug/zylcode-core-cli`, built earlier), but **not in a clean worktree** — confirming the test reads a developer-local artifact.

### B.3 The intended Cargo test contract

The CI workflow runs:
```yaml
run: cargo test --workspace --all-targets
```

`--all-targets` includes binaries, but `--lib` (used in the mandated verification command) does not. The test was written assuming `--all-targets`, while the verification command used `--lib`.

However, even with `--all-targets`, the binary may not be built **before** the test runs if cargo's job scheduling places the test before the binary build completes.

### B.4 Proposed remediation (not implemented)

| Approach | Files | Description |
|---|---|---|
| **Preferred** | `cli.rs` test module | Use `std::env::current_exe()` to find the test binary itself, or compile the CLI inline via `cargo build --bin` in a `#[test]` setup, or convert the integration tests to use the library API directly instead of subprocess |
| Alternative | `Cargo.toml` | Add a `[[test]]` target that depends on the binary being built first |
| Alternative | CI | Split into `cargo build --bin` then `cargo test --workspace --all-targets` |

**The test architecture is wrong, not the build command.** A hermetic test must not depend on a binary that the test runner does not guarantee to build.

---

## C. SYNTHETIC OFFLINE ROUTER TEST — Root cause

### C.1 Symptom

`router::tests::synthetic_offline_dispatch_returns_parseable_payload` fails with:
```
all providers failed — primary: provider openrouter returned 401 Unauthorized
```

The test's own comment claims *"Hermetic: no persistent vector cache"*.

### C.2 Root cause

`RouterConfig::default()` sets:
- `primary_provider = ModelProvider::OpenRouter` (requires API key)
- `fallback_provider = ModelProvider::LocalOllama` (no key required)

`dispatch_prompt()` has a synthetic fast-path:
```rust
let primary_key = self.config.api_key(&self.config.primary_provider);
let fallback_key = self.config.api_key(&self.config.fallback_provider);
let offline_fast = primary_key.is_none() && fallback_key.is_none();
let both_need_keys = self.config.primary_provider != ModelProvider::LocalOllama
    && self.config.fallback_provider != ModelProvider::LocalOllama;
if offline_fast && both_need_keys {
    return Ok(Self::synthetic_response(prompt, system));
}
```

**The leak:** `api_key()` falls back to `std::env::var("OPENROUTER_API_KEY")`:
```rust
std::env::var(env_key).ok().filter(|v| !v.is_empty())
```

In this environment, `OPENROUTER_API_KEY=YOUR_OPENROUTER_API_KEY` is set — a **placeholder value** that `api_key()` treats as a real key. This makes `primary_key = Some("YOUR_OPENROUTER_API_KEY")`, which makes `offline_fast = false`, which **bypasses the synthetic path entirely**. The test then attempts a live HTTP call to OpenRouter with the placeholder key, receiving 401.

### C.3 Falsification test (proving non-hermeticity)

**Hypothesis:** The test is hermetic (no external dependencies).
**Falsification:** Set `OPENROUTER_API_KEY` to any non-empty string and run the test.
**Result:** Test fails with a live network error (401) instead of taking the synthetic path.
**Conclusion:** The test is **not hermetic** — its behaviour depends on an environment variable.

### C.4 Proposed remediation (not implemented)

| Approach | Files | Description |
|---|---|---|
| **Preferred** | `router.rs` test module | Construct `RouterConfig` with `api_keys: HashMap::new()` explicitly (not `Default`), or temporarily clear `OPENROUTER_API_KEY` in test setup |
| Alternative | `router.rs` | Add `ModelProvider::SyntheticOffline` as a first-class provider that always returns the synthetic response, and use it in the test |
| Alternative | `router.rs` | Change `api_key()` to accept an optional env-var override parameter, defaulting to `std::env::var` in production but injectable in tests |

**The test must guarantee the synthetic path regardless of the developer's environment.**

---

## D. TOOL CATALOGUE TRUTH TABLE

### D.1 Forensic findings

**The `>= 100` and `>= 25` assertions were introduced by `29cc936` alongside the 8 `get_more_*_tools()` call sites.** The assertions were calibrated to the **uncommitted** `get_more_*` implementations (85 additional tools, total 113). They have **never** been satisfiable on a clean checkout.

**The bridge is not reachable from the product.** `EnhancedMcpBridge` is referenced only inside `zylcode-mcp` itself. The desktop app uses `ZylCodeEngine` + `execute_with_recovery` + `DynamicTool` configured from `mcp.tools.yaml`.

### D.2 Truth table (all 27 committed bridge tools)

| tool_id | category | definition | executor | real_exec | product_reach | test_cov | status |
|---|---|---|---|---|---|---|---|
| git.commit | development | yes | yes | yes | no | yes | **REAL** |
| git.push | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| git.pull | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| npm.install | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| yarn.install | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| webpack.build | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| vite.build | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| jest.test | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| vitest.test | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| eslint.lint | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| prettier.format | development | yes | no | no | no | yes | **DEFINITION_ONLY** |
| openai.complete | ai_ml | yes | no | no | no | yes | **DEFINITION_ONLY** |
| deepseek.complete | ai_ml | yes | no | no | no | yes | **DEFINITION_ONLY** |
| pinecone.query | ai_ml | yes | no | no | no | yes | **DEFINITION_ONLY** |
| postgres.query | database | yes | no | no | no | yes | **DEFINITION_ONLY** |
| mysql.query | database | yes | no | no | no | yes | **DEFINITION_ONLY** |
| mongodb.find | database | yes | no | no | no | yes | **DEFINITION_ONLY** |
| aws.s3.upload | cloud | yes | no | no | no | yes | **DEFINITION_ONLY** |
| vercel.deploy | cloud | yes | no | no | no | yes | **DEFINITION_ONLY** |
| docker.build | devops | yes | no | no | no | yes | **DEFINITION_ONLY** |
| kubernetes.deploy | devops | yes | no | no | no | yes | **DEFINITION_ONLY** |
| slack.send | communication | yes | no | no | no | yes | **DEFINITION_ONLY** |
| email.send | communication | yes | no | no | no | yes | **DEFINITION_ONLY** |
| notion.create_page | productivity | yes | no | no | no | yes | **DEFINITION_ONLY** |
| jira.create_issue | productivity | yes | no | no | no | yes | **DEFINITION_ONLY** |
| snyk.scan | security | yes | no | no | no | yes | **DEFINITION_ONLY** |
| vault.read_secret | security | yes | no | no | no | yes | **DEFINITION_ONLY** |

**Summary: REAL = 1, DEFINITION_ONLY = 26**

### D.3 Additional real tools (not in the bridge)

`real_tools.rs` has a factory `get_real_tool()` with **real executors** for these IDs:

| tool_id | executor | real_exec | in bridge? |
|---|---|---|---|
| fs.read | FileSystemTool | yes | **no** |
| fs.write | FileSystemTool | yes | **no** |
| fs.list | FileSystemTool | yes | **no** |
| shell.execute | ShellTool | yes | **no** |
| npm.run | ShellTool | yes | **no** |
| cargo.test | ShellTool | yes | **no** |
| git.status | GitTool | yes | **no** |
| git.diff | GitTool | yes | **no** |
| search.find | SearchTool | yes | **no** |
| search.grep | SearchTool | yes | **no** |

**Note:** `git.push` and `git.pull` are in the bridge but **not** in the factory (only `git.commit`, `git.status`, `git.diff` are). `npm.run` is in the factory but `npm.install` is in the bridge — a **mismatch**.

### D.4 The assertion contradiction

| Assertion | Required | Actual (committed) | Satisfiable? |
|---|---|---|---|
| `tool_count >= 100` | 100 | 27 | **NO** |
| `dev_tools.len() >= 25` | 25 | 11 | **NO** |
| `categories.len() >= 8` | 8 | 8 | yes |

The assertions represent a **fabricated historical claim** — they were written against code that was never committed. They are not a valid unfinished requirement because the surface they test (`EnhancedMcpBridge`) is unreachable from the product.

---

## E. PROPOSED MINIMAL REMEDIATION COMMITS

| Commit | Files | Description | Tests |
|---|---|---|---|
| `clippy-fix-a` | `lib.rs`, `hot_reload.rs`, `real_tools.rs` | Remove unused imports, `mut`, prefix unused var | `cargo clippy -D warnings` green for these files |
| `clippy-fix-b` | `enhanced_plugin_marketplace.rs`, `enhanced_skills.rs`, `enhanced_bridge.rs`, `skills_system.rs` | Prefix dead fields with `_` or remove | `cargo clippy -D warnings` green for these files |
| `clippy-fix-c` | 17 struct definitions | Add `#[derive(Default)]` | `cargo clippy -D warnings` green |
| `cli-test-hermetic` | `cli.rs` test module | Replace subprocess with library API call or build binary in test setup | `cargo test --workspace --lib` passes cli tests |
| `router-test-hermetic` | `router.rs` test module | Clear `OPENROUTER_API_KEY` env or use explicit empty config | `cargo test --workspace --lib` passes router test |
| `tool-assertion-honest` | `integration_test.rs` | **Owner decision required** — either re-scope assertions to match committed reality (27 tools, 11 dev) or remove the test if the surface is deprecated | `cargo test --workspace --lib` passes integration_test |

### Dependency/order

```
clippy-fix-a, clippy-fix-b, clippy-fix-c  →  independent (any order)
cli-test-hermetic, router-test-hermetic     →  independent (any order)
tool-assertion-honest                       →  depends on owner product decision
```

All six can be applied on top of `f1b2b36` in any order except `tool-assertion-honest`, which requires the owner's decision on whether to:
- (a) re-scope the assertions to `>= 27` and `>= 11` (honest but tests a dead surface);
- (b) remove the test entirely (the surface is unreachable);
- (c) wire the real tools into the bridge and update assertions (product decision, not a repair).

---

## F. PUSH STATUS

**UNCHANGED.** `f1b2b36` is preserved exactly. The GitHub connector is connected but its tools have not loaded in this session. No credential workaround was attempted. No PAT was requested or transmitted.

| State | Value |
|---|---|
| Recovery commit | `f1b2b36eaca4f015173f58485d6f6b6dd50ebaef` |
| Remote | `1338d0b` — still broken |
| Pushed | **NO** |
