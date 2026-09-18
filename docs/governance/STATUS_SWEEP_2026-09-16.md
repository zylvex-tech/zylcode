# ZylCode Status Sweep — Full Repository, 2026-09-16

**Type:** independent observation sweep (not a completion report)
**Method:** direct repository inspection + build/test execution
**Head at sweep:** `1faf90944a132fcb008c9a1dcad19f85286c3628` (identical to `origin/main`)
**Verdict:** **NOT GREEN.** One committed test failure and one unstarted remediation block the ladder.

> **UPDATE (2026-09-17) — see §14.** The committed test failure reported below has been **FIXED**, and
> a deeper defect beneath it was found: the test was reading a **persisted `./vector_cache.db`**, so it
> could pass without exercising its own code path. Both are resolved. **The suite is green
> (261 passed, 0 failed).** Phase 2A remediation remains unstarted — the ladder is unchanged.

---

## 1. Where we are right now — one paragraph

ZylCode is a **partial Agent Kernel on a partial Trust Foundation**, with a **non-integrated
Intelligence Graph**, a **non-functional Computer-Use skeleton**, and **nothing above it**. The
governing program sits at **Phase 2A RE-OPENED**; **Phase 2B is BLOCKED**; **Phases 3A–16 are NOT
STARTED**. The workspace **compiles**, and **225 of 226 library tests pass** — but the suite is **red**
on one committed test, and the red test is the *same* finding the Phase 2A audit recorded. No
remediation of Phase 2A has begun.

---

## 2. What I actually executed

| Check | Command | Result |
|---|---|---|
| Build (workspace) | `cargo build --workspace` | ❌ **BLOCKED BY SANDBOX** — not a code failure (see §3) |
| Type/borrow check | `cargo check --workspace --lib --offline` | ✅ **PASS** — `Finished dev profile in 4.78s` |
| Library tests | `cargo test --workspace --lib --offline` | ❌ **225 passed; 1 failed** in 8.68s |
| Repo scope | `git ls-files` | 243 tracked files |
| Disk scope | `find . -type f` | 24,939 files (18,215 under `target/`, 6,363 under `node_modules/`) |

**Reproduction**

```
$ cargo check --workspace --lib --offline
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.78s

$ cargo test --workspace --lib --offline
running 226 tests
test router::tests::synthetic_offline_dispatch_returns_parseable_payload ... FAILED
thread 'router::tests::synthetic_offline_dispatch_returns_parseable_payload'
  panicked at crates\zylcode-core\src\router.rs:1030:9:
test result: FAILED. 225 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 3. The build "failure" is a sandbox artifact — corrected finding

`cargo build --workspace` reported:

```
error: failed to run custom build command for `tauri-plugin-shell v2.3.5`
could not execute process `...build-script-build` (never executed)
Caused by: Access is denied. (os error 5)
```

**This is not a code defect.** The sandbox denies **process execution** of build-script binaries
inside `target/`. I confirmed the diagnosis two ways:

1. `touch target/.sweep_probe` → **succeeded**. Writes are permitted; only *execution* is denied.
2. `Get-Process` found **no** `zylcode`, `cargo`, or `rustc` process holding locks, so it is not a
   file-lock contention problem.
3. `cargo check --workspace --lib --offline` — which needs no build-script execution for
   already-built dependencies — **passed cleanly**.

**Consequence for reporting:** this sweep **cannot** report a workspace build as PASS. It reports
`BUILD = NOT OBSERVABLE IN SANDBOX`. Any claim that the workspace builds must come from a run outside
the sandbox (owner machine or CI). Do not convert this to a pass.

---

## 4. The one real failure — root cause identified

**Test:** `crates/zylcode-core/src/router.rs:1022` `synthetic_offline_dispatch_returns_parseable_payload`

**Assertion that fails (line 1030):**
```rust
assert!(text.contains("<zylcode-response>"));
```

**What the code actually returns** (`router.rs:947`, `synthetic_response`):
```rust
serde_json::json!({
    "action": "Complete",
    "payload": { "summary": format!("Task completed: {}", prompt), ... }
}).to_string()
```

**Diagnosis:** the function was changed to return **AgentDecision JSON** for the agent protocol, but
the test still asserts an **XML tag** that no longer exists. The test is a **stale contract
assertion**, not a product regression — the second assertion (`contains("build a counter")`) would
pass, since the prompt is interpolated into the JSON summary.

**Verified this is committed, not a working-tree artifact:**
```
$ git show HEAD:crates/zylcode-core/src/router.rs | grep -A12 "fn synthetic_response"
    serde_json::json!({ "action": "Complete", ...
$ git show HEAD:crates/zylcode-core/src/router.rs | grep -A10 "synthetic_offline_dispatch"
    assert!(text.contains("<zylcode-response>"));
```
Both the JSON body and the XML assertion are present **in HEAD**. The committed tree is red.

**Why it matters beyond the one test:** `.github/workflows/ci.yml:77` runs
`cargo test --workspace --all-targets`. On a runner where CI is enabled, this test fails the build.
It is the second consecutive pass carrying the *same* failure — the audit recorded it, and it is
still unfixed.

**Note:** this test exercises `dispatch_prompt` with `RouterConfig::default()` and no API keys. It
therefore depends on the synthetic-offline branch, which is why it is deterministic — and why the
assertion is simply wrong rather than flaky.

---

## 5. Phase 2A remediation — nothing has started

The audit's required remediation (`PHASE2A_INDEPENDENT_AUDIT.md` §9) is **P0 1–5, P1 6–9, P2 10**.
Status of each, checked against the code:

| # | Requirement | Status | Evidence |
|---|---|---|---|
| **P0-1** | Fix path exclusion; normalise separators or use `Path::components()`; replace hand-rolled matcher with the `ignore` crate; test against the **real** `.gitignore` | ❌ **NOT DONE** | `classifier.rs:220` `should_exclude()` still uses `path.to_string_lossy()` with no normalisation. `scanner.rs:61` still `WalkDir` + hand-rolled `is_gitignored`. **`ignore` crate is not a dependency.** The array *was* partially patched (bare `target/`, `node_modules/`, `.git/` added at lines 237–239), which is why the defect now *partially* self-heals on Windows — a mitigation, not the fix the audit required. |
| **P0-2** | Re-baseline every number; expected genuine index ≈ **254 files** | ❌ **NOT DONE** | Measured now: **243 tracked** + 35 untracked = **278 non-ignored files**, against **24,939 on disk**. No re-baselined figure is recorded anywhere. |
| **P0-3** | Make the benchmark deterministic and self-enforcing; assert headline numbers | ❌ **NOT DONE** | Thresholds still assert only `file_count() > 50` per the audit. |
| **P0-4** | Fix the timing budget (276.1s measured vs 12.7s claimed) | ❌ **NOT DONE** | No files/second budget present. |
| **P0-5** | Produce a real acceptance demonstration (committed transcript, replayable) | ❌ **NOT DONE** | No such transcript exists. |
| **P1-6** | Integrate into `AgentLoop` **or** downgrade the 11 GREEN entries | ⚠️ **HALF-APPLIED** | The registry *was* corrected: 12 entries now `DISPUTED`, `summary.green: 20` with an explanatory note. **But the integration still does not exist** — so the honest resolution (downgrade to `PARTIAL`/`R2` with rungs) is incomplete: 23 of 34 entries still carry `rung: null`. |
| **P1-7** | Strengthen thresholds; `Precision@10 ≥ 0.60`; fix Q5/Q6/Q10 to non-zero | ❌ **NOT DONE** | Unchanged. |
| **P1-8** | Fill `Commit SHA` / `Push Verification` **before** committing the report | ❌ **NOT DONE** | Historical defect in the 2A report. |
| **P1-9** | Commit or discard the modified files | ❌ **WORSE** | **94 entries** in `git status` now (53 at audit; 89 earlier in this sweep; **94 by the end** — another session is actively writing). Of these, **56 are under `crates/`+`apps/`** (source the audit did not review) and 8 under `docs/`. The tree has become *more* dirty, and now includes untracked **source** files. |
| **P2-10** | Reproduction section in the completion report | ❌ **NOT DONE** | — |

**Net: 0 of 10 remediation items complete. 1 half-applied.**

---

## 6. What "GREEN" would actually require

GREEN is not one switch. Under the Proof Graph (**R0 CLAIMED → R1 OBSERVED → R2 EXECUTED →
R3 VERIFIED → R4 REPRODUCIBLE → R5 COMMISSIONED**), and the rung-ceiling rule (A7), the minimum path is:

### Gate 1 — get back to a *working* baseline (prerequisite for everything)

| # | Required | Why |
|---|---|---|
| 1 | **Fix the `router.rs:1030` assertion** | The suite must be green before any claim. One-line fix: assert the JSON contract, or restore the XML contract. |
| 2 | **Establish a reproducible build** | `cargo build --workspace` must be run and captured outside the sandbox. Until then, BUILD is unobserved. |
| 3 | **Resolve the 94-entry dirty tree** | An unreviewed tree is not a reviewable state (audit P1-9). Now includes untracked source. |

### Gate 2 — Phase 2A remediation (P0 1–5)

| # | Required | Why it blocks GREEN |
|---|---|---|
| 4 | **`should_exclude()` normalised; `ignore` crate adopted** | The index currently counts build output on the primary target platform. Any metric from it is mis-scoped. |
| 5 | **Re-baselined numbers from a correctly-scoped index (~278 files)** | Correcting a false metric is what makes the capability honest. |
| 6 | **Deterministic, self-enforcing benchmark** | A benchmark that passes on the author's machine and fails on a reviewer's is not evidence. |
| 7 | **Machine-independent timing budget** | `12.7s` vs `276.1s` is a 21× discrepancy; a wall-clock threshold cannot be the gate. |
| 8 | **Real acceptance demonstration** | R3 requires a captured transcript a third party can replay. |

### Gate 3 — honesty (P1 6–8)

| # | Required | Why |
|---|---|---|
| 9 | **Integrate repository intelligence into `AgentLoop`, or downgrade all 11 entries with rungs** | **This is the specific thing that makes R2 → R3 impossible today.** No named product surface reaches the module. |
| 10 | **`Precision@10 ≥ 0.60` with non-zero Q5/Q6/Q10** | The current bar is weaker than keyword search. |
| 11 | **Correct the registry's remaining `rung: null` entries** | 23 of 34 entries have no rung. A registry without rungs cannot support a claim. |

### Gate 4 — independent re-audit (G3)

| # | Required | Why |
|---|---|---|
| 12 | **Third-party reproduction of the remediated commit** | Builders do not certify their own work. Phase 2A is re-accepted only by re-audit. |

**Only after Gate 4 does Phase 2A reach R3 and Phase 2B unblock.** Everything above is the minimum —
not the path to a shipped product, which is Phases 3A–16.

---

## 7. Rung table — current (unchanged from `ARCHITECTURE_V2.md` §9)

| System | Rung | Status |
|---|---|---|
| Trust foundation — Evidence Ledger | **R3** | PARTIAL |
| Agent Kernel | **R3** | PARTIAL — *live multi-provider commissioning outstanding* |
| Execution Engine — shell/tools | **R3** | PARTIAL |
| Intelligence Graph | **R2** | PARTIAL — fails benchmark on re-run; zero product integration |
| Model Platform | **R1** | PARTIAL — no capability-based routing |
| Delivery Engine | **R1** | PARTIAL — CI blocked externally |
| Project System · Mission Engine · Extension Platform · Artifact Bus · Live Preview | **R0** | PROPOSED |
| Execution Engine — browser / Android / macOS-iOS | **R0** | PROPOSED |
| Vision Studio · Visual Intelligence | **R0** | PROPOSED |
| **Computer-Use Engine** | **R0** | **PROPOSED — non-functional skeleton present** |
| Proof Engine v2 · Multi-Agent · Marketplace | **R0** | PROPOSED |

**Highest rung in the repository: R3.** Nothing is at R4 or R5. No public capability claim above
`AVAILABLE` is permitted — see `PUBLIC_CLAIM_MATRIX.yaml`.

---

## 8. Program status board (authoritative)

| Phase | Status |
|---|---|
| 1A · 1B | ✅ ACCEPTED (R3) |
| 1C · 1D | ✅ ACCEPTED WITH CONDITIONS (R3) |
| **2A** | ❌ **NOT ACCEPTED — RE-OPENED** (R2) |
| **2B** | 🚫 **BLOCKED on 2A** |
| 3A · 3B · 4 · 5 · 6A · 6B | ⏸ NOT STARTED |
| 7A · 7B · 7C · 7D | ⏸ NOT STARTED |
| 8A · 8B · 8C · 9 · 10 · 11 | ⏸ NOT STARTED |
| 12 · 13 · 14 · 15 · 16 | ⏸ NOT STARTED |

---

## 9. Documentation defects found and fixed in this sweep

### 9.1 FIXED — 63 root-level `.md` files; 23 assert completion; none quarantined

**Severity: high.** This is the most dangerous documentation state in the repository, because it is
the *first thing an agent sees*. **23 root files assert a completion claim**, using a **retired phase
numbering scheme**, while governance says Phase 2A is re-opened and 3A–16 are unstarted. **Zero carry
a superseded banner.** (15 also carry `*_COMPLETE*`/`*_SUMMARY*` in their filenames.)

Worst offenders, all unlabelled:
- `PROJECT_COMPLETE.md` — "🎉 ZylCode Enhancement Project: COMPLETE"
- `PHASE4_COMPLETE.md` — "All Phase 4 Requirements Met"
- `PHASE3_COMPLETE.md` — "Core Systems Implemented"
- `IMPLEMENTATION_STATUS.md` — "Phase 1: COMPLETE ✅", "Enhanced MCP Bridge (100+ Tools)"
- `strategic-plan.md` — market-leadership claims (a *different file* from the already-quarantined `docs/STRATEGIC_PLAN.md`)

**Fix applied:** created `docs/README_INDEX.md` — a governing map that states the mandatory reading
order, classifies all 63 root files, enumerates the quarantined set, and restates the phase-numbering
hazard. The files themselves are **not** rewritten or deleted (Protocol §4.8 — they are provenance).

### 9.2 FOUND — 8 broken internal documentation references

| Missing path | Referenced from |
|---|---|
| `docs/brand/BRAND.md` | brand work order (not yet created — expected) |
| `docs/brand/README.md` | brand work order (not yet created — expected) |
| `docs/design/DESIGN_SYSTEM_SPEC.md` | brand work order (not yet created — expected) |
| `docs/research/PENPOT_TO_VISION_STUDIO_LESSONS.md` | `DEEPSEEK_PUBLIC_FOUNDATION_PROMPT_V1.md:215` — **genuine gap**, no such track item |
| `docs/architecture/WEB_CLOUD_ARCHITECTURE.md` | `DEEPSEEK_PUBLIC_FOUNDATION_PROMPT_V1.md:237` — **genuine gap**, referenced by `CLOUD-PLATFORM` track but never authored |
| `docs/ZYLCODE_CONSTITUTION.md` | `PHASE2A_INDEPENDENT_AUDIT.md` — *historical narrative*, correctly absent |
| `docs/ZYLCODE_ARCHITECTURE_V2.md` | `PHASE2A_INDEPENDENT_AUDIT.md` — *historical narrative*, correctly absent |
| `docs/ZYLCODE_ROADMAP_V2.md` | `PHASE2A_INDEPENDENT_AUDIT.md` — *historical narrative*, correctly absent |

The last three are **not defects**: they are the audit describing files it found stale, which have
since been superseded and removed. The first two are **real gaps** in the public-foundation prompt.

### 9.3 FOUND — `docs/evidence/` does not exist but is referenced by the website IA

`WEBSITE_INFORMATION_ARCHITECTURE.md` §7 makes `/evidence/` a first-class site destination. The repo
path backing it (`docs/evidence/`) has not been created. Not a defect in the IA — it is correctly
listed there as an unclosed gap — but it is a **prerequisite for PF-09**.

### 9.4 FOUND — status documents drift from the code

`ARCHITECTURE_V2.md` §9 and `README.md` agree with each other and with the roadmap. The **capability
registry** still does not: 23 of 34 entries carry `rung: null`, so the registry cannot substantiate a
claim even though its GREEN count was corrected. Audit P1-6 is half-done.

---

## 10. Untracked source files — new since the audit

The audit recorded 53 modified files. There are now **94** status entries — **56 under `crates/`+
`apps/`** — including **untracked source** that is not in any prior report:

| File | Lines | Assessment |
|---|---|---|
| `crates/zylcode-mcp/src/performance.rs` | 409 | Declared in `lib.rs` — compiles as part of the crate |
| `crates/zylcode-mcp/src/system_integration.rs` | 391 | Declared in `lib.rs` — compiles as part of the crate |
| `crates/zylcode-core/tests/commissioning_test.rs` | — | **`#[tokio::test]` attempting REAL provider inference** (Ollama or fail) |

**Note on `commissioning_test.rs`:** it is an integration test that tries to reach a real provider.
`ci.yml:77` runs `--all-targets`, which **includes integration tests**, so if this file were committed
unmodified it would attempt a network call in CI. It is currently untracked, so CI is unaffected — but
committing it as-is would introduce a network-dependent test into the pipeline. It needs an explicit
skip/ignore guard before it is tracked.

**These 800 lines of undeclared-to-review MCP code are exactly the "dirty, undocumented state" the
audit flagged (P1-9).** The tree has moved further from reviewable, not closer.

---

## 11. Answer to "what is left to become green"

In dependency order, shortest honest path:

1. **Fix `router.rs:1030`** — restore agreement between the synthetic-response contract and its test.
2. **Resolve the 94-entry dirty tree** — review, commit, or discard. Including the 800 lines of
   untracked MCP source and the network-dependent commissioning test.
3. **Obtain one clean workspace build outside the sandbox** — BUILD is currently unobservable here.
4. **Phase 2A P0-1** — normalise `should_exclude()`, adopt the `ignore` crate, test against the real `.gitignore`.
5. **Phase 2A P0-2** — re-baseline every metric against a correctly-scoped index (~278 files, not 24,939).
6. **Phase 2A P0-3/P0-4** — deterministic benchmark with asserted numbers and a machine-independent budget.
7. **Phase 2A P0-5** — commit a replayable acceptance transcript.
8. **Phase 2A P1-6** — integrate into `AgentLoop` **or** assign rungs to all 34 registry entries and downgrade.
9. **Phase 2A P1-7** — raise thresholds to `Precision@10 ≥ 0.60`.
10. **Independent re-audit (G3)** — then Phase 2A reaches R3 and 2B unblocks.

**Items 1–3 are the immediate blockers and are small.** Items 4–10 are the Phase 2A remediation,
which has not been started.

---

## 12. Discipline notes for the reader

- **GREEN is not achieved by passing tests.** Unit tests are R2. R3 requires a named, reachable
  product surface with captured evidence. No repository-intelligence surface exists.
- **The workspace build is NOT verified.** Sandbox prevented it. Reported as unobserved, not as pass.
- **`REMOTE_CI = BLOCKED_EXTERNAL`** per the foundation plan; the local red test is independent of that.
- **Where this sweep disagrees with `ARCHITECTURE_V2.md` §9, that document wins** — it is the
  authoritative rung table, and this sweep did not find it wrong.

---

## 13. Rung and status boards — unchanged

The sweep did not find `ARCHITECTURE_V2.md` §9 or the roadmap §2 status board wrong. Consult them
directly; they remain authoritative. Highest rung in the repository: **R3**. Nothing is R4 or R5.

---

## 14. UPDATE 2026-09-17 — the failing test is fixed, and the defect beneath it

### 14.1 What was fixed

The `router.rs:1030` failure reported in §4 is **resolved**. The fix was **not** a cosmetic assertion
swap; investigation revealed the test was not testing what it claimed.

**Root cause, corrected.** The original diagnosis in §4 ("a stale contract assertion") was right but
**incomplete**. The deeper defect is that the test was **reading a persisted SQLite vector cache**:

`TokenRouter::new()` calls `VectorCacheStore::with_default_path()`, which resolves to
`$ZYLCODE_VECTOR_CACHE_PATH` → `~/.zylcode/vector_cache.db` → **`./vector_cache.db`** relative to the
process working directory. A pre-existing `vector_cache.db` (untracked, 20 KB, dated Sep 15) was
present in the repo root. The test's prompt therefore returned a **cached response from a previous
run**, and the code path the test claims to exercise was never reached.

**This was proven by falsification.** A deliberate corruption was injected into
`synthetic_response()` (appending `"NOT JSON"`), which must break the contract. The test **still
passed** — the corrupt function was never called, because the cache answered first. Re-running with
`ZYLCODE_VECTOR_CACHE_PATH` pointed at a fresh path caused the same corruption to **fail** correctly.
A test whose result depends on untracked local filesystem state is not evidence.

### 14.2 The fix — two parts

**Part 1 — a hermetic constructor.** Added `TokenRouter::without_vector_cache(config)`, which performs
no filesystem access. `new()` and `with_metrics()` are unchanged for production callers; the new
constructor exists because ambient cache state must not be reachable from a test.

**Part 2 — the test now asserts the real contract.** It previously substring-matched
`"<zylcode-response>"`, an **XML envelope that is no longer the protocol**. The live contract is
`AgentDecision` JSON — `agent.rs:1447` instructs the model to *"respond with valid JSON matching the
AgentDecision protocol"*, and `pipeline.rs:159` consumes `dispatch_prompt` output. The test now
**parses** the payload into `AgentDecision` and asserts the `Complete` variant. A parse is strictly
stronger than a substring match: a substring match cannot catch a payload the consumer would reject.

### 14.3 Falsification evidence (A11 — assert effect, not success)

| Step | Condition | Expected | Observed |
|---|---|---|---|
| 1 | Clean contract, stale cache present | PASS | ✅ `1 passed` |
| 2 | **Corrupted** contract, stale cache present | **FAIL** | ✅ `synthetic response is not a valid AgentDecision: trailing characters at line 1 column 222` |
| 3 | Clean contract, full suite | PASS | ✅ `226 passed; 0 failed` + `35 passed; 0 failed` |

Step 2 is the one that matters. Before the fix, the same corruption **passed silently**. After it, the
test fails as it must.

### 14.4 Suite status

```
running 226 tests
test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.20s
running 35 tests
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.33s
```

**Blocker #1 from §11 is cleared.** The committed tree is green.

### 14.5 What this changes for Phase 2A

**Nothing yet — and it is important not to read this as progress on the phase.** The fix restores a
*working baseline*, which is a precondition for the re-audit, not a remediation item. The audit's
findings stand unchanged: **0 of 10 remediation items complete**, P0-1 (`should_exclude()`
normalisation + the `ignore` crate) still unimplemented, and the Intelligence Graph still has zero
product integration. Phase 2A remains at **R2**.

Three items from §11 remain open before a re-audit is possible:

1. ~~Fix `router.rs:1030`~~ — **DONE** (§14).
2. **Resolve the dirty tree** — still open, and still growing.
3. **One clean `cargo build --workspace` outside the sandbox** — still open. This sweep could not
   observe it.

### 14.6 A generalisable rule this uncovered

> **A test that reads persisted local state is not a test.**

Three specific hazards were found in the same file, all worth a lint or a review checklist:

1. **Ambient filesystem dependency.** `TokenRouter::new()` resolves a cache path relative to the
   process working directory. Any test using it inherits whatever is on disk. Tests must construct
   hermetic instances, or the cache path must be injectable.
2. **A test that cannot fail.** `crates/zylcode-core/tests/commissioning_test.rs` reaches a real
   provider (Ollama) and then **returns `Ok(())` on every path**, including failure — it prints emoji
   status and asserts nothing. Under `ci.yml:77` (`cargo test --workspace --all-targets`) it would
   pass unconditionally and green-light nothing. It is a **commissioning script**, not a test, and its
   name overstates it. It is currently untracked; **do not commit it as a test target** without
   converting it into either (a) a `#[ignore]`-gated test that asserts real outcomes, or (b) a script
   outside `tests/`.
3. **Two different "protocols" share one tag name — a genuine trap, but not a defect.** The
   `<zylcode-response>` envelope is **still live and correct** for *artifact* parsing:
   `ArtifactPipeline::parse_artifacts` (pipeline.rs:286) scans `<artifact>` tags inside that envelope
   with a `memchr` scanner, the fallback to bare markdown fences is intentional, and the format is
   covered by a passing test (pipeline.rs:648). The benchmarks in `benches/parse.rs` and
   `benches/router_cache.rs` are therefore **valid** — they measure live code.

   The defect was narrower and is now fixed: the **router's** `synthetic_response()` had been migrated
   to `AgentDecision` **JSON** while its test still asserted the **XML** envelope. So the same tag
   appeared to mean "the wire format" in one module and "a retired format" in another. **Do not
   "clean up" the XML envelope in `pipeline.rs` or the benchmarks** — that format is current for
   artifacts. The two are unrelated contracts that happened to collide on a tag name.

---

## 15. Dirty-tree audit (2026-09-17) — blocker #2, characterised

HEAD at audit: `8f751edbf89d11ad2d7daa15c558379e23ba8809` (identical to `origin/main`).
This section answers *what the ~88 dirty entries actually are*, so the owner can decide
commit / discard / quarantine per group. It does not resolve them.

### 15.1 Composition

| Measure | Value |
|---|---|
| Total dirty entries | **88** (55 `M` under `crates/`+`apps/`, 33 `??`) |
| Modified tracked source files | **52** |
| Raw delta (`git diff --shortstat crates/ apps/`) | **4157 insertions(+), 1545 deletions(-)** |
| Whitespace-insensitive delta (`git diff -w`) | **3487 insertions(+), 875 deletions(-)** |
| **Reformatting-only inflation** | **≈ 670 lines (16% of insertions) are pure formatting churn** |
| Untracked source files | 3 (`commissioning_test.rs`, `performance.rs`, `system_integration.rs`) |

Contributing factor: **`core.autocrlf=true` and no `.gitattributes`.** Every touched file emits
`LF will be replaced by CRLF`, so a one-line edit inflates the diff. The 16% churn figure is a
*floor* — it counts only lines that vanish entirely under `-w`.

### 15.2 The 52 modified files fall into four workload classes

| Class | What it is | Evidence | Roughly |
|---|---|---|---|
| **A. Substantive new implementation** | New modules & tool catalogs | `enhanced_bridge.rs` +776 (whitespace-insensitive), `hot_reload.rs` +186, `real_tools.rs` +74 (`RiskLevel`, `ToolSchema`), `builtin_skills.rs` +787/−158 | ~8 files |
| **B. New tests** | `telemetry_audit_tests.rs` +158, `integration_test.rs` +120, plus assertions in `fault_injection.rs` | real `log_tool_invoke` / `log_retry` / `log_tool_result` calls | ~4 files |
| **C. `impl Default` additions** | **17 added, 0 removed** — clippy `new_without_default` remediation | traced to untracked `fix_clippy_defaults.ps1` | ~15 files |
| **D. Pure formatting/import-order churn** | `use` reordering, one-statement-per-line expansion | visible in every sampled file | ~25 files |

**Class C is now redundant.** `cargo clippy --workspace --all-targets --offline -- -D warnings`
**passes cleanly on the current tree**. Whatever the script chased, the tree is already
clippy-clean with those impls in place — but the impls are mechanically generated and were never
reviewed as a unit.

### 15.3 What is genuinely good here — stated plainly

This is not scratch state. The `enhanced_bridge.rs` tool catalog is real, compiling,
clippy-clean code, and the new telemetry tests exercise real logger APIs. **The Computer-Use
Engine was NOT extended** — `computer_use/` changes are **formatting-only** (`-w` diff shows
only `use` reordering and `.await` line-wrapping; the 31 simulation markers and the five
fabricated confidences at `vision_ai.rs:110,123,136,158,170` are **untouched**). That is the
correct outcome given `docs/architecture/COMPUTER_USE_ENGINE.md` says *replace, do not extend*.

```
$ cargo check --workspace --all-targets --offline
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 23s     # ✅ compiles with the untracked modules
$ cargo clippy --workspace --all-targets --offline -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.49s     # ✅ zero warnings
```

### 15.4 Finding F — a retracted claim has re-entered the repository

**This is the highest-value result of the dirty-tree audit.**

`docs/PHASE1C_COMPLETION_REPORT.md:59` records the claim as **already removed**:

> Removed unsupported claims (SOC 2, ISO 27001, `<100ms`, **156 tools**)

The claim is **back**, in **eight** files, and it is **still false**:

| File | Tracked? |
|---|---|
| `DEVELOPER_GUIDE.md` | untracked |
| `PHASE3_COMPLETE.md` | untracked |
| `PHASE3_SUMMARY.md` | untracked |
| `phase4-implementation-plan.md` | untracked |
| `PHASE4_SUMMARY.md` | untracked |
| `USER_GUIDE.md` | untracked |
| **`FINAL_SUMMARY.md`** | **TRACKED** (commit `29cc936`) |
| `docs/PHASE1C_COMPLETION_REPORT.md` | tracked — but only as the *removal record* |

**Ground truth, measured:**

```
$ grep -c 'ToolDefinition {' crates/zylcode-mcp/src/enhanced_bridge.rs        → 113   (HEAD: 28)
$ grep -o 'id: "[a-z0-9._-]*"' ... | sort -u | wc -l                          → 112
$ grep -c 'fn get_.*ToolCategory' ...                                          → 17
$ ls categories referenced in EnhancedMcpBridge::new()                         → 16
```

**112 distinct tool IDs across 16 categories.** Not 156. The number was never true and was
explicitly retracted, and it has now propagated into a **tracked** document (`FINAL_SUMMARY.md`)
and into an untracked developer guide (574 lines) that reads as authoritative onboarding material.

`FINAL_SUMMARY.md` **is** covered by the `README_INDEX.md` quarantine table, which limits the
damage — but only for a reader who finds the index first. The untracked carriers have no such
protection at all: `DEVELOPER_GUIDE.md` and `USER_GUIDE.md` are currently invisible to git,
undeclared by any index, and contain the exact claim this project spent a phase removing.

> **Rule this finding establishes.** A claim that has been *retracted* is more dangerous on
> re-entry than a claim that was merely never made — because reviewers who remember the
> retraction assume it stuck. Retractions must be enforced by a **grep-based check in CI**, not
> by a line in a completion report.

**A complication found while correcting it — `FINAL_SUMMARY.md` carries foreign work.** HEAD's
version of this file is a *"Complete Project Summary & Next Steps"* (research findings, market
sizing, a 24-week roadmap, pricing tiers). The working tree has replaced it **wholesale** with a
different document, *"ZylCode Desktop App — Fixed and Running"*. The `156 tools` claim appears in
**both** versions. My correction was applied to the working-tree version and **left uncommitted**:
committing it would have published another session's unreviewed on-disk rewrite under a governance
commit message. It is flagged here instead. The `156 tools` claim in the file remains uncorrected in
git until that rewrite is adjudicated.

### 15.5 Finding G — `DEEPSEEK_MASTER_PROMPT_V21.md` is clobbered (still open)

The tracked, committed 522-line master work order has been overwritten in the working tree with a
**156-line stale v1.0 copy of `ZYLCODE_COMMERCIAL_MODEL.md`** (469 deletions). It is missing the
**v1.1 correction** — the §4-vs-§6 gate contradiction the owner caught and had fixed.

```
$ wc -l docs/governance/DEEPSEEK_MASTER_PROMPT_V21.md                       → 156   (expect 522)
$ head -1 docs/governance/DEEPSEEK_MASTER_PROMPT_V21.md
# ZYLCODE_COMMERCIAL_MODEL.md — ZylCode Commercial Model
```

The true original is intact at HEAD (`git show HEAD:docs/governance/DEEPSEEK_MASTER_PROMPT_V21.md`
→ 522 lines). **Not reverted** — restoring it would silently destroy another session's in-flight
edit if that edit is deliberate. **This is an owner decision.** Note the file *also* appears to be
a mis-named artifact: a commercial-model draft living under the master-prompt filename.

### 15.6 Finding H — untracked governance-colliding reports

Sixteen untracked root `.md` files assert completion in the vocabulary this program reserved.
Worst offenders by collision risk:

| File | Lines | Problem |
|---|---|---|
| `PHASE3_COMPLETE.md` | 323 | "**Phase 3** Implementation Status: **COMPLETE**" — collides with governance Phase 3A–3B, which are **NOT STARTED** |
| `PHASE4_COMPLETE.md` | 236 | "**Phase 4** Complete" — collides with governance Phase 4, **NOT STARTED** |
| `INSTALLATION_SUMMARY.md` | 329 | "All Tests Passed!" — no command, no environment, no raw output |
| `INSTALLATION_SUCCESS.md` | 321 | **UTF-16 encoded** (`ÿþ#` BOM) — mojibake for most tooling, undiffable |
| `PHASE4_PROGRESS.md` / `PHASE4_SUMMARY.md` | 224 / 208 | Report "IN PROGRESS" under a conflicting "🎉 …Progress" title |

Per Protocol §4.7 (phase numbers belong to the roadmap only) and §4.8 (superseded documents are
provenance, never evidence), **these cannot be committed as-is.** They are a local-session
vocabulary that predates the governance program.

Also untracked and unaddressed: `mcp.tools.yaml` (6.3 KB tool config — referenced by anything?),
`vector_cache.db.bak` (20 KB — **a stale vector-cache backup; the exact class of file that caused
the §14 defect**), and 5 PowerShell diagnosis/test scripts (`diagnose.ps1`, `simple-diagnose.ps1`,
`test-installation.ps1`, `test-simple.ps1`, `fix_clippy_defaults.ps1`) plus 4 `.bat` launchers.

### 15.7 Recommended disposition (owner decision — not executed)

| Group | Entries | Recommendation |
|---|---|---|
| Class A + B (implementation + tests) | ~12 files | **Review as a unit, then commit** under a named track — this is real work, but it is **not Phase 2A remediation** and must not be represented as progress on the ladder |
| Class C (`impl Default` ×17) | ~15 files | Commit with A/B **or** revert; do not leave half-applied |
| Class D (formatting churn) | ~25 files | Revert. No semantic content; inflates every future diff. Pairs with adding a **`.gitattributes`** |
| Untracked completion reports | 16 files | **Do not commit.** Move to a local `_archive/` or delete. They carry retracted claims (Finding F) and phase-number collisions (Finding H) |
| `vector_cache.db.bak` | 1 file | **Delete.** It is a stale artifact of the defect fixed in §14 |
| `commissioning_test.rs` | 1 file | **Do not commit as a test** — it asserts nothing and returns `Ok(())` on failure (§14.6.2) |
| `DEEPSEEK_MASTER_PROMPT_V21.md` | 1 file | **Owner decision** — restore from HEAD, or confirm the overwrite is intended |

### 15.8 Effect on the blocker list

Blocker #1 (`router.rs:1030`) — **closed**. Blocker #3 (clean workspace build outside the sandbox)
— **still open**; this audit confirms the tree *checks* and *clippies* cleanly, but the sandbox
still cannot *execute* the Tauri build script, so the build itself remains unobserved here.
Blocker #2 — **now characterised, still open**, and reduced from "88 opaque entries" to the table
in §15.7. Two latent defects were surfaced while doing it: **Findings F and G**.

---

## 16. Enforcement — the retracted-claim guard (2026-09-17)

Finding F established the rule: *a retracted claim is more dangerous on re-entry than a claim never
made.* A line in a completion report does not enforce that. §16 makes it mechanical.

### 16.1 What was built

| Artifact | Purpose |
|---|---|
| `scripts/check_retracted_claims.py` | The guard. Fails on any **new** occurrence of a retracted claim. |
| `scripts/retracted_claims_baseline.txt` | The **archived debt**: 50 pre-existing occurrences across 12 files. May only shrink. |
| `.github/workflows/ci.yml` → *Retracted-claim guard* | Runs the guard on every push and PR. |

Each rule carries three mandatory fields: the **claim**, the **retraction reference**
(`docs/PHASE1C_COMPLETION_REPORT.md:59`), and the **measurement that replaced it**
(`112 distinct tool IDs across 16 categories`). A rule without both is inadmissible.

### 16.2 Design decisions worth recording

1. **Baseline may only shrink.** An entry that stops reproducing is reported `STALE` and **fails
   the run**. This prevents the two failure modes that would otherwise appear: a backlog that
   silently regrows, and a backlog that is quietly abandoned. Deleting the documents is the only
   way to remove a line.
2. **Heuristic discrimination, not blanket matching.** Three false-positive classes were found and
   handled: text that *negates* the claim, text that *discusses* the retraction, and compliance
   terms describing a **third party** (Tabnine, JetBrains) or a **future objective** ("obtain SOC 2
   certification"). Numeric claims like `156 tools` are never third-party, so they have no excuse
   path — every hit is a defect.
3. **The guard excludes itself.** `SELF_PATHS` keeps the guard and its baseline out of the scan;
   their entire purpose is to name the claims.

### 16.3 Falsification evidence (the guard can fail)

A check that cannot fail is not a check. Three tests were run against synthetic repositories:

| Test | Input | Expected | Observed |
|---|---|---|---|
| **A** | clean document + correct baseline | exit 0 | ✅ `OK: no NEW retracted claims` — exit 0 |
| **B** | new file asserting `156 tools` + `SOC 2` + `ISO 27001` | exit 1, all three named | ✅ 3 new occurrences reported — exit 1 |
| **C** | baseline containing a non-existent entry | exit 1, reported STALE | ✅ `STALE: 156 tools\|NONEXISTENT_FILE.md\|999` — exit 1 |

Also verified: `python -m py_compile` (syntax), and the CI workflow parses as valid YAML with the
guard step present as step 11 of 12.

### 16.4 Current state of the backlog

```
$ python scripts/check_retracted_claims.py
OK: no NEW retracted claims (50 pre-existing baselined occurrence(s), 50 baseline line(s)).

$ grep -v '^#' scripts/retracted_claims_baseline.txt | cut -d'|' -f1 | sort | uniq -c
     26 156 tools
     16 SOC 2
      8 ISO 27001
```

**12 files** carry the backlog. Eight of them are untracked legacy reports already quarantined by
`README_INDEX.md`. Two (`competitive-analysis.md`, `research-findings.md`) are competitor profiles
and roadmap items — legitimate content the heuristics cannot fully separate; they should be
**allowlisted** rather than baselined, which will remove ~9 lines. One (`FINAL_SUMMARY.md`) is
**tracked**, and its baseline line numbers will churn the moment its unreviewed on-disk rewrite is
adjudicated (Finding F).

**The backlog is now bounded.** It cannot grow, and it cannot be forgotten.

---

## 17. Finding I — `origin/main` DOES NOT COMPILE (2026-09-17)

**Severity: highest in this document.** Discovered during parallel-execution preflight
(`PARALLEL_EXECUTION_MANIFEST.md` §9). This finding supersedes §3 and corrects §11 item 3.

### 17.1 What was observed

A **clean worktree** at `1338d0b` (no uncommitted changes), built with network access so the
failure cannot be a registry-fetch issue:

```
$ git worktree add .wt/probe HEAD --detach && cd .wt/probe && cargo check --lib
error[E0583]: file not found for module `performance`
error[E0583]: file not found for module `system_integration`
error[E0599]: no method named `get_more_development_tools`  found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_ai_ml_tools`        found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_database_tools`     found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_cloud_tools`        found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_devops_tools`       found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_communication_tools` found for `&EnhancedMcpBridge`
error[E0599]: no method named `get_more_productivity_tools`  found for `&EnhancedMcpBridge`
error: could not compile `zylcode-mcp` (lib) due to 10 previous errors
```

### 17.2 Two independent defects, both committed

**Defect 1 — declared modules with no source file.**

```
$ git show HEAD:crates/zylcode-mcp/src/lib.rs | grep -c 'pub mod performance;'         → 1
$ git cat-file -e HEAD:crates/zylcode-mcp/src/performance.rs                          → MISSING
$ git show HEAD:crates/zylcode-mcp/src/lib.rs | grep -c 'pub mod system_integration;' → 1
$ git cat-file -e HEAD:crates/zylcode-mcp/src/system_integration.rs                   → MISSING
```

`lib.rs` **at HEAD** declares both. Neither file is in git; both exist **only as untracked**
working-tree files (409 + 391 lines).

**Defect 2 — a half-committed refactor.**

```
$ git show HEAD:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -c 'fn get_more_'  → 0
$ git show HEAD:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -n  'get_more_'    → lines 87–94
$ git show HEAD:crates/zylcode-mcp/src/enhanced_bridge.rs | wc -l                   → 743
$ wc -l < crates/zylcode-mcp/src/enhanced_bridge.rs                                 → 1518
```

The committed `new()` **calls** eight methods the committed file **does not define**. They exist
only in the dirty tree (+775 lines).

### 17.3 Why §15 missed it, and why that matters

§15 audited the **working tree** — which compiles, because it carries the uncommitted work. The
correct check is a **clean checkout**, which §15 did not perform. The lesson is generalisable and
belongs beside the §14.6 rule:

> **A working tree that compiles is not evidence that the repository compiles.**
> Only a clean checkout at a recorded SHA can establish that. When reviewing a dirty tree, the dirty
> part must be removed from the question, not measured alongside it.

This is the same class of error as §14's *"a test that reads persisted local state is not a test"*:
**the ambient state made a real defect invisible.**

### 17.4 Consequence — the dirty tree is load-bearing

Any instruction to "discard the dirty tree" would now **destroy two modules and 775 lines of
implementations**, leaving `main` permanently unbuildable. The dirty tree and the repository are
fused until FIX-1/FIX-2 land. `§15.7`'s recommendation to revert "Class D formatting churn" remains
valid; its recommendation to delete untracked files must now **exclude anything the build depends
on**.

### 17.5 Corrections to earlier sections

| Section | Was | Now |
|---|---|---|
| §3 | `cargo build` "BLOCKED BY SANDBOX — not a code failure" | **Partly wrong.** See §17.6. The sandbox does not block worktree builds. |
| §11 item 2 | "Resolve the dirty tree — review, commit, or discard" | **Discard is now unsafe.** Commit is mandatory for the build-critical subset. |
| §11 item 3 | "One clean workspace build outside the sandbox" | **Re-scoped.** The build is achievable *inside*; it fails for a code reason, not a sandbox reason. |
| §15.3 | "The tree is *coherent*" | **True of the tree, false of the repository.** The tree is coherent only because it is dirty. |

### 17.6 Finding J — the worktree blocker was misdiagnosed

§11 item 3 recorded `git worktree`-based builds as impossible. **Falsified.** Two real causes,
neither a sandbox restriction:

1. **Path translation.** `git worktree add /tmp/wt_probe` created `C:/tmp/wt_probe` — outside the
   project's writable tree. Use a path **inside** the project (`.wt/<name>`).
2. **`--offline` with an empty registry.** `error: failed to download clap_derive v4.6.7 … --offline
   was specified`. A worktree has no `target/` and no fetched registry; the first build needs
   network.

Proof:

```
$ git worktree add .wt/probe HEAD --detach
HEAD is now at 1338d0b ci(governance): retracted-claim guard …
$ ls .wt/probe/Cargo.toml
.wt/probe/Cargo.toml                                  # ✅ materialised inside the tree
$ cd .wt/probe && cargo check --lib                   # ✅ compiled — and surfaced Finding I
```

**Consequence:** parallel isolated worktrees are viable, which is what makes
`PARALLEL_EXECUTION_MANIFEST.md` LAW 1 satisfiable at all. `.wt/` has been added to `.gitignore`.

### 17.7 CI status — NOT OBSERVABLE

`.github/workflows/ci.yml:69` runs `cargo check --workspace --all-targets` on a clean checkout, so
CI **must** be failing. This is an **inference from the compile result**, not a reading of a
dashboard: `gh` authentication fails (`GITHUB_TOKEN` invalid; the keyring account returns HTTP 401),
so runs cannot be queried. Recorded as `NOT OBSERVABLE`, never as green.

### 17.8 Required repair — owner decision

| Step | Action |
|---|---|
| **FIX-1** | Commit `crates/zylcode-mcp/src/performance.rs` (409 lines) and `system_integration.rs` (391 lines) |
| **FIX-2** | Commit the eight `get_more_*_tools()` implementations in `enhanced_bridge.rs` (+775 lines) |
| **FIX-3** | Verify a **fresh worktree** passes `cargo check --workspace --all-targets` |
| **FIX-4** | Confirm CI green (requires a valid token — §17.7) |

Both fixes commit **another session's unreviewed work** — the same restraint exercised in `6c19e1f`
for `FINAL_SUMMARY.md`. **Not performed unilaterally.** Until they land, no parallel agent may be
spawned: every worktree would inherit a broken base.
