# ZylCode product introduction film — repository audit

**Audited:** 2026-10-31 · **Auditor:** NEXUS (agent), Zylvex Technologies Ltd
**Purpose:** establish the source of truth the film must be built on, before any footage is
recorded or a word of narration is written.

Rule applied throughout: **the repository is authoritative. Older documentation does not
override newer implementation evidence.** Where a document and the running code disagree,
both are recorded and the disagreement is named rather than resolved in the film's favour.

---

## 1. Repository state

| Field | Value | Source |
|---|---|---|
| Repository | `https://github.com/zylvex-tech/zylcode` | remote `origin` |
| Local checkout | `C:\Projects\zylcode` | `git rev-parse --show-toplevel` |
| Branch | `main` (local fast-forwarded to `origin/main`) | `git log -1 main` |
| HEAD | `d060b7c` — *Merge pull request #44 from zylvex-tech/feat/home-mission-pipeline* | `git log` |
| Working tree | clean; `git status --short` empty at audit time | `git status` |
| App version shown in product | **`v0.2.3`** (status bar) | `package.json`, `/api/version` |
| In-app milestone label | `v0.3.0-convergence` (header) — a **product-milestone string, not a release version** | product UI |
| `/api/version` response | `{"app_version":"0.2.3","branch":"…","dirty":false,"engine":"zylcode-core","head_commit":"063d1e6"}` (served from the pre-merge build; rebuilt after merge is required before final capture) | live endpoint |

### PR inventory (latest merged)

| PR | Title | Merged |
|---|---|---|
| #44 | Coverage floor of 80% per file, live-backend captures, Respan gateway | 2026-10-31 · **APPROVED by `Gar-gar22`**, merged by `zylvex-tech`, merge commit `d060b7c` |
| #43 | Benchmark green, frontend coverage gated at 70%, v0.2.3 release prep | 2026-10-29 |
| #41 / #39 | dependency bumps (user-event, taiki-e/install-action) | 2026-10-30 |
| #38 | animate the mission lifecycle on the home surface + roadmap visual | 2026-10-27 |
| #36 / #35 | terminal: clear on clear/cls, right shell, real workspace | 2026-10-26/27 |

Branch protection on `main`: 4 required status checks (`CI / check / test / frontend` on
ubuntu-22.04, windows-latest, macos-latest, and `Coverage / coverage`), `strict` up-to-date
required, **1 approving review required**, `enforce_admins: true`. PR #44 satisfied all of it:
5/5 checks green (5m / 4m / 8m / 5m / 1m).

---

## 2. Test and CI results (measured, not quoted)

| Gate | Result |
|---|---|
| Frontend suite | **67 files, 760 tests, all passing** |
| Frontend coverage | statements **97.2%**, branches **91.06%**, functions **96.54%**, lines **97.2%** — **zero files below 80%** on any metric |
| Coverage thresholds | 70 lines / 83 branches / 80 functions / 70 statements — pass, none lowered |
| Rust suite | `cargo test --workspace --all-targets` → **932 passed, 0 failed** |
| Rust lints | `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; `cargo check` clean |
| Frontend lints | `eslint` 0 problems; `tsc --noEmit` clean; `prettier --check` conformant |
| Repository guards | `check_retracted_claims.py` OK (11 pre-existing baselined); `check_hardcoded_secrets.py` clean |
| Build | `pnpm --filter zylcode-desktop build` exit 0 |
| N-ATLAS | typecheck clean; `smoke.mjs` **16/16 assertions** |
| CI on `main` | `CI` success, `Coverage` success (a subsequent `CI` run was in progress at audit time) |

---

## 3. Documents inspected

* `README.md` — positioning: *"engineering software platform … local-first and
  bring-your-own-key … early access, pre-release, free/invite-only, no pricing published
  during public development."*
* `docs/ARCHITECTURE.md` — workspace graph: `apps/zylcode-desktop` (Tauri 2 + React + Vite),
  `crates/zylcode-core` (pure Rust engine: router, cache, compression, pipeline),
  `zylcode-cli`, `zylcode-mcp`, `zylcode-nav`.
* `docs/capability-registry.json` — **v1.4.1-audited**, timestamped **2026-09-16**:
  **37 capabilities — 22 GREEN, 2 PARTIAL, 12 DISPUTED, 1 EXTERNAL_BLOCKER.**
  Its own note: four GREEN entries (`repository_query_api`, `context_retrieval`,
  `agent_context_gathering`, `benchmark_determinism`) are **builder-claimed R3 evidence
  pending independent acceptance**.
* `docs/PROJECT_MASTER_STATUS.md` — last updated **2026-10-23 @ `6469bae`** (8 days older
  than HEAD, so it is *not* the newest evidence for anything changed since):
  16 phases / 6 epochs; **1A Real Tool Runtime ✅ ACCEPTED**, **1B Agent Execution Loop ✅
  ACCEPTED**, **1C ✅ ACCEPTED WITH CONDITIONS**, **1D ✅ ACCEPTED WITH CONDITIONS**,
  **2A Repository Intelligence ❌ NOT ACCEPTED — RE-OPENED**, **2B BLOCKED on 2A**,
  **3A / 3B ⏸ NOT STARTED**. Stated gap: *"wire existing libraries into product surfaces,
  and get Phase 2A accepted"* — not more code.
* `docs/roadmap/ZYLCODE_ROADMAP_V2.md` — current roadmap (an older copy is marked
  `governance/superseded/`).
* Governance: `PHASE2A_REAUDIT_REQUEST_2026-10-24.md`, `POST_MERGE_AUDIT_2026-10-22.md`,
  `RELEASE_v0.2.1_READINESS_REPORT_2026-10-25.md`, plus the DataFactor findings/plan/register
  added by PR #44.

---

## 4. What the desktop UI genuinely exposes (browser preview, live backend)

Verified by driving the real application in a browser against a running
`zylcode serve-intel` on `127.0.0.1:17630`:

| Surface | State observed |
|---|---|
| Explorer | `AVAILABLE`, **672 files**, real tree |
| Terminal | `AVAILABLE · powershell · command runner · cwd persists per session`; real commands, real exit codes (verified: `git log --oneline -3`, `echo`) |
| Source control | `AVAILABLE · via intel service`, live branch and change list |
| Code Navigation (intel) | query surface present; navigation queries return engine results |
| Missions / composer | composer, autonomy selector, `RUN MISSION` affordance |
| Evidence ledger | honest `BLOCKED`/`start zylcode serve-intel …` state when no ledger exists |
| Providers | controlled state, no key material rendered |
| Themes, bottom dock, settings, menu bar, activity rail | functional |

**Not available in browser preview (must not be shown as working):** Tauri IPC paths
(`safeInvoke` short-circuits to `desktop required`), desktop-only panels, and any
model-driven mission that needs a configured provider key.

---

## 5. Agent-execution paths — verified before recording

| Path | Result |
|---|---|
| `POST /api/intent` (serve-intel, browser-preview parity for `process_intent`) | **FAILS on this machine**: `intent failed: The filename, directory name, or volume label syntax is incorrect. (os error 123)`. Serve-intel canonicalises the workspace to the Windows verbatim form `\\?\C:\Projects\zylcode` and hands it to the engine. **Disclosed as a current defect; must not be presented as working.** |
| **`zylcode mission`** (CLI, First Mission) | **VERIFIED COMPLETED**, exit 0 — see below. This is the film's hero demo. |
| `zylcode repo-context` | available (repository intelligence retrieval) |
| `zylcode best-of-n`, `package`, `build`, `deploy-status` | available (not yet recorded) |

### Hero demo — probe run (recorded evidence, 2026-10-31 11:27 UTC)

```
cargo run -p zylcode-cli -- mission \
  --spec crates/zylcode-core/tests/fixtures/first_mission_spec.md \
  --fixture crates/zylcode-core/tests/fixtures/first_mission_project \
  --workdir target/film-mission
```

Result: `status: COMPLETED`, exit 0 —
`proven: 2`, `derived: 5`, `hypotheses: 1`, `unknowns: 1`, `failures: 1`
(`failure recovered with evidence operation=run:after_mean_coverage`),
`integrity: ledger=true graph=true claims=true`, with an engineering record and a
reproducibility package written under `target/film-mission/.zylcode/first_mission/`.

**Why this task was chosen:** it is small, reversible, and self-contained — the fixture is
copied into its own workdir, so nothing in the production repository, no credential, and no
unrelated user file is touched. It exercises exactly the story the film has to tell:
specification → plan → real tool calls → a real failure → a real repair → verification →
evidence.

---

## 6. Capabilities the film may claim, and how they are supported

| Claim | Evidence |
|---|---|
| Runs locally, bring-your-own-key | `README.md`, `docs/PROVIDERS.md` |
| Real tool runtime with permission gating | Phase 1A **ACCEPTED** (`PROJECT_MASTER_STATUS.md` §2) |
| Structured agent execution loop | Phase 1B **ACCEPTED** |
| Durable memory, evidence ledger, recovery | Phase 1D **ACCEPTED WITH CONDITIONS**; mission probe output `integrity: ledger=true graph=true claims=true` |
| End-to-end evidence-first mission with real failure and repair | live probe run above (log retained in `target/film_mission_probe.log`) |
| Repository intelligence (files, symbols, packages, nav) | live index: 669 files, 2949 symbols, 8 packages; `zylcode repo-context` |
| Terminal executes real commands | live session, real exit codes |
| Test/verification gates in CI | 5 required checks green on PR #44 |
| Coverage discipline | 97.2 / 91.06 / 96.54 / 97.2, zero files < 80% |

## 7. Limitations that must be disclosed in the film

1. **Phase 2A (Repository Intelligence Foundation) is NOT ACCEPTED — RE-OPENED**, and 2B is
   blocked behind it. Navigation and index results are real, but independent acceptance of
   2A has not happened.
2. **12 of 37 registered capabilities are DISPUTED**, 2 PARTIAL, 1 EXTERNAL_BLOCKER; four
   GREEN entries are builder-claimed pending independent acceptance.
3. **Pre-release / early access**, invite-only waitlist, **no published pricing**.
4. **Bring-your-own-key**: no model is invoked in any recording; provider panels show
   controlled states, never credentials.
5. **Browser preview ≠ desktop runtime**: Tauri IPC paths are not exercised in this
   footage; the desktop app is a separate build.
6. **`POST /api/intent` currently fails on Windows** with `os error 123` (verbatim path
   handling) — an open defect, disclosed, not hidden.
7. **N-ATLAS is an integration workstream inside ZylCode**, not the whole product; its
   smoke test passes 16/16 locally, and no live external endpoint is demonstrated.
8. No NVIDIA (or other vendor) technology is claimed as deployed; no adoption figures,
   customers, partnerships, certifications or benchmarks beyond the ones measured above.

---

## 8. Pre-capture checklist

- [ ] Rebuild `dist` from `d060b7c` (or later) so `/api/version` reports the merged commit.
- [ ] Re-run `serve-intel` + `vite preview` and re-confirm `/healthz`, `terminal/exec`, `/api/version`.
- [ ] Re-run the hero mission in a **fresh** workdir during recording (state resumes if reused).
- [ ] Confirm no credential, token, key or private path appears in any frame (repo path
      `C:\Projects\zylcode` is the public checkout path and is retained deliberately).
- [ ] Record every scene before writing final narration (brief §6).
