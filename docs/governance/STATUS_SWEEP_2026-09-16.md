# ZylCode Status Sweep — Full Repository, 2026-09-16

**Type:** independent observation sweep (not a completion report)
**Method:** direct repository inspection + build/test execution
**Head at sweep:** `1faf90944a132fcb008c9a1dcad19f85286c3628` (identical to `origin/main`)
**Verdict:** **NOT GREEN.** One committed test failure and one unstarted remediation block the ladder.

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
