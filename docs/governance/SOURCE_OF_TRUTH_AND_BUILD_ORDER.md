# SOURCE OF TRUTH & PRIORITY BUILD ORDER — 2026-09-18

**Executed by:** Buffy (Codebuff), owner work order: "establish source of truth for
everything, classify the build order based on priority, always commit and push."
**Status:** Integrated, verified, pushed, and remote-verified on `main`. Recorded below
with the commands that produced every figure. Nothing in this document is accepted by
audit; per LAW 3, builder results remain builder results until independently verified.

---

## 1. What the source of truth actually is

There were **five** parallel versions of the project. Measured this session:

| Version | Where | State before this session |
|---|---|---|
| `origin/main` | `1338d0b` | **Did not compile** (declared modules missing, 8 undefined methods) |
| Local `main` | `639dbbb` | Compiled only via an uncommitted 88-entry dirty tree; 3 unpushed docs commits |
| Remediation candidate | `.wt/remediation` @ `34d29a9` (25 commits, `f1b2b36..34d29a9`) | Fully tested (416/0, clippy clean); never merged |
| Recovery commit | `f1b2b36` (made in the worktree) | Not an ancestor of any branch on `main` |
| The dirty working tree | 56 modified + 38 untracked | Load-bearing: it was the only thing that compiled |

**As of this session, all five are reconciled into one:** `main` **is** the source of
truth, locally and on the remote. It contains the dirty tree (committed in coherent
groups), the remediation chain (merged; chain wins the 22 overlapping files), the
recovery commit (via the chain), and every doc commit.

`origin/main` is the authoritative remote source of truth. The integration tree first
reached the remote at `6e93c43d` (Gate-0 squash); subsequent governance/documentation
corrections followed. The current authoritative SHA is whatever
`git ls-remote origin refs/heads/main` returns — determine it at read time. A document
committed at SHA X cannot reliably name "the final repository SHA" inside its own
content, because naming it creates another commit; this document therefore records
**events** (first push, verifications) and never a current-state SHA.

Provenance is preserved: the pre-merge dirty-tree variants remain reachable in the
granular history (see §4, "Delivery form and provenance"); nothing was discarded, only
resolved.

### Verification battery, executed on the merged tree (this session)

| Check | Command | Result |
|---|---|---|
| Compile | `cargo check --workspace --all-targets` | **PASS** |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| Tests | `cargo test --workspace --all-targets --no-fail-fast` | **417 passed / 0 failed** |
| Claims guard | `python scripts/check_retracted_claims.py` | **OK** (11 baselined, 0 new) |

Caveat recorded per LAW 7: the earlier clean-worktree check ran from a worktree
carrying state; this session's battery ran on the primary tree. A third-party
reproduction on a fresh clone is the remaining step to R4 (P1.1 below).

---

## 2. Defects found and fixed during integration (beyond the plan)

1. **CI was red at HEAD before any push.** The old guard scanned the filesystem; the
   baselined legacy reports were untracked, so 39 baseline entries reported STALE on a
   clean checkout (reproduced in `.wt/ci-probe`). Fixed by the chain's
   tracked-files-only guard plus committing-then-untracking the quarantined reports
   (`67d7d05`).
2. **Windows path bug in the intelligence scanner** (`c247b00`). `should_exclude()` and
   `classify_role()` matched `/target/`-style substrings against backslash paths, so
   Windows scans walked `target/`, `node_modules/`, `.wt/` — 225 s scans indexing build
   output. This is the mechanical cause of the Phase 2A audit's "counts build output"
   finding. After the one-line normalisation: **344 files scanned in 0.52 s.** The
   precision bound moved 0.25 → 0.20 with the recalibration reason recorded inline
   (recall and timing assertions unchanged).
3. **Guard crashed on Windows consoles** — cp1252 `UnicodeEncodeError` while printing
   findings (`714614e`, `943a94a`).
4. **Retracted claims in tracked `FINAL_SUMMARY.md`** — 6 occurrences corrected
   in-document (`67d7d05`, `3594d1d`).
5. **Two latent dirty files** (`tool.rs` whitespace churn; Finding-G clobber of
   `DEEPSEEK_MASTER_PROMPT_V21.md`) resolved per the triage doc before merging.
6. **Secret-scanner trigger inside a test fixture** — see §4; root-fixed by reshaping
   the hermeticity fixture (no provider prefix, no hex body).

---

## 3. Priority build order (dependency-classified)

### P0 — Repository integrity (DONE this session)

- [x] Commit the load-bearing dirty tree in coherent groups (`76bf9b6`…`a50ea77`)
- [x] Merge the Phase 2A remediation candidate; chain supersedes on overlap (`67e30f1`)
- [x] Retraction compliance (`67d7d05`, `3594d1d`)
- [x] Windows scanner fix + benchmark recalibration (`c247b00`)
- [x] Guard console crash fix (`943a94a`)
- [x] Battery green on merged `main`
- [x] **Push and verify the remote SHA** — see §4 for the push record

### P0.5 — Owner actions outstanding on the remote

1. ~~**Apply the withheld ci.yml comment fix**~~ — **DONE in P1.2 (2026-09-19)**:
   applied on `track/p12-2a-reacceptance` after re-measuring the canonical
   figure from source (`EnhancedMcpBridge::all_definitions()` = 27 tool IDs);
   comment-only, no behaviour change. See
   `PHASE_2A_REACCEPTANCE_EVIDENCE_2026-09-18.md` §15-16.
2. **Push or archive the granular history** (`local/granular-integration`, tip
   `e540ad8`) — requires rewriting the old router.rs fixture strings out of its
   history (push protection evaluates every pushed commit) or using the per-secret
   unblock URLs from the rejection messages.
3. **Resolve the GitHub Actions billing lock** — **RECORDED 2026-09-18, NOT
   OBSERVABLE converted to its observed value:** runs trigger on every push and all
   three OS jobs fail with zero executed steps. Check-run annotation, quoted from the
   API: *"The job was not started because your account is locked due to a billing
   issue."* Classification: **BLOCKED_EXTERNAL**. No code change can clear it; until
   billing is resolved, the local battery is the only verification authority.

### P1 — Make main GREEN in the governance sense (highest value now)

1. **Fresh-clone reproduction** (R4 for Gate-0) — **EXECUTED 2026-09-18, PASS** — see
   `P1_1_FRESH_CLONE_REPRODUCTION_REPORT.md`: on a brand-new clone of the remote HEAD,
   with ambient provider credentials present, the full battery reproduced (417/0
   tests, clippy `-D warnings` clean, guard 11/0, frozen frontend install + build
   green, all-features check green), and the Windows scanner exclusion is proven by
   planted trap files against 16,686 contaminating files in `target/` +
   `node_modules/` (330 entries scanned / 284 indexed / 1,545 symbols / 462.99 ms;
   0 trap hits). Recorded discrepancies: stale doc SHA (D1, fixed by this edit),
   stale ci.yml comment (D2), Recall@10 run-to-run variance 0.58→0.43 (D3, future
   fix: deterministic query ordering), corpus delta 344→330 (D4), CI
   BLOCKED_EXTERNAL (D5). **Rung promotion to R4 is the independent auditor's
   decision, not the builder's.**
1.5. **P1.2 — Phase 2A re-acceptance preparation** — **EXECUTED 2026-09-19** — see
   `PHASE_2A_REACCEPTANCE_EVIDENCE_2026-09-18.md`: benchmark made deterministic
   (10/10 byte-identical cross-process runs; root causes: HashMap iteration,
   stable-sort ties, absolute-path model inputs), known-answer set expanded to
   20 fact-verified queries (P@10=0.48, R@10=1.00), product surface
   `zylcode repo-context` commissioned with committed transcript,
   ContextBuilder/AgentLoop seam integrated with a discriminating test,
   capability registry rebaselined with dispute history preserved. Builder
   recommendation: READY_FOR_INDEPENDENT_PHASE_2A_AUDIT. **Phase 2A remains
   RE-OPENED; acceptance is the auditor's.**

2. **Phase 2A re-acceptance audit** (agent H role): the 10-item remediation order +
   G3 gate, now testable against a fixed, green base. Until accepted, 2A stays
   RE-OPENED.
3. **R3 commissioning slice** per `R3_COMMISSIONING_PLAN.md` — the 7 read/write tools
   (`fs.read`, `fs.write`, `search.find/grep`, `git.status/diff`, `shell.execute`)
   reached through a named product surface with all eleven evidence fields.
   `git.commit` stays excluded until the read/write slice proves the evidence record.

### P2 — Re-opened Phase 2A substance (after P1.2 accepts)

4. Secret-exclusion Windows correctness audit (flagged in the Phase 2A audit).
5. Registry/capability map: `docs/capability-registry.json`, honest status markers.
6. Desktop surface forensic (agent B track) — now unblocked by a compiling base.

### P3 — Parallel docs tracks (C/D/E in the manifest; safe anytime, merge only onto green)

7. Public foundation (community files), DevLog drafts, benchmark **design**.
   Owner decisions outstanding: ZylForge repo/deployment provenance (manifest §15.2),
   Penpot credentials (manifest §15.3), archive-or-delete for the 12 baselined legacy
   carriers.

### Explicit non-starts (LAW 3)

Phase 2B (blocked on 2A re-acceptance), Project System, Vision Studio, Computer-Use
implementation (**replace, do not extend** — the R0 facade), marketplace anything,
Extension ABI reversals, any public claim above its proof rung.

---

## 4. Push record (2026-09-18) — root cause falsified twice, then found

The integrated tree could not be pushed as granular history. Two hypotheses were
tested and rejected before the cause was identified:

1. ~~Branch protection on `main`~~ — **falsified**: a brand-new branch with the same
   commits was rejected too.
2. ~~Missing `workflow` scope blocking the ci.yml delta~~ — **falsified**: a squash
   commit whose diff touches no workflow file was still rejected.

**Actual cause: GitHub Push Protection.** The full rejection message (initially
hidden by output truncation) names an "OpenRouter API Key" finding at
`crates/zylcode-core/src/router.rs:1217` — the hermeticity test's own fixture
(`sk-or-v1-` + 64 hex digits), written by the remediation chain to prove the router
is immune to ambient key values. It is not a real credential, but the scanner
pattern-matches the shape regardless.

**Root fix, not bypass:** the fixture was reshaped to `placeholder-credential-…` —
still non-empty, constant, credential-like, and distinct, but with no provider prefix
and no hex body, so no repository string can match scanner patterns. The test's
semantics are unchanged and the router lib suite still passes (29/0). Push then
succeeded.

### Delivery form and provenance

- **`origin/main` = `6e93c43d`** at first success (verified via `ls-remote`): one
  squash commit on top of the previous remote base `1338d0b`, whose tree is exactly
  the  verified integrated tree. Local `main` was reset onto this lineage so local and
  remote share history; subsequent documentation corrections advanced it further —
  each push was verified by `ls-remote` at the time it happened, and none is
  hard-coded here as a current-state SHA.
- **Granular history preserved locally** at `local/granular-integration` (44 commits,
  tip `e540ad8`). It cannot be pushed as-is: its *history* still contains the old
  fixture strings (push protection evaluates every pushed commit — falsified: the
  archive push was rejected naming router.rs lines 1213/1217/1223). Pushing it
  requires a history rewrite of the fixture or the per-secret unblock URLs.
- ~~**Withheld from the remote:** the chain's 2-line ci.yml comment correction~~ —
  applied in P1.2 (2026-09-19) with the figure re-measured from source; see P0.5.1.

---

## 5. Standing rules for the next agent

- `main` is the integration branch; work in `.wt/<track>` worktrees; one track per
  path.
- Before touching anything: battery first (`check`, `clippy -D warnings`, `test`,
  guard), then diff, then read `docs/governance/`.
- The dirty tree is gone as a concept — it is committed. Unknown work = read the
  provenance commits, never delete.
- `NOT OBSERVABLE` is never `green`. A push is not a push until `ls-remote` shows the
  SHA. A rejected push must be read in full before theorising — the root cause this
  session was visible only in the complete remote message.
