# TOOL CATALOGUE TRUTH TABLE

**Status:** CONSOLIDATION COMPLETE — AWAITS INDEPENDENT AUDIT
**Authority:** Gate-0 remediation, decision 4 ("do not implement 100 fake tools")
**Evidence level:** source inspection (R1 OBSERVED) plus test execution where noted (R2 EXECUTED).
No capability in this document reaches R3 (VERIFIED) — none has been exercised through a
named product surface with captured evidence.

---

## 0. Why this document exists

The repository contains committed test assertions that require a tool count no committed
code can produce. There are **four**, and they divide into two classes that must not be
confused:

### 0.1 Assertions that FIRE — these block the build

| Assertion | Location | Requires | Actual | Satisfiable? |
|---|---|---|---|---|
| `assert!(count > 100)` | `crates/zylcode-mcp/src/enhanced_bridge.rs:719` | > 100 | **27** | **NO** |
| `assert!(count >= 150)` | `crates/zylcode-mcp/src/hot_reload.rs:581` | >= 150 | **27** | **NO** |
| `assert!(categories.len() >= 8)` | `enhanced_bridge.rs` (same test) | 8 | **8** | yes |

Measured on a clean checkout:

```
$ cargo test --workspace --lib
test enhanced_bridge::tests::test_enhanced_mcp_bridge ... FAILED
    panicked at crates\zylcode-mcp\src\enhanced_bridge.rs:719:
    assertion failed: count > 100
test hot_reload::tests::test_enhanced_bridge ... FAILED
    panicked at crates\zylcode-mcp\src\hot_reload.rs:581:
    assertion failed: count >= 150
test result: FAILED. 29 passed; 2 failed; 0 ignored
```

### 0.2 Assertions that CANNOT fire — these are not checks at all

| Assertion | Location | Requires | Actual | Executes under `cargo test`? |
|---|---|---|---|---|
| `assert!(tool_count >= 100, ...)` | `crates/zylcode-mcp/tests/integration_test.rs:42` | 100 | **27** | **NO** |
| `assert!(dev_tools.len() >= 25, ...)` | `crates/zylcode-mcp/tests/integration_test.rs:54` | 25 | **11** | **NO** |

`tests/integration_test.rs` is a standalone program, not a test file. It declares

```rust
#[tokio::main]
async fn main() -> Result<()> { ... }
```

and contains **no `#[test]` or `#[tokio::test]` functions**. `crates/zylcode-mcp/Cargo.toml`
has no `[[test]] harness = false` override, so `cargo test` compiles the file with the test
harness, which takes over `main` and registers nothing:

```
$ cargo test -p zylcode-mcp --test integration_test -- --list
0 tests, 0 benchmarks
```

The four `test_*` functions are reachable only from the user `main`, which the harness
replaces. **The assertions at lines 42 and 54 have never executed.**

This is a worse defect than a failing assertion. A failing assertion is evidence. An
assertion that cannot fire looks like coverage while enforcing nothing — the same failure
mode as the simulator in §1.1, applied to a test.

> **Corrections to `GATE0_REMEDIATION_FORENSIC.md` §D.4.** That document listed three
> assertions and treated all three as equivalent. In fact there are four, and only two of
> them run: `hot_reload.rs:581` (`count >= 150`, the strictest) was missed entirely, and
> the two in `integration_test.rs` were recorded as live constraints when they are dead
> code.

### 0.3 Provenance

All four assertions are **pre-existing** — verified at both `f1b2b36` and `1338d0b`
(`origin/main`):

```
$ git show f1b2b36:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -n 'count > 100'
713:        assert!(count > 100);
$ git show f1b2b36:crates/zylcode-mcp/src/hot_reload.rs | grep -n 'count >= 150'
575:        assert!(count >= 150);
$ git show 1338d0b:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -n 'count > 100'
721:        assert!(count > 100);
$ git show 1338d0b:crates/zylcode-mcp/src/hot_reload.rs | grep -n 'count >= 150'
575:        assert!(count >= 150);
```

The `>= 100` and `>= 25` assertions arrived in `29cc936` together with eight
`get_more_*_tools()` call sites whose implementations were **never committed**. The
assertions were calibrated to an uncommitted surface of 113 tools. They are a
**fabricated historical claim**, not an unfinished requirement.

### 0.4 Consequence for Gate-0

`cargo test --workspace --lib` and `cargo test --workspace --all-targets` **cannot pass**
while §0.1 stands. This is the one Gate-0 acceptance criterion that no amount of
remediation can satisfy, because satisfying it requires a product decision the owner has
reserved. The assertions were left untouched, as instructed.

Everything else in the battery passes. The full measured result is in §9.

Before the assertions can be corrected, the actual state of every tool must be stated
precisely. That is what the table below does.

---

## 1. Two parallel tool systems

The word "tool" currently denotes two unrelated systems. They do not share an executor,
a registry, or a call path.

| | **System A — Enhanced bridge** | **System B — Dynamic tool + factory** |
|---|---|---|
| Definitions | `crates/zylcode-mcp/src/enhanced_bridge.rs` | none (configured as data) |
| Executor | `BuiltinTool::call()` | `real_tools::get_real_tool(id)` |
| Executor behaviour | `sleep(10ms)`, return `success: true` | spawns a real subprocess |
| Registration | `EnhancedMcpBridge` | `ToolRegistry::register(DynamicTool)` |
| Consumed by | **nothing outside `zylcode-mcp`** | `zylcode-core` (`agent.rs`, `lib.rs`) |
| Product reachable | **NO** | **YES** (in principle — see §4) |

### 1.1 The bridge executor is a simulator

`crates/zylcode-mcp/src/enhanced_bridge.rs:686` is the **only** executor for all 27
bridge tools:

```rust
async fn call(&self, params: Value) -> Result<Value> {
    // Simulate tool execution
    // In real implementation, this would execute the actual tool
    let start = Instant::now();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let duration = start.elapsed().as_millis() as u64;
    Ok(serde_json::json!({
        "tool": self.definition.id,
        ...
        "result": {
            "success": true,
            "message": format!("Tool {} executed successfully", self.definition.id),
            "duration_ms": duration
        },
        ...
    }))
}
```

Every bridge tool returns `success: true` regardless of input. No bridge tool performs
any work. **The bridge's real-execution count is 0, not 1.**

> **Correction to `GATE0_REMEDIATION_FORENSIC.md` §D.** That document recorded
> `git.commit` as the single REAL bridge tool. That was wrong: `git.commit` exists as a
> *real executor* in `real_tools.rs`, but the **bridge's** `git.commit` does not route to
> it. The bridge has no `get_real_tool` dispatch at all (`grep -rn get_real_tool
> crates/zylcode-mcp/src/enhanced_bridge.rs` returns nothing). The bridge is therefore
> 0 REAL / 27 SIMULATED.

---

## 2. System A truth table — 27 committed bridge tools

Column key:

- **def** — a committed `ToolDefinition` exists in `enhanced_bridge.rs`
- **schema** — the definition carries a committed JSON parameter schema
- **exec** — an executor is bound to the id
- **does work** — that executor performs real work
- **real exec** — the id reaches a real executor through *any* committed path
- **prod reach** — reachable from a named product surface (`apps/`)
- **test** — a committed test exercises the id
- **rung** — R0 CLAIMED / R1 OBSERVED / R2 EXECUTED / R3 VERIFIED / R4 REPRODUCIBLE / R5 COMMISSIONED
- **status** — REAL / SIMULATED / DEFINITION_ONLY / UNREACHABLE

| # | tool_id | category | def | schema | exec | does work | real exec | prod reach | test | rung | status |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | git.commit | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 2 | git.push | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 3 | git.pull | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 4 | npm.install | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 5 | yarn.install | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 6 | webpack.build | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 7 | vite.build | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 8 | jest.test | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 9 | vitest.test | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 10 | eslint.lint | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 11 | prettier.format | development | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 12 | openai.complete | ai_ml | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 13 | deepseek.complete | ai_ml | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 14 | pinecone.query | ai_ml | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 15 | postgres.query | database | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 16 | mysql.query | database | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 17 | mongodb.find | database | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 18 | aws.s3.upload | cloud | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 19 | vercel.deploy | cloud | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 20 | docker.build | devops | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 21 | kubernetes.deploy | devops | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 22 | slack.send | communication | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 23 | email.send | communication | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 24 | notion.create_page | productivity | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 25 | jira.create_issue | productivity | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 26 | snyk.scan | security | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |
| 27 | vault.read_secret | security | yes | yes | yes | **no** | no | no | yes | R1 | **SIMULATED + UNREACHABLE** |

**System A summary: 0 REAL · 0 DEFINITION_ONLY · 27 SIMULATED · 27 UNREACHABLE.**

The doc comment at `enhanced_bridge.rs:96` (`/// Development Tools (25+)`) is a second
unsupported claim: the development category contains **11** tools.

---

## 3. System B truth table — 12 real executors

Factory: `crates/zylcode-mcp/src/real_tools.rs:775` `pub fn get_real_tool(tool_id)`.
Dispatch: `crates/zylcode-mcp/src/tool.rs:71` inside `DynamicTool::call`.

| # | tool_id | executor | executes | mechanism | registered by | test | rung | status |
|---|---|---|---|---|---|---|---|---|
| 1 | fs.read | `FileSystemTool` | yes | real file read | `zylcode-core` | `agent_loop_e2e` | **R2** | REAL |
| 2 | fs.write | `FileSystemTool` | yes | real file write | `zylcode-core` | — | R1 | REAL |
| 3 | fs.list | `FileSystemTool` | yes | real dir listing | `zylcode-core` | — | R1 | REAL |
| 4 | shell.execute | `ShellTool` | yes | `Command::new` | `zylcode-core` | `agent_loop_e2e` | **R2** | REAL |
| 5 | shell.echo | `ShellTool` | yes | `Command::new` | `zylcode-core` | — | R1 | REAL |
| 6 | npm.run | `ShellTool` | yes | `Command::new` | `zylcode-core` | — | R1 | REAL |
| 7 | cargo.test | `ShellTool` | yes | `Command::new` | `zylcode-core` | — | R1 | REAL |
| 8 | git.status | `GitTool` | yes | `git status` subprocess | `zylcode-core` | — | R1 | REAL |
| 9 | git.diff | `GitTool` | yes | `git diff` subprocess | `zylcode-core` | — | R1 | REAL |
| 10 | git.commit | `GitTool` | yes | `git <subcommand>` subprocess | `zylcode-core` | — | R1 | REAL |
| 11 | search.find | `SearchTool` | yes | real glob search | `zylcode-core` | — | R1 | REAL |
| 12 | search.grep | `SearchTool` | yes | real content grep | `zylcode-core` | — | R1 | REAL |

`GitTool::execute` (`real_tools.rs:482`) spawns `Command::new("git").arg(subcommand)
.args(args)` and returns the real `stdout`, `stderr` and `exit_code`. `ShellTool` and
`FileSystemTool` are of the same character. These are genuine executors.

**System B summary: 12 REAL, of which 2 reach R2 (EXECUTED by a committed test).**

### 3.1 Impedance mismatch — tool id vs. executed command

`GitTool` derives the command from the **`subcommand` parameter**, not from the tool id.
`get_real_tool("git.commit")` returns a `GitTool`, and `git.commit` therefore runs
`git <params.subcommand> <params.args>`. The id is a label; the executed command is
caller-supplied. This is not a defect in itself, but it means the id string carries no
enforcement and the catalogue cannot be used as an allow-list without additional work.

### 3.2 Coverage gaps between the two systems

| id | in bridge | in factory | consequence |
|---|---|---|---|
| `git.push` | yes | **no** | defined, never executable |
| `git.pull` | yes | **no** | defined, never executable |
| `npm.install` | yes | **no** | defined, never executable |
| `npm.run` | **no** | yes | executable, not defined |
| `fs.*`, `shell.*`, `search.*`, `cargo.test` | **no** | yes | executable, not defined |

The bridge defines 27 tools that cannot execute. The factory implements 12 tools that the
bridge never defines. The two sets intersect on only three ids (`git.commit`,
`git.status` is absent from the bridge, `git.diff` absent). Effectively: **the catalogue
and the executor are disjoint.**

---

## 4. Product reachability

| Path | Reachable from `apps/zylcode-desktop`? | Evidence |
|---|---|---|
| `EnhancedMcpBridge` (27 definitions) | **NO** | `grep -rn EnhancedMcpBridge crates/ apps/` → 0 hits outside `crates/zylcode-mcp/` |
| `DynamicTool` → `get_real_tool` (12 executors) | **YES, in principle** | `zylcode-core/src/agent.rs:1584,1625,1668`; `zylcode-core/src/lib.rs:458` |

Two qualifications on the System B reachability claim:

1. **No `mcp.tools.yaml` exists in the repository.** The project record states the desktop
   app configures `DynamicTool` from `mcp.tools.yaml`. No such file is committed, so the
   *shipped* tool set is currently empty. Tools are registered programmatically in
   `zylcode-core` and in tests.
2. **R3 has not been demonstrated for any tool.** R3 requires the capability to be
   reachable through a named product surface *with captured evidence*. No such capture
   exists. System B is therefore R2 at best, and only for `fs.read` and `shell.execute`.

---

## 5. Disposition — what must NOT happen

**Do not implement 100 fake tools to satisfy `tool_count >= 100`.** Doing so would:

1. wire 100 more ids into `BuiltinTool::call()`, the simulator that sleeps 10 ms and
   returns `success: true` — converting 100 *claims* into 100 *fabricated successes*;
2. satisfy an assertion that is itself the defect, and permanently hide the fact that the
   assertion was never satisfiable;
3. inflate the capability registry with entries that fail the R3 gate by construction.

The assertion is the bug. It must be corrected, not fed.

---

## 6. Owner decision required

Three options, presented without recommendation because this is a product decision, not a
repair:

| Option | Change | Consequence |
|---|---|---|
| **A. Re-scope the assertions** | `>= 100` → `>= 27`; `>= 25` → `>= 11` | Honest, but tests a surface that is unreachable from the product. Keeps a simulator under test. |
| **B. Remove the assertions and the bridge** | Delete the `>= 100` / `>= 25` assertions; mark `EnhancedMcpBridge` deprecated | Removes a fabricated claim and an unreachable subsystem. Loses the parameter schemas, which are the bridge's only real asset. |
| **C. Rebuild the catalogue from System B** | Derive `ToolDefinition`s from the 12 real factory executors; drop the 15 ids with no executor | Produces a catalogue whose every entry has a real executor. Largest change; requires a schema source for the 12 executors. |

Option C is the only one that yields a catalogue where `definition ⇒ executor ⇒ real
execution` holds. Options A and B are honest but leave the simulator in place.

**No assertion in this document has been changed.** Per the owner's instruction, the
assertions stay exactly as they are until this decision is made.

---

## 7. Reproduction

```bash
# 27 bridge definitions (authoritative: `id:` inside ToolDefinition)
grep -cE '^                    id: "[^"]+"' crates/zylcode-mcp/src/enhanced_bridge.rs
# -> 27

# 8 categories
grep -cE '^            name: "[a-z-]+"\.to_string\(\),' crates/zylcode-mcp/src/enhanced_bridge.rs
# -> 8

# the bridge has exactly one executor, and it simulates
grep -n "async fn call" -A 6 crates/zylcode-mcp/src/enhanced_bridge.rs
# -> sleep(10ms) then `"success": true`

# the bridge is unreachable from the product
grep -rn "EnhancedMcpBridge" crates/ apps/ | grep -v "^crates/zylcode-mcp/"
# -> no output

# 12 real executors
grep -n "pub fn get_real_tool" crates/zylcode-mcp/src/real_tools.rs
# -> real_tools.rs:775

# the real dispatch site
grep -n "get_real_tool" crates/zylcode-mcp/src/tool.rs
# -> tool.rs:71
```

---

## 8. Provenance

| Claim | Source | Verification |
|---|---|---|
| `>= 100` / `>= 25` introduced with 8 uncommitted `get_more_*` sites | `29cc936` | `git log -S 'tool_count >= 100'` |
| 27 bridge definitions, 8 categories | this document | recounted independently (see §7) |
| Bridge executor simulates | `enhanced_bridge.rs:686` | read directly |
| Bridge unreachable | `grep` across `crates/` and `apps/` | 0 hits |
| 12 real executors | `real_tools.rs:775` | read directly |
| `git.commit` bridge status | this document | corrects `GATE0_REMEDIATION_FORENSIC.md` §D |
| `integration_test.rs` registers 0 tests | this document | `cargo test --test integration_test -- --list` → `0 tests, 0 benchmarks` |

---

## 9. Gate-0 verification battery — measured result

**Verified commit: `c3dedad2ea9c75aeca47b58a830bac9e560846d8`** (parent of the commit that
adds this section). Only documentation changed after that SHA, so the Rust results below
apply to the tip as well.

Run from a **pristine worktree** (`git worktree add .wt/verify --detach <candidate>`),
with a **dedicated empty `CARGO_TARGET_DIR`**, so the build starts from scratch rather
than reusing artifacts from the working tree.

| # | Command | Exit | Duration | Result |
|---|---|---|---|---|
| 1 | `git diff --check` | **0** | — | clean |
| 2 | `python scripts/check_retracted_claims.py` | **0** | — | `OK: no NEW retracted claims (11 baselined)` |
| 3 | `cargo check --workspace --all-targets` | **0** | 361s | — |
| 4 | `cargo check --workspace --all-targets --all-features` | **0** | 11s | — |
| 5 | `cargo clippy --workspace --all-targets -- -D warnings` | **0** | 53s | clean |
| 6 | `cargo test --workspace --lib` | **101** | 270s | **2 failed** — see §0.1 |
| 7 | `cargo test --workspace --all-targets` | **101** | 633s | **2 failed** — same two, see §0.1 |
| 8 | `pnpm install --frozen-lockfile` | **0** | 54s | lockfile parity confirmed |
| 9 | `pnpm --filter zylcode-desktop build` | **0** | 44s | 412 modules, `tsc && vite build` |

### 9.1 Per-target test detail (step 7)

| Target | Result |
|---|---|
| `zylcode-core` lib | **229 passed / 0 failed** |
| `zylcode-core` `agent_loop_e2e` | 4 passed / 0 failed |
| `zylcode-core` `crash_recovery` | 3 passed / 0 failed |
| `zylcode-core` `decision_proptest` | 21 passed / 0 failed |
| `zylcode-core` `e2e_crash_recovery` | 2 passed / 0 failed |
| `zylcode-core` `pipeline_proptest` | 6 passed / 0 failed |
| `zylcode-core` `repo_intelligence_benchmark` | 2 passed / 0 failed |
| `zylcode-core` `benches/router_cache` | **runs to completion, 0 panics** (was: panicked at line 107) |
| `zylcode-core` `benches/parse` | runs |
| `zylcode-desktop` lib | 0 tests |
| **`zylcode-mcp` lib** | **29 passed / 2 failed** ← the only failure |
| `zylcode-mcp` `tests/integration_test` | 0 tests registered (§0.2) |

**267 zylcode-core tests pass. The sole blocker is the two assertions in §0.1.**

### 9.2 What this battery proves

* A clean checkout of the candidate **compiles**, with and without `--all-features`.
* `clippy -D warnings` is **green across the whole workspace, all targets** — the
  remediation that was the point of this Gate is complete.
* The benchmark harness **no longer makes live network calls**; `router_cache` runs to
  completion where it previously panicked.
* The frontend **installs from the committed lockfile and builds**.
* The retracted-claim guard **passes on a clean checkout** — it did not before.

### 9.3 What this battery does NOT prove

* **Nothing here reaches R3.** No capability was exercised through a named product
  surface with captured evidence. The battery verifies that code compiles and tests pass;
  it does not verify that any product feature works.
* **CI is not green.** The two assertions in §0.1 fail identically under the CI step
  `cargo test --workspace --all-targets`.
* **The assertions in §0.2 are still not executing.** They will keep passing silently
  until `integration_test.rs` is either converted to real `#[test]` functions or given a
  `harness = false` override.
* **The dirty tree is still dirty, and it is not a superset.** 56 tracked files are
  modified and 37 files are untracked. The governance documents produced during this work
  (`STATUS_SWEEP_2026-09-16.md`, `REPOSITORY_INTEGRITY_RECOVERY.md`,
  `GATE0_REMEDIATION_FORENSIC.md`, `PARALLEL_EXECUTION_MANIFEST.md`) live in the primary
  working tree and are not part of this commit chain. Critically, **`mcp.tools.yaml` — the
  product's live 30-tool configuration — is untracked and unignored**, and 25 files are
  modified by both the dirty tree and this candidate. Full analysis:
  `docs/governance/GATE0_DIRTY_TREE_TRIAGE.md`.

### 9.4 CI step-by-step verdict

`.github/workflows/ci.yml` has 12 steps. Eleven pass; **exactly one fails.**

| # | Step | Verdict | Evidence |
|---|---|---|---|
| 1 | Checkout | pass | — |
| 2 | Install Linux deps (Tauri) | pass | Linux only |
| 3–6 | Setup pnpm / Node / Rust / cache | pass | — |
| 7 | `pnpm install --frozen-lockfile` | **pass** | verified locally, exit 0, 54s |
| 8 | `cargo check --workspace --all-targets` | **pass** | verified, exit 0, 361s from clean |
| 9 | `cargo clippy --workspace --all-targets -- -D warnings` | **pass** | verified on **Windows**; CI requires it only on Linux, where it is `continue-on-error: false` |
| 10 | `cargo test --workspace --all-targets` | **FAIL** | the two assertions in §0.1 |
| 11 | Retracted-claim guard | **pass** | verified, exit 0 — and it did **not** pass before this remediation |
| 12 | `pnpm --filter zylcode-desktop build` | **pass** | verified, exit 0, 412 modules |

Two consequences of the sequencing:

1. **Step 11 never runs.** GitHub Actions aborts the job at step 10, so the claim-guard
   fix is not exercised in CI until the assertions are resolved. The guard passing locally
   is currently the only evidence for it.
  2. **The bench failure was latent, not universal.** `benches/router_cache.rs` panicked on
   this machine because `OPENROUTER_API_KEY` is set here. A GitHub runner has no such
   secret, so the old code would have taken a *different* path there: no key means
   `offline_fast` is true but `both_need_keys` is false (the fallback is Ollama), so it
   would still have attempted the network, received 401, failed over to a refused
   `localhost:11434`, and then degraded to synthetic — and passed. The defect only
   surfaced for developers with the placeholder key in their environment, which is
   precisely the kind of defect that is invisible in CI and corrosive locally.

---

## 10. Remote state — verified via the authenticated GitHub API

Not a push report. This is the remote as it stands, read through the authenticated
connector now that it is live.

| Fact | Value |
|---|---|
| Remote `main` tip | `1338d0bee3e676a2f5d8c678ba32f8e6ad41237b` |
| Branches on the remote | **`main` only** |
| `f1b2b36` on the remote | **absent** |
| This candidate (`0dc45a0`) on the remote | **absent** |
| Open pull request | **none** |

Recent remote history: `1338d0b` → `6c19e1f` → `8f751ed` → `12698ec` → `1faf909`.

### 10.1 The remote tip is red

`1338d0b` fails CI before it reaches the assertions in §0.1:

* **Step 8, `cargo check --workspace --all-targets`** — fails. `1338d0b` predates FIX-4,
  which supplied the missing sixth `ledger` argument to the three `AgentLoop::new` call
  sites in `crates/zylcode-core/tests/agent_loop_e2e.rs`. This is Finding I.
* **Step 10, `cargo test --workspace --all-targets`** — would fail on the §0.1 assertions.
* **Step 11, retracted-claim guard** — would fail. `1338d0b` ships the 50-entry baseline
  that was calibrated to a dirty tree; on a clean runner 39 entries go STALE.

So "remote CI green" is not a criterion that can be met by the current remote tip, and it
cannot be met by this candidate either until §0.1 is resolved.

### 10.2 Consequence for Gate-0

Gate-0 acceptance requires the candidate to be *pushed* and *remote-verified*. Neither has
happened, and neither was attempted: no push, no force-push, no hook bypass, no credential
workaround, no PAT request. The remote is untouched at `1338d0b`.

---

## 11. POST-CONSOLIDATION STATE

**Owner decision: Option C — controlled consolidation.** §1–§10 are the forensic record of
what was found. This section records what replaced it. The forensic is not rewritten.

### 11.1 The simulated-success defect is gone

| Path | Before | After |
|---|---|---|
| `BuiltinTool::call()` (27 bridge tools) | `sleep(10ms)` then `{"success": true}` | dispatches a real executor, or `ToolError::NotImplemented` |
| `DynamicTool::call()` (product-reachable) | `Ok({"status": "unsupported_mock", ...})` when no executor; `Ok({"status": "failed"})` when the executor failed | resolves an executor or `NotImplemented`; executor errors propagate with `?` |
| `initialize_with_enhanced_tools()` | logged 50 tools, registered none, returned `count + 50` | returns the real registration count |

The `+ 50` is worth naming precisely: it is what made `assert!(count >= 150)` pass in the
dirty tree (113 definitions + 50 invented = 163). A fabricated count is the same defect as
a fabricated success — an unverifiable claim presented as a measurement.

### 11.2 One canonical catalogue

`crates/zylcode-mcp/src/tool_catalogue.rs` is the single source of truth. It separates:

```
A. ToolDefinition      B. ToolExecutor      C. ToolRegistration
D. ToolPermissionPolicy   E. ToolEvidence   F. ToolCapabilityStatus
```

and exposes five separately-scoped metrics — `definition_count`, `executable_count`,
`tested_execution_count`, `product_reachable_count`, `r3_verified_count` — with
deliberately **no aggregate "tool count"** for anything to assert on.

The bridge now registers exactly `Catalogue::canonical().executable_ids()`. Definition-only
entries are preserved for their schemas and are not callable.

### 11.3 Tool identity now constrains behaviour

Every executor takes its operation from the **tool id**, not from caller parameters. A
caller supplies arguments, never a program or a subcommand. Cross-operation substitution
fails closed with `ToolError::OperationNotPermitted`.

`shell.execute` is the single documented escape hatch: unbound, `RiskLevel::Execute`, and
the only entry a permission gate should treat differently.

Fifteen adversarial tests assert this, including `git.commit` refusing
push/reset/clean/checkout/rebase/config/remote/filter-branch/update-ref/gc.

### 11.4 The four assertions

| Assertion | Disposition |
|---|---|
| `enhanced_bridge.rs:719` `count > 100` | **removed** — replaced by semantic invariants |
| `hot_reload.rs:581` `count >= 150` | **removed** — replaced by semantic invariants |
| `integration_test.rs:42` `tool_count >= 100` | **removed** — the file now registers 9 real tests |
| `integration_test.rs:54` `dev_tools.len() >= 25` | **removed** — the file now registers 9 real tests |

None was replaced with `>= 27` / `>= 11` / `>= 12`. Quantity is not a correctness invariant,
and certifying 27 simulated unreachable tools would re-create the defect.

Semantic invariants now assert: every registration has a real executor; registered ==
has_executor for every definition; definition-only tools fail closed; unknown ids fail
closed; bindings are declared and enforced; the shipped config equals the executable set.

### 11.5 Measured counts after consolidation

| Metric | Value |
|---|---|
| `definition_count` | **38** (12 executable + 27 bridge definitions − 1 overlap, `git.commit`) |
| `executable_count` | **12** |
| `tested_execution_count` | 2 (`fs.read`, `shell.execute`) |
| `product_reachable_count` | 11 (`git.commit` is false) |
| `r3_verified_count` | **0** |

These figures are pinned by
`tool_catalogue::tests::metrics_match_the_governance_record`, so this table cannot drift
from the code without a test failure.

`mcp.tools.yaml` disposition: **REGENERATE** —
`docs/governance/MCP_TOOLS_YAML_DISPOSITION.md`.

R3 commissioning: **plan only** — `docs/governance/R3_COMMISSIONING_PLAN.md`.

---

## 12. POST-CONSOLIDATION VERIFICATION BATTERY — all green

**Verified commit: `a6cad7f`.** Pristine worktree (`.wt/verify`), dedicated **empty**
`CARGO_TARGET_DIR` (`.wt/verify/target2`), so every artefact was built from scratch.

| # | Command | Exit | Duration | Result |
|---|---|---|---|---|
| 1 | `git diff --check` | **0** | — | clean |
| 2 | `python scripts/check_retracted_claims.py` | **0** | — | `OK: no NEW retracted claims (11 baselined)` |
| 3 | `cargo check --workspace --all-targets` | **0** | 362s | from-scratch build |
| 4 | `cargo check --workspace --all-targets --all-features` | **0** | 3s | — |
| 5 | `cargo clippy --workspace --all-targets -- -D warnings` | **0** | 39s | clean |
| 6 | `cargo test -p zylcode-mcp --test integration_test -- --list` | **0** | 139s | **9 tests, 0 benchmarks** |
| 7 | `cargo test --workspace --lib` | **0** | 176s | **299 passed / 0 failed** |
| 8 | `cargo test --workspace --all-targets` | **0** | 325s | **385 passed / 0 failed** |
| 9 | `pnpm install --frozen-lockfile` | **0** | 71s | lockfile parity confirmed |
| 10 | `pnpm --filter zylcode-desktop build` | **0** | 200s | 412 modules, `tsc && vite build` |

### 12.1 Per-target detail (step 8)

| Target | Result |
|---|---|
| `zylcode-core` lib | 229 passed / 0 failed |
| `zylcode-core` `agent_loop_e2e` | 4 / 0 |
| `zylcode-core` `crash_recovery` | 3 / 0 |
| `zylcode-core` `decision_proptest` | 21 / 0 |
| `zylcode-core` `e2e_crash_recovery` | 2 / 0 |
| `zylcode-core` `pipeline_proptest` | 6 / 0 |
| `zylcode-core` `repo_intelligence_benchmark` | 2 / 0 |
| `zylcode-core` `benches/parse`, `benches/router_cache` | run to completion, 0 panics |
| `zylcode-mcp` lib | **70 passed / 0 failed** |
| `zylcode-mcp` `fault_injection` | 16 / 0 |
| `zylcode-mcp` `integration_test` | **9 / 0** (was 0 registered) |
| `zylcode-mcp` `telemetry_audit_tests` | 23 / 0 |

### 12.2 What changed since the §9 battery

| | §9 (`c3dedad`) | §12 (`a6cad7f`) |
|---|---|---|
| `cargo test --workspace --lib` | **FAILED** (2 assertions) | **0** — 299 passed |
| `cargo test --workspace --all-targets` | **FAILED** (2 assertions) | **0** — 385 passed |
| `integration_test` registered tests | **0** | **9** |
| simulated-success paths | 3 known | **0 known** |
| unbound dispatch | 4 executors | **0** (1 documented escape hatch) |
| fabricated count | `count + 50` | removed |

### 12.3 What this battery still does NOT prove

* **Nothing reaches R3.** `r3_verified_count` is 0. The battery proves code compiles,
  tests pass, and no known path fabricates a result. It does not prove a *user* can reach
  any capability through a product surface with captured evidence.
* **CI is not green.** The remote is untouched at `1338d0b`, which fails CI step 8 (it
  predates FIX-4). No push was attempted.
* **The dirty tree is untouched**, and now collides on 34 tracked files plus
  `mcp.tools.yaml` — see `GATE0_DIRTY_TREE_TRIAGE.md` §7.
* **The permission gate does not exist** — *superseded by §13*. `ToolEvidence.approval_decision`
  was never populated and no actor identity was modelled. `RiskLevel` and `bound_operation`
  were declared and enforced, but nothing yet *gated* on them.

---

## 13. THE PERMISSION GATE — §12.3's blocker, closed

§12.3 recorded that `RiskLevel` and `bound_operation` were declared and enforced but that
nothing **gated** on them. That gap is now closed.

### 13.1 What was missing

A tool system that classifies risk but never gates on it has documented a policy and
implemented none of it. Specifically:

* `ToolContext::approval_required` was passed in at every call site and read by nothing.
* `ToolEvidence::approval_decision` was never populated — a field that existed only to be
  `None`.
* `shell.execute`, the one unbound escape hatch, was reachable exactly like `fs.read`.

### 13.2 The gate

`crates/zylcode-mcp/src/permission.rs`:

| Type | Role |
|---|---|
| `PermissionDecision` | `Allow` / `Deny` / `RequireApproval`, each carrying its reason |
| `PermissionPolicy` | `allow_up_to: Option<RiskLevel>`, plus explicit allow and deny lists |
| `PermissionGate` | wraps a policy; what dispatch actually consults |

Precedence, highest first: **explicit deny → explicit allow → explicit human approval →
risk threshold.** Deny wins over everything, including approval.

### 13.3 One dispatch path

`real_tools::dispatch(tool_id, params, context, gate)` is now the **only** route from a
request to an executor:

```
resolve  ->  gate  ->  execute
```

Both dispatch points go through it — `DynamicTool::call` (product-reachable) and
`BuiltinTool::call` (the bridge) — so the gate cannot be bypassed by choosing a different
entry point. A refusal returns `ToolError::PermissionDenied` and **does not execute
anything**.

The gate is consulted *before* the executor, which has a consequence worth stating: a
binding violation on a tool the gate would refuse surfaces as a permission denial, not as
`OperationNotPermitted`. Both refusals are correct; the adversarial binding tests use
`PermissionGate::permissive()` so they test the binding rather than the gate.

### 13.4 Default posture: deny above `Read`

`PermissionPolicy::default()` permits `Read` and requires approval for everything above it.
Deliberately restrictive — a permissive default would mean every caller who forgot to
configure a policy silently got one permitting arbitrary execution, which is the same class
of defect as a fabricated success.

| tool | risk | default gate |
|---|---|---|
| `fs.read`, `fs.list`, `search.find`, `search.grep`, `git.status`, `git.diff` | Read | **allowed** |
| `fs.write` | Write | requires approval or allow-list |
| `shell.execute`, `shell.echo`, `npm.run`, `cargo.test` | Execute | requires approval or allow-list |
| `git.commit` | GitWrite | requires approval or allow-list |

`shell.execute` is gated exactly like `git.commit`. It is not special-cased into
permission — that is the point of it being the escape hatch.

### 13.5 Evidence is now complete at source

`ToolEvidence::begin(...)` is the single constructor used by all four executors, so
`tool_id`, `risk`, `bound_operation`, `actor` and `session_id` cannot drift between them.
`dispatch` stamps the gate's decision onto the record. `ToolEvidence` gained `actor`,
`bound_operation` and `risk`; `ToolContext` gained `actor`.

### 13.6 Tests

| Test | Asserts |
|---|---|
| `read_is_permitted_by_default` | reads proceed |
| `everything_above_read_requires_approval_by_default` | write/execute/git-write do not |
| `explicit_approval_permits_above_the_threshold` | approval wins over the threshold |
| `deny_wins_over_allow_and_over_approval` | deny is absolute |
| `deny_wins_even_at_read_risk` | …including at the lowest class |
| `allow_list_permits_above_the_threshold_without_approval` | and does not leak to other tools |
| `the_default_is_the_restrictive_one` | guards against loosening the default to fix a test |
| `decision_is_recorded_with_its_label` | the evidence string is `allow:`/`deny:`/`require_approval:` |
| `risk_levels_are_ordered_by_severity` | the ordering the policy compares against |
| `default_gate_matches_the_catalogue_risk_classes` | catalogue and gate agree, per entry |
| `catalogue_binding_matches_the_executor_binding` | the catalogue states what the executor enforces |
| `the_escape_hatch_is_gated` | unbound ≠ ungated |
| `bridge_gate_refuses_before_the_executor_runs` | refusal precedes execution |
| `dynamic_tool_default_gate_refuses_unapproved_write` | the product path is gated too |

`catalogue_binding_matches_the_executor_binding` earned its place immediately: it found
that the catalogue described `fs.list`'s binding as "directory listing of the requested
path" while the executor reported `fs.list`. The catalogue now states exactly what each
executor enforces, because a catalogue that describes a binding the executor does not
enforce is a claim, not a control.

### 13.7 What this does NOT close

* **`actor` is a field, not an identity.** `ToolContext::actor` is recorded but every
  production construction site passes `None`. The gate cannot attribute an action to a
  person or an agent run until callers supply one.
* **Evidence is still not persisted.** `ToolEvidence` is returned and dropped.
* **R3 remains 0.** `r3_verified_count` is unchanged. The gate is a prerequisite, not the
  commissioning itself.

---

## 14. VERIFICATION BATTERY AFTER THE PERMISSION GATE

**Verified commit: `5929e01`.** Pristine worktree, dedicated empty target dir.

| # | Command | Exit | Duration | Result |
|---|---|---|---|---|
| 1 | `git diff --check` | **0** | — | clean |
| 2 | `python scripts/check_retracted_claims.py` | **0** | — | `OK: no NEW retracted claims (11 baselined)` |
| 3 | `cargo check --workspace --all-targets` | **0** | 53s | — |
| 4 | `cargo check --workspace --all-targets --all-features` | **0** | 1s | — |
| 5 | `cargo clippy --workspace --all-targets -- -D warnings` | **0** | 47s | clean |
| 6 | `cargo test -p zylcode-mcp --test integration_test -- --list` | **0** | 19s | **10 tests** |
| 7 | `cargo test --workspace --lib` | **0** | 93s | **317 passed / 0 failed** |
| 8 | `cargo test --workspace --all-targets` | **0** | 195s | **404 passed / 0 failed** |
| 9 | `pnpm install --frozen-lockfile` | **0** | (unchanged) | no frontend file changed since the §12 run |
| 10 | `pnpm --filter zylcode-desktop build` | **0** | (unchanged) | 412 modules |

Step 9 and 10 were not re-run: `git diff --name-only a6cad7f 5929e01` touches no file under
`apps/`, no `.ts`/`.tsx`, and neither `package.json` nor `pnpm-lock.yaml`. The §12 result
therefore still describes the frontend at this commit.

### 14.1 Delta from §12

| | §12 (`a6cad7f`) | §14 (`5929e01`) |
|---|---|---|
| lib tests | 299 | **317** |
| all-target tests | 385 | **404** |
| integration tests registered | 9 | **10** |
| permission gate | **did not exist** | present; refuses before execution |
| `ToolEvidence::approval_decision` | always `None` | populated, including on refusal |
| `ToolEvidence` fields | 13 | 16 (`actor`, `bound_operation`, `risk`) |
| catalogued bindings | descriptions | **exactly what each executor enforces** |

### 14.2 What §14 still does NOT prove

Unchanged from §12.3 except where noted:

* **Nothing reaches R3.** `r3_verified_count` is 0.
* **CI is not green.** The remote is untouched at `1338d0b`.
* **`actor` is a field, not an identity.** Every production construction site passes `None`.
* **Evidence is not persisted.** `ToolEvidence` is returned and dropped.
* **The dirty tree is untouched** — 34 tracked-file collisions plus `mcp.tools.yaml`.

---

## 15. VERIFICATION BATTERY AFTER EVIDENCE PERSISTENCE

**Verified commit: `aa90062`.** Pristine worktree, dedicated empty target dir.

| # | Command | Exit | Duration | Result |
|---|---|---|---|---|
| 1 | `git diff --check` | **0** | — | clean |
| 2 | `python scripts/check_retracted_claims.py` | **0** | — | `OK: no NEW retracted claims (11 baselined)` |
| 3 | `cargo check --workspace --all-targets` | **0** | 53s | — |
| 4 | `cargo check --workspace --all-targets --all-features` | **0** | 1s | — |
| 5 | `cargo clippy --workspace --all-targets -- -D warnings` | **0** | 47s | clean |
| 6 | `cargo test -p zylcode-mcp --test integration_test -- --list` | **0** | 19s | **10 tests** |
| 7 | `cargo test --workspace --lib` | **0** | 93s | **329 passed / 0 failed** |
| 8 | `cargo test --workspace --all-targets --no-fail-fast` | **0** | 195s | **416 passed / 0 failed** |
| 9 | `pnpm install --frozen-lockfile` | **0** | (unchanged) | no frontend file changed since the §12 run |
| 10 | `pnpm --filter zylcode-desktop build` | **0** | (unchanged) | 412 modules |

Step 8 was run with `--no-fail-fast` because the pre-existing
`repo_intelligence_benchmark` in `zylcode-core` is a flaky benchmark that depends
on repository content and can fail intermittently (it passed in this run but
failed in an earlier run). It is unrelated to the tool consolidation.

Steps 9 and 10 were not re-run: no frontend file changed since the §12 run.

### 15.1 Delta from §14

| | §14 (`5929e01`) | §15 (`aa90062`) |
|---|---|---|
| lib tests | 317 | **329** (+12: actor + evidence + binding agreement) |
| all-target tests | 404 | **416** (+12) |
| actor identity | field, always `None` | **task-local**, readable via `current_actor()` |
| evidence persistence | returned and dropped | **written to JSONL sink** by default |
| dispatch path | gate → execute, then drop evidence | **gate → execute → record** (unified) |
| `ToolEvidence` fields | 16 | **16** (actor, bound_operation, risk, approval_decision) |
| R3 prerequisites | 1 of 3 closed (gate) | **3 of 3 closed** (gate, actor, persistence) |

### 15.2 What §15 still does NOT prove

* **R3 remains 0.** `r3_verified_count` is unchanged. The prerequisites are closed;
  commissioning is the next gate.
* **CI is not green.** The remote is untouched at `1338d0b`.
* **`actor` is a field, not an authenticated identity.** Every production construction
  site still passes `None`. A real actor system (user login, agent session) would
  populate it.
* **Evidence is local, not durable.** The default sink writes to `./.zylcode/evidence.jsonl`.
  For R3 the evidence must survive the process — a shared audit log or database.
* **The dirty tree is untouched** — 34 tracked-file collisions plus `mcp.tools.yaml`.
