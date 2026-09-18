# PARALLEL_EXECUTION_MANIFEST.md — Preflight & Coordination

**Program:** ZylCode Parallel Execution Master Work Order
**Coordinator:** architect/auditor session
**Preflight executed:** 2026-09-17, ~08:20–08:30 UTC+1
**Preflight re-verified:** 2026-09-17, ~17:00 UTC+1 (see §15 — four corrections)
**Status of this document:** **PREFLIGHT ONLY — no agent has been delegated.**

---

## 0. Verdict up front

> **Parallel execution is NOT authorized to begin yet.**
>
> A clean checkout of `origin/main` **does not compile**. Both the integration branch and the
> recorded "unobservable build" blocker need addressing first, and **neither is a Phase 2A item** —
> they are release-integrity defects that would make every parallel worktree start from a broken
> base.
>
> This is exactly the situation LAW 3 exists to catch: the dependency graph, not agent capacity,
> decides what may run.

---

## 1. Observed `origin/main` SHA

| Item | Value |
|---|---|
| `origin/main` (fetched this session) | **`1338d0bee3e676a2f5d8c678ba32f8e6ad41237b`** |
| Local `HEAD` | `1338d0bee3e676a2f5d8c678ba32f8e6ad41237b` (**identical**) |
| Working branch | `main` |

```
$ GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=echo git fetch origin
 * [new branch]      main       -> origin/main
$ git rev-parse origin/main
1338d0bee3e676a2f5d8c678ba32f8e6ad41237b
```

Local and remote agree. No divergence, no unpushed commits.

---

## 2. Dirty-tree state

**87 entries.** Composition unchanged in character from the sweep audit
(`docs/governance/STATUS_SWEEP_2026-09-16.md` §15).

| Measure | Value |
|---|---|
| Total entries | **87** |
| Modified (tracked) | 55 |
| Untracked | 32 |
| Source delta (`crates/`+`apps/`) | 52 files, 4157 / 1545 raw |
| Whitespace-insensitive | 3487 / 875 |
| Already audited | **Yes** — `STATUS_SWEEP` §15 (composition, per-group disposition §15.7) |

**No unknown work was discarded and no unknown work was absorbed.** Nothing was staged, committed,
or reverted during this preflight.

> ### ⚠ THE DIRTY TREE IS LOAD-BEARING — this is new
>
> The working tree is not merely *unreviewed*. It is **what makes the repository compile**.
> Discarding it — the obvious reading of "leave a clean tree" — would leave `origin/main` in a
> permanently broken state and destroy the only copy of two modules. See §9.

---

## 3. Worktree / branch plan

### 3.1 Worktrees ARE viable — the recorded blocker is WRONG

`docs/governance/STATUS_SWEEP_2026-09-16.md` §11 records: *"The sandbox blocks cargo writes outside
the primary `target/` directory, so `git worktree`-based clean-checkout builds fail."*

**This preflight falsified that finding.** Two independent causes, neither a sandbox restriction:

| Cause | Evidence | Fix |
|---|---|---|
| **Path translation** | `git worktree add /tmp/wt_probe` created `C:/tmp/wt_probe` — outside any writable/tracked tree | Use a worktree path **inside** the project (`.wt/<name>`) |
| **`--offline` with an empty registry** | `error: failed to download clap_derive v4.6.7 … --offline was specified` | Build the first time **with** network |

Proof — a worktree inside the project built without sandbox interference:

```
$ git worktree add .wt/probe HEAD --detach
Preparing worktree (detached HEAD 1338d0b)
HEAD is now at 1338d0b ci(governance): retracted-claim guard …
$ ls .wt/probe/Cargo.toml
.wt/probe/Cargo.toml                       # ✅ materialised
$ cd .wt/probe && cargo check --lib
… error: could not compile `zylcode-mcp` (lib) due to 10 previous errors   # ✅ COMPILED, and found §9
```

**Consequence:** LAW 1 is satisfiable. Parallel isolated worktrees are possible. Blocker #3 in the
sweep's blocker list is **misdiagnosed** and should be corrected to
`BLOCKED BY BROKEN HEAD — see §9`, not `BLOCKED BY SANDBOX`.

### 3.2 Proposed structure (once the base is fixed)

```
main  (integration only — never a working tree)
 ├─ .wt/phase2a        →  track/phase2a-remediation   AGENT A
 ├─ .wt/surface        →  track/product-surface       AGENT B
 ├─ .wt/public         →  track/public-foundation     AGENT C
 ├─ .wt/devlog         →  track/devlog-foundation     AGENT D
 ├─ .wt/benchmark      →  track/benchmark-foundation  AGENT E
 ├─ .wt/website        →  track/zylforge-website      AGENT F   (separate repo — see §7)
 ├─ .wt/brand          →  track/brand-penpot          AGENT G
 └─ .wt/audit          →  track/independent-audit     AGENT H   (read-only over candidates)
```

`.wt/` must be added to `.gitignore` before any worktree is created.

**Named tracks only. No phase numbers are invented for parallel work** (Law 3 /
Protocol §4.7). A, B, C, D, E, F, G are tracks; none is a "Phase N".

---

## 4. Agent allocation

| Agent | Workstream | Track | Base | Parallel-safe? |
|---|---|---|---|---|
| **A** | Phase 2A remediation | `phase2a-remediation` | needs fixed base | ⛔ **BLOCKED** (§9) |
| **B** | Product-surface forensic | `product-surface` | needs fixed base | ⛔ **BLOCKED** (§9) |
| **C** | Public Foundation / community | `public-foundation` | docs-only, own paths | ✅ **READY** |
| **D** | DevLog foundation | `devlog-foundation` | docs-only, own paths | ✅ **READY** |
| **E** | Public benchmark design | `benchmark-foundation` | docs-only, own paths | ✅ **READY** |
| **F** | ZylForge.com website | `zylforge-website` | **separate repo** | ✅ **READY** (§7) |
| **G** | Brand / Penpot | `brand-penpot` | needs Penpot probe | ⚠️ **CONDITIONAL** |
| **H** | Independent verifier | `independent-audit` | — | ✅ Reserved (§8) |

**Parallelism limit honoured.** I will not spawn C+D+E+F+G simultaneously. Per the work order's
priority order (A, F, C, B, G, then D, E) and because **A and B are blocked**, Wave A reduces to
**C, D, E, F** — all docs-scoped or separate-repo, zero source-file contention.

---

## 5. Dependency analysis

```
                    ┌──────────────────────────────────────────┐
                    │  HEAD 1338d0b  — DOES NOT COMPILE (§9)    │
                    └───────────────────┬──────────────────────┘
                                        │ must be repaired FIRST
                    ┌───────────────────▼──────────────────────┐
                    │  FIX-1: commit the 2 missing modules      │
                    │  FIX-2: commit the 8 missing methods      │
                    │  → gives a green, buildable base          │
                    └───────────────────┬──────────────────────┘
                                        │
        ┌───────────────────────────────┼──────────────────────────────┐
        │                               │                              │
   ┌────▼─────┐                  ┌──────▼──────┐                ┌──────▼──────┐
   │ AGENT A  │                  │  AGENT B    │                │  AGENT C/D/E│
   │ Phase 2A │                  │  Surface    │                │  Docs-only  │
   │ BLOCKED  │                  │  BLOCKED    │                │  READY      │
   └────┬─────┘                  └─────────────┘                └─────────────┘
        │ Phase 2A re-accepted (G3)
        ▼
   Phase 2B  —  may NOT begin (Law 3 / roadmap)

   AGENT F — independent repository, no dependency on ZylCode build state
   AGENT G — independent, gated on a real Penpot connection
   AGENT H — independent, gated on candidates existing
```

**Explicit non-starts, per Law 3:**

- Phase 2B — blocked until Phase 2A is re-accepted at G3.
- Phase 3A — not started. Free agent capacity is **not** a prerequisite satisfaction.
- Project System / Vision Studio / Computer-Use — no early implementation. Note the standing trap:
  `crates/zylcode-core/src/computer_use/` is a simulation facade at R0; **replace, do not extend.**

---

## 6. Path-ownership matrix

Paths are **exclusive**. No two agents may own the same file.

| Agent | Owns | Forbidden |
|---|---|---|
| **A** | `crates/zylcode-core/src/intelligence/*`, `crates/zylcode-core/tests/intelligence*`, `docs/capability-registry.json`, `docs/roadmap/` (Phase 2A sections) | everything else |
| **B** | `apps/zylcode-desktop/**` | `crates/**` (report findings to A instead) |
| **C** | `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`, `.github/ISSUE_TEMPLATE/**`, `.github/PULL_REQUEST_TEMPLATE*`, `docs/community/**` | `.github/workflows/ci.yml` (coordinator-owned), `crates/**` |
| **D** | `docs/devlog/**` | `docs/governance/**` |
| **E** | `benchmarks/**`, `docs/benchmark/**` | `docs/community/PUBLIC_CLAIM_MATRIX.yaml` |
| **F** | **separate repository** (§7) | this repository entirely |
| **G** | `docs/design/**`, `design/tokens/**` | `docs/governance/ZYLCODE_ARCHITECTURE_V2.md` |
| **H** | **read-only** — writes only `docs/audit/**` | all implementation paths |

**Contention found and resolved:** Agents A and B both plausibly want
`crates/zylcode-mcp/**` (A for the registry correction, B for IPC fixes). **B is forbidden from
`crates/`**; it reports IPC defects for A to fix. Those two workstreams are **serialized**, not
concurrent. This is the LAW 2 resolution.

**Coordinator-owned (no agent):** `.gitignore`, `.gitattributes`, `.github/workflows/**`,
`docs/governance/**`, `scripts/**`.

---

## 7. Tasks that can execute NOW

| Agent | Track | Why it is safe now |
|---|---|---|
| **F** | ZylForge.com website | **Separate repository.** Zero file contention with ZylCode. Highest-value parallel work available. |
| **C** | Public Foundation | Scoped to root community files + `docs/community/**`. No `crates/`, no build dependency. |
| **D** | DevLog foundation | Scoped to `docs/devlog/**`. Drafts only — **no publishing**. |
| **E** | Benchmark framework | Scoped to `benchmarks/**`. Design only — **no results, no rankings**. |
| **G** | Brand / Penpot | Conditional on a real connection probe (§8). Repository-side prep only if absent. |

**All four/five require one precondition:** `.wt/` in `.gitignore` and worktrees created from a
base that builds. C/D/E/F are docs-scoped so they tolerate the broken base — **but they must not
be merged while `main` is red**, or they will inherit blame for it.

---

## 8. Tasks that are BLOCKED

| Task | Blocked by | Unblocks when |
|---|---|---|
| **AGENT A — Phase 2A remediation** | `origin/main` does not compile (§9) | FIX-1 + FIX-2 committed and CI green |
| **AGENT B — product-surface forensic** | Same. Also cannot be verified without a running app. | Same + a runnable desktop build |
| **AGENT G — Penpot** | No verified Penpot connection. **Not probed; must not be assumed.** | A real connection is demonstrated |
| **AGENT H — verification** | No candidates exist | A builder produces a candidate commit |
| Phase 2B | Phase 2A not re-accepted at G3 | G3 audit passes |
| Blocker #3 (sweep list) | — | **Misdiagnosed**; see §3.1. Corrected to "blocked by broken HEAD", not by sandbox. |

**Phase 2A remediation status: 0 of 10 items complete.** P1.9 (`router.rs:1030`) is **RESOLVED**
(`8f751ed`). No remediation work may be reported as progress until A runs.

---

## 9. ⛔ BLOCKING DEFECT — `origin/main` does not compile

**This is the single most important finding of the preflight. It outranks the entire parallel
program.**

### 9.1 Evidence

A clean worktree at `1338d0b`, built with network access (so the failure is not a registry issue):

```
$ cargo check --lib
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

### 9.2 Two independent defects, both committed

**Defect 1 — declared modules with no source file.**

```
$ git show HEAD:crates/zylcode-mcp/src/lib.rs | grep -c 'pub mod performance;'          → 1
$ git cat-file -e HEAD:crates/zylcode-mcp/src/performance.rs                           → MISSING
$ git show HEAD:crates/zylcode-mcp/src/lib.rs | grep -c 'pub mod system_integration;'  → 1
$ git cat-file -e HEAD:crates/zylcode-mcp/src/system_integration.rs                    → MISSING
```

`lib.rs` **at HEAD** declares both modules. Neither file is in git. Both exist only as **untracked**
files in the working tree (409 and 391 lines).

**Defect 2 — a half-committed refactor.**

```
$ git show HEAD:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -c 'fn get_more_'   → 0
$ git show HEAD:crates/zylcode-mcp/src/enhanced_bridge.rs | grep -n  'get_more_'     → lines 87–94 (CALLS them)
$ wc -l < crates/zylcode-mcp/src/enhanced_bridge.rs                                  → 1518
$ git show HEAD:… | wc -l                                                            → 743
```

HEAD's file **calls** eight methods on itself that it does not define. The working tree defines
them (775 additional lines) but **has not been committed**.

### 9.3 Why this is worse than a normal red build

1. **It is invisible from the developer's working tree.** The tree compiles, so the person editing
   it sees green. Only a clean checkout reveals it — which is why the sweep's §15 audit reported
   the tree as "coherent".
2. **CI must be failing.** `.github/workflows/ci.yml:69` runs `cargo check --workspace --all-targets`
   on a clean checkout of `main`. **CI status could NOT be independently observed** (§10), so this
   is an *inference from the compile result*, not a reading of a CI dashboard. It is a high-
   confidence inference, but it is recorded as such.
3. **It silently fuses the dirty tree to the repository.** Any instruction to "discard the dirty
   tree" would destroy two modules and leave the project unbuildable. The two are now inseparable
   until FIX-1/FIX-2 land.
4. **Every parallel worktree would inherit it** — the exact failure Law 1 exists to prevent.

### 9.4 Required repair (FIX-1 / FIX-2) — sequenced, not parallel

| Step | Action | Owner |
|---|---|---|
| **FIX-1** | Commit `crates/zylcode-mcp/src/performance.rs` and `system_integration.rs` so declared modules exist | *owner decision* — currently untracked foreign work |
| **FIX-2** | Commit the `get_more_*` implementations in `enhanced_bridge.rs` (775 lines) | *owner decision* — same |
| **FIX-3** | Verify a clean worktree compiles: `cargo check --workspace --all-targets` | coordinator |
| **FIX-4** | Confirm CI green on the push (needs a valid token — §10) | coordinator |
| **FIX-5** | Only then: create Wave A worktrees | coordinator |

**FIX-1 and FIX-2 are not mine to make unilaterally.** They commit another session's unreviewed
work — the same restraint exercised in `6c19e1f` for `FINAL_SUMMARY.md`. **Owner decision required.**

Note the irony worth recording: the two "untracked, never reviewed" modules flagged in
`STATUS_SWEEP` §15.4 as *risk* are in fact **load-bearing** — their absence is what breaks `main`.

---

## 10. Owner actions required

**P0 — blocking the entire program**

1. **Authorize FIX-1 / FIX-2** (commit the two modules + the `get_more_*` methods), or state that
   the other session will commit them. **Nothing else can safely proceed until `main` builds.**
2. **Provide a valid GitHub token** so CI state is observable. *(`GITHUB_TOKEN` is invalid;
   the keyring account also returns HTTP 401. CI state is currently **NOT OBSERVABLE** — I will not
   report it as green.)*

**P1 — needed for Wave A**

3. **Confirm the ZylForge.com repository location/name.** Agent F must not contaminate this
   repository. If no separate repo exists, F cannot start.
4. **Confirm whether a Penpot connection exists.** If not, Agent G returns
   `PENPOT_CONNECTION_REQUIRED` and proceeds repository-side only.
5. **Adjudicate the earlier open items** still outstanding from `STATUS_SWEEP` §15.5/§15.6:
   - Finding G — restore `DEEPSEEK_MASTER_PROMPT_V21.md` from HEAD?
   - `FINAL_SUMMARY.md` — its unreviewed wholesale rewrite
   - The 16 untracked completion reports and the 12 retracted-claim carriers

**P2 — before parallel execution is genuinely safe**

6. **Resolve the dirty tree**, now with the added constraint that **it cannot be discarded** (§9.3).

---

## 11. Exact Wave A plan

**Wave A is CONDITIONAL on FIX-1…FIX-4.** If the owner authorizes the fixes, this executes in order.

### Wave A-0 — Base repair (serial, not parallel)

```
1. .gitignore += .wt/
2. FIX-1: commit performance.rs + system_integration.rs
3. FIX-2: commit enhanced_bridge.rs get_more_* implementations
4. Push → verify remote SHA
5. Independently build a fresh worktree → cargo check --workspace --all-targets MUST pass
6. Record base SHA  ── this becomes every agent's BASE
```

Gate: **no agent is spawned until step 5 passes in a clean worktree.**

### Wave A-1 — Parallel, 4 agents, zero file contention

| Order | Agent | Track | Owns | Deliverable |
|---|---|---|---|---|
| 1 | **F** | `zylforge-website` | separate repo | Homepage + 9 IA pages, staging evidence, media labelled by class |
| 2 | **C** | `public-foundation` | root community files, `docs/community/**` | CONTRIBUTING / CoC / SECURITY / SUPPORT / templates |
| 3 | **D** | `devlog-foundation` | `docs/devlog/**` | 4 article drafts with FACT / OBSERVED / HISTORICAL / CURRENT / INTENT separation |
| 4 | **E** | `benchmark-foundation` | `benchmarks/**`, `docs/benchmark/**` | Task definitions, anti-gaming controls, impl/results separation |

**G (Penpot)** joins Wave A-1 only if a connection is proven; otherwise it returns
`PENPOT_CONNECTION_REQUIRED`.

### Wave A-2 — After Wave A-1 integration

| Agent | Track | Gate |
|---|---|---|
| **A** | `phase2a-remediation` | Requires a green base. Now the critical path. |
| **B** | `product-surface` | Requires A's base, or at least a runnable desktop build. Serialized behind A on `crates/`. |

### Wave A-3 — Verification

| Agent | Role |
|---|---|
| **H** | Receives each candidate. Falsifies. Classifies ACCEPT / ACCEPT_WITH_CONDITIONS / REJECT / BLOCKED_EXTERNAL. **H builds nothing.** |

### Integration rule (unchanged)

One accepted workstream at a time. Refresh `main`, verify SHA, check candidate base, inspect
divergence, merge **one**, run regression, push, verify remote, inspect CI, update manifest. **A
rebase invalidates prior evidence** — re-verify, never carry it forward.

---

## 12. Live coordination table

Statuses: `QUEUED` `RUNNING` `BLOCKED` `CANDIDATE_READY` `AUDITING` `REJECTED` `ACCEPTED` `MERGED`

| Workstream | Agent | Base SHA | Branch | Status | Deps | Files owned | Tests | Commit | Audit | Merge |
|---|---|---|---|---|---|---|---|---|---|---|
| Base repair (FIX-1/2) | coord | `1338d0b` | `main` | **BLOCKED** — owner decision | — | `crates/zylcode-mcp/src/{performance,system_integration,enhanced_bridge}.rs` | `cargo check --workspace --all-targets` | — | — | — |
| Phase 2A remediation | A | — | `phase2a-remediation` | **BLOCKED** | base repair | `crates/zylcode-core/src/intelligence/**` | 10-item order | — | — | — |
| Product surface | B | — | `product-surface` | **BLOCKED** | base repair | `apps/zylcode-desktop/**` | surface map | — | — | — |
| Public foundation | C | `1338d0b` | `public-foundation` | **QUEUED** | — | community files | link/file check | — | — | — |
| DevLog foundation | D | `1338d0b` | `devlog-foundation` | **QUEUED** | — | `docs/devlog/**` | evidence citations | — | — | — |
| Benchmark foundation | E | `1338d0b` | `benchmark-foundation` | **QUEUED** | — | `benchmarks/**` | task schema | — | — | — |
| ZylForge website | F | — | `zylforge-website` | **BLOCKED** — repo unknown | owner confirm | separate repo | staging evidence | — | — | — |
| Brand / Penpot | G | `1338d0b` | `brand-penpot` | **BLOCKED** — no connection | Penpot probe | `docs/design/**` | token governance | — | — | — |
| Independent audit | H | — | `independent-audit` | **QUEUED** (idle) | candidates | `docs/audit/**` | falsification | — | — | — |

**No row may read `COMPLETE`.** `ACCEPTED` requires H. `MERGED` requires integration + remote
verification.

---

## 13. GitHub visibility snapshot

### T0 — before this program

| Item | Value |
|---|---|
| `origin/main` SHA | `1338d0bee3e676a2f5d8c678ba32f8e6ad41237b` |
| Local `HEAD` | `1338d0bee3e676a2f5d8c678ba32f8e6ad41237b` |
| Commit count on `main` | **68** |
| Remote branches | `main` only |
| Local branches | `main`, `origin/main` |
| Tags | `v0.1.0`, `v0.2.0`, `v0.2.0-dev` |
| Release state | Not observed (needs API) |
| README state | Tracked, present |
| Repository About | **NOT OBSERVABLE** — `gh` token invalid |
| Topics | **NOT OBSERVABLE** |
| Discussions | **NOT OBSERVABLE** |
| CI runs | **NOT OBSERVABLE** — HTTP 401 (invalid token) |

```
$ git rev-parse origin/main
1338d0bee3e676a2f5d8c678ba32f8e6ad41237b
$ git rev-list --count HEAD
68
$ git ls-remote --heads origin
1338d0bee3e676a2f5d8c678ba32f8e6ad41237b  refs/heads/main
```

> **`NOT OBSERVABLE` is not `green`.** Per the standing rule, an unobserved state is never converted
> into a pass. The work order's concern — *"'the agent says it pushed'"* must not be treated as
> *"'GitHub changed'"* — is satisfied by recording the SHA directly from `ls-remote`, and by
> recording what could **not** be read.

---

## 14. Program-level rules carried forward

1. **`main` is an integration branch.** No agent edits it directly.
2. **A phase number belongs to the roadmap and to nothing else** (Protocol §4.7). Parallel work uses
   named tracks.
3. **R2 ≠ R3.** 100 documentation commits do not equal one user-reachable capability.
4. **Builders do not certify their own work.** Agent H is preserved and untouched by builders.
5. **A rebase invalidates prior evidence.** Re-run, do not carry forward.
6. **Acting ≠ having acted.** An action is not successful until its effect is re-observed.
7. **`NOT OBSERVABLE` is never `green`.**
8. **An unlabelled skeleton is more dangerous than an absent feature.**
9. **Do not discard unknown work.** The dirty tree is now load-bearing (§9).
10. **Superseded documents are provenance, never evidence** (Protocol §4.8).

---

## 15. Preflight re-verification — four corrections (2026-09-17 ~17:00 UTC+1)

Re-verification before reporting surfaced **four corrections**. Three of them contradict assumptions
in the work order itself. Recorded here rather than silently reconciled.

### 15.1 CORRECTION — `origin/main` is `1338d0b`, but local `HEAD` has moved to `29a9654`

Verified:

```
$ git ls-remote origin refs/heads/main
1338d0bee3e676a2f5d8c678ba32f8e6ad41237b  refs/heads/main
$ git log --oneline origin/main..HEAD
29a9654 docs(governance): parallel preflight — Finding I: origin/main does not compile
```

| State | Value |
|---|---|
| `origin/main` (authoritative remote) | **`1338d0bee3e676a2f5d8c678ba32f8e6ad41237b`** |
| Local `HEAD` | **`29a9654`** |
| Unpushed commits | **1** |
| Branch | `main` |

`remote == 1338d0b` is the base every agent must start from. **`29a9654` is the preflight's own
documentation commit and is not on the remote** — it changes no source file, so it does not affect
what agents build from, but it does mean *the preflight record itself is unpublished*.

### 15.2 ⚠ CORRECTION — **AGENT F's repository is not what the work order assumes. It is already discovered and already gated.**

The work order instructs AGENT F to stand up **the ZylForge.com website** as *"a separate repo"*, and
lists *"Confirm the ZylForge.com repository location"* as an owner action. **That repository exists,
and so does the live site.** Verified:

```
$ /c/Projects/zylforge-mr                    → origin: github.com/zylvex-tech/zylforge-mr.git (main = 15c212a)
$ /c/Projects/zylforge-web-build             → a git worktree OF zylforge-mr
                                                gitdir: C:/Projects/zylforge-mr/.git/worktrees/zylforge-web-build
                                                branch: feat/zylforge-web-v1 (== main, 15c212a)
                                                origin: github.com/zylvex-tech/zylforge-mr.git
$ curl -sL https://zylforge.com → HTTP 200
  <title>ZylForge — The Engineering OS for Physical AI | Zylvex Technologies</title>
```

Three sub-findings:

1. **There is no `zylforge.com` repository.** The site work lives on branch `feat/zylforge-web-v1`
   **inside `zylforge-mr`**, checked out into a worktree directory named `zylforge-web-build` that is
   not (and cannot be) committed into the repository.
2. **`feat/zylforge-web-v1` carries zero commits beyond `main`** — it is at the same `15c212a` and is
   **not pushed** (`ls-remote` on `zylforge-mr` returns only `refs/heads/main`). So the branch name
   promises web work that the branch does not contain.
3. **`zylforge.com` is nevertheless live and serving a Zylvex-branded front page.** The source of the
   deployed site is therefore **not established**: it is neither on `zylforge-mr`'s remote branch
   list, nor identifiable from this environment's DNS/header response.

> **This falsifies the "must not contaminate ZylCode" premise as stated.** The correct premise is
> stronger and stranger: *the ZylForge web surface is a worktree of an unrelated repository
> (`zylforge-mr`) that already exists on the owner's disk, and whose deployment source cannot be
> derived from either repository's remote state.* Agent F as written would either duplicate existing
> work or overwrite it. **It must not be dispatched until the owner resolves 15.2(3).**

### 15.3 CORRECTION — **AGENT G's blocker is cleared: Penpot is reachable**

The work order instructs AGENT G to return `PENPOT_CONNECTION_REQUIRED` if Penpot is unreachable.
It is reachable:

```
$ curl -s -o /dev/null -w "%{http_code}" https://penpot.app        → 200
$ curl -s -o /dev/null -w "%{http_code}" https://design.penpot.app → 200
```

**Network reachability ≠ an authorized connection.** No Penpot account, token, or project is
configured in this environment, so the *API* path is still unverified. The distinction matters:
`PENPOT_CONNECTION_REQUIRED` is **not** warranted (the service is up), but **credentials are still
required** before G can commission anything. Reported as such rather than as either green or blocked.

### 15.4 CORRECTION — dirty-tree composition re-measured (`55 M` / `32 ??` = 87), defects confirmed at `HEAD`

Composition re-measured:

```
$ git status --porcelain | awk '{print $1}' | sort | uniq -c
     55 M      (tracked, modified)
     32 ??     (untracked)
```

Breakdown of the `crates/zylcode-mcp` defects, re-confirmed directly against `HEAD`:

| Claim | Verified |
|---|---|
| `performance.rs` / `system_integration.rs` tracked in git | **No** — `git ls-files` returns empty |
| Both files present on disk | **Yes** — 11,092 B and 12,652 B |
| `pub mod performance;` declared at `HEAD` | **Yes** — `lib.rs:10` |
| `pub mod system_integration;` declared at `HEAD` | **Yes** — `lib.rs:15` |
| `fn get_more_*` defined at `HEAD` | **0** |
| `fn get_more_*` defined in the working tree | **8** |

The 19 modified `zylcode-mcp` paths plus the 2 untracked modules are therefore **one indivisible
group**: the crate cannot build unless both committed declarations find their files. This is the
load-bearing core of §9 and the reason FIX-1/FIX-2 cannot be partially applied.

### 15.5 Effect on Wave A

| Agent | Prior status | **Corrected status** | Reason |
|---|---|---|---|
| A | BLOCKED | **BLOCKED** | base does not compile (Finding I) |
| B | BLOCKED | **BLOCKED** | needs a compiling base to observe product surface |
| C | executable | **executable** | docs-scoped, named track |
| D | executable | **executable** | docs-scoped, named track |
| E | executable | **executable** | docs-scoped, named track |
| F | executable | **BLOCKED — OWNER DECISION** | §15.2 — repo already exists, deployment source unresolved |
| G | conditional | **AWAITING CREDENTIALS** | §15.3 — service up, no authorized connection |
| H | reserved | **RESERVED** | builds nothing |

**Net effect: the executable Wave A set shrinks from four agents to three (C, D, E).** That is the
honest reading of the dependency graph. Adding capacity does not clear a prerequisite (LAW 3).

---

## 16. Repository integrity recovery — FIX-1…FIX-4 (2026-09-17)

Owner authorized P0 surgical recovery. **Status: COMMITTED `f1b2b36`, NOT PUSHED.**

**Four defects found, not two.** All the same mode: a commit landed the *caller* but withheld the
*callee*.

| ID | Defect | Root commit | Repair | In order? |
|---|---|---|---|---|
| FIX-1 | `lib.rs` declares `performance` / `system_integration`; files never committed | `5f5a99e` | remove 2 decls + 2 re-exports | yes |
| FIX-2 | 8 × `get_more_*_tools()` called, never defined | `29cc936` | remove 8 call sites | yes |
| **FIX-3** | `zylcode-core` needs `ToolSchema` / `RiskLevel` / `list_schemas` — absent | `158370f` | add 2 types + 1 method | **no** |
| **FIX-4** | `agent_loop_e2e.rs` passes 5 args to a 6-arg fn | same lineage | add `ledger` arg | **no** |

### 16.1 The rejection that matters (FIX-2)

Committing the 113 uncommitted tool definitions was **rejected on evidence.** Every tool routes to
`BuiltinTool::call()` — committed at `HEAD` — which sleeps 10 ms and returns `success: true`
without executing anything. Committing the definitions would have moved the tool count **28 → 141**
and registered 113 tools that report success while doing nothing. That is fabricated execution.
Repair option **C** (remove the call sites) was chosen; the bridge is also unreachable from the
desktop app, which uses `ZylCodeEngine` + `execute_with_recovery`.

### 16.2 New open contradiction (not repaired)

Committed `crates/zylcode-mcp/tests/integration_test.rs` asserts `tool_count >= 100` (actual **28**)
and `dev_tools >= 25` (actual **11**). **Neither can pass on any clean checkout** — only the
never-committed `get_more_*` block could satisfy them. Suppressing the assertions would be a builder
certifying its own work. **Owner product decision required.** This is the **third** documented form
of the `100+ tools` claim re-entering after retraction.

### 16.3 Verification (fresh clean worktree at the repaired SHA)

| Command | Result |
|---|---|
| `cargo check --workspace --all-targets` | **EXIT 0 — 0 errors** |
| `git diff --check` | **EXIT 0** |
| untracked required source | **none** |
| `cargo clippy --workspace --all-targets -- -D warnings` | **EXIT 1 — 31 pre-existing warnings** (none from this patch) |
| `cargo test --workspace --lib` | 215 pass / 11 fail — **all pre-existing** (10 `cli` subprocess tests needing an unbuilt bench binary; 1 non-hermetic `router` test hitting a live provider 401) |

Patch: **6 files, +379 / −25** (5 source files + the recovery report). The 338-line dirty
`real_tools.rs` refactor was **not** committed; only the 22 lines `zylcode-core` structurally
requires were extracted. The other 51 dirty paths are untouched.

### 16.4 Effect on the dependency graph

**Gate 0 is not yet cleared.** A clean checkout *compiles*, but `cargo test --workspace` does not
pass, and the patch is not on the remote. Until both hold:

- **AGENT A / B** remain **BLOCKED** — Phase 2A remediation must not start on an unaccepted base.
- **AGENT F** remains **BLOCKED — OWNER DECISION** (§15.2).
- **AGENT G** remains **AWAITING CREDENTIALS** (§15.3).
- Executable set is unchanged: **C, D, E**.

**`PUSHED = NO`.** `origin/main` is still `1338d0b` — i.e. **still broken**. The repair is not
remotely complete until the remote SHA contains it.
