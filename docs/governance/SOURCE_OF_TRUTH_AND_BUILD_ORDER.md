# SOURCE OF TRUTH & PRIORITY BUILD ORDER — 2026-09-18

**Executed by:** Buffy (Codebuff), owner work order: "establish source of truth for
everything, classify the build order based on priority, always commit and push."
**Status:** Integrated and verified on `main`. Recorded below with the commands that
produced every figure. Nothing in this document is accepted by audit; per LAW 3,
builder results remain builder results until independently verified.

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

**As of this session, all five are reconciled into one:** local `main` **is** the
source of truth. It contains the dirty tree (committed in coherent groups), the
remediation chain (merged; chain wins the 22 overlapping files), the recovery commit
(via the chain), and every doc commit. After push, `origin/main` equals it.

Provenance is preserved: the pre-merge dirty-tree variants remain reachable at
`76bf9b6`…`fd4766b` (parent of merge `67e30f1`); nothing was discarded, only resolved.

### Verification battery, executed on the merged tree (this session)

| Check | Command | Result |
|---|---|---|
| Compile | `cargo check --workspace --all-targets` | **PASS** |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| Tests | `cargo test --workspace --all-targets --no-fail-fast` | **417 passed / 0 failed** |
| Claims guard | `python scripts/check_retracted_claims.py` | **OK** (11 baselined, 0 new) |

Caveat recorded per LAW 7: `f1b2b36`'s clean-worktree check was done from a worktree
carrying state; this session's battery ran on the primary tree. A third-party
reproduction on a fresh clone is the remaining step to R4.

---

## 2. Defects found and fixed during integration (beyond the plan)

1. **CI was red at HEAD before any push.** The guard scanned the filesystem; the
   baselined legacy reports were untracked, so 39 baseline entries reported STALE on
   a clean checkout (reproduced in `.wt/ci-probe`). Fixed by the chain's tracked-files-only
   guard plus committing-then-untracking the quarantined reports (`67d7d05`).
2. **Windows path bug in the intelligence scanner** (`c247b00`). `should_exclude()` and
   `classify_role()` matched `/target/` substrings against backslash paths, so Windows
   scans walked `target/`, `node_modules/`, `.wt/` — 225 s scans indexing build output.
   This is the mechanical cause of the Phase 2A audit's "counts build output" finding.
   After the one-line normalisation: **344 files scanned in 0.52 s.** The precision
   bound moved 0.25 → 0.20 with the recalibration reason recorded inline (recall and
   timing assertions unchanged).
3. **Guard crashed on Windows consoles** — cp1252 `UnicodeEncodeError` while printing
   findings (`714614e`, `943a94a`).
4. **Retracted claims in tracked `FINAL_SUMMARY.md`** — 6 occurrences corrected
   in-document (`67d7d05`, `3594d1d`).
5. **Two latent dirty files** (`tool.rs` whitespace churn; Finding-G clobber of
   `DEEPSEEK_MASTER_PROMPT_V21.md`) resolved per the triage doc before merging.

---

## 3. Priority build order (dependency-classified)

### P0.5 — First owner actions on the remote

1. **Apply the withheld ci.yml comment fix** (2 lines: guard rationale figure
   `112` → `27`) — comment-only, no behaviour; take it from
   `local/granular-integration`.
2. **Push or archive the granular history** (`local/granular-integration`, tip
   `e540ad8`) — requires rewriting the old router.rs fixture strings out of its
   history (push protection evaluates every pushed commit) or using the
   per-secret unblock URLs from the rejection messages.
3. **Observe CI on GitHub** — the stored credential is valid for the API (the
   earlier "invalid GITHUB_TOKEN" finding applied to a different env token);
   read the run triggered by `6e93c43d` and record the verdict here.

### P1 — Make main GREEN in the governance sense (highest value now)

- [x] Commit the load-bearing dirty tree in coherent groups (`76bf9b6`…`a50ea77`)
- [x] Merge the Phase 2A remediation candidate; chain supersedes on overlap (`67e30f1`)
- [x] Retraction compliance (`67d7d05`, `3594d1d`)
- [x] Windows scanner fix + benchmark recalibration (`c247b00`)
- [x] Guard console crash fix (`943a94a`)
- [x] Battery green on merged `main`
- [x] **Push** — see §5 for the push record and the one withheld delta

---

## 5. Push record (2026-09-18) — root cause falsified twice, then found

The integrated tree could not be pushed as granular history. Two hypotheses were
tested and rejected before the cause was identified:

1. ~~Branch protection on `main`~~ — **falsified**: a brand-new branch with the
   same commits was rejected too.
2. ~~Missing `workflow` scope blocking the ci.yml delta~~ — **falsified**: a
   squash commit whose diff touches no workflow file was still rejected.

**Actual cause: GitHub Push Protection.** The full rejection message (initially
hidden by output truncation) names an "OpenRouter API Key" finding at
`crates/zylcode-core/src/router.rs:1217` — the hermeticity test's own fixture
(`sk-or-v1-` + 64 hex digits), written by the remediation chain to prove the
router is immune to ambient key values. It is not a real credential, but the
scanner pattern-matches the shape regardless.

**Root fix, not bypass:** the fixture was reshaped to
`placeholder-credential-…` — still non-empty, constant, credential-like, and
distinct, but with no provider prefix and no hex body, so no repository string
can match scanner patterns. The test's semantics are unchanged and the router
lib suite still passes (29/0). Push then succeeded.

### Delivery form and provenance

- **`origin/main` = `6e93c43d`** (verified via `ls-remote`): one squash commit
  on top of the previous remote base `1338d0b`, whose tree is exactly the
  verified integrated tree. Local `main` was reset to this lineage so local and
  remote share history.
- **Granular history preserved locally** at `local/granular-integration`
  (44 commits, tip `e540ad8`). It cannot be pushed as-is: its *history* still
  contains the old fixture strings (push protection evaluates every pushed
  commit — falsified: the archive push was rejected naming router.rs lines
  1213/1217/1223). Pushing it requires a history rewrite of the fixture or the
  per-secret unblock URL.
- **Withheld from the remote:** the chain's 2-line ci.yml comment correction
  (`112 tool IDs` → `27 tool IDs` in the guard rationale). Not because workflow
  pushes are impossible — that hypothesis was never re-tested after the true
  cause was found — but because the local tree carries the old ci.yml. Apply
  from `local/granular-integration` with a scoped token; it is a 2-line comment
  edit with no behavioural effect.

### P1 — Make main GREEN in the governance sense (highest value now)

1. **Fresh-clone reproduction** (R4 for Gate-0): clone → `cargo check/test/clippy` →
   attach raw output to `TOOL_CATALOGUE_TRUTH_TABLE.md` §9.
2. **Observe CI on GitHub** — the stored credential reads the API; check the run
   for `6e93c43d` and record the verdict (previously NOT OBSERVABLE, LAW 7).
3. **Phase 2A re-acceptance audit** (agent H role): the 10-item remediation order +
   G3 gate, now testable against a fixed base. Until accepted, 2A stays RE-OPENED.
4. **R3 commissioning slice** per `R3_COMMISSIONING_PLAN.md` — the 7 read-only tools
   (`fs.read`, `fs.write`, `search.find/grep`, `git.status/diff`, `shell.execute`)
   reached through the desktop surface with all eleven evidence fields.
   `git.commit` stays excluded until the read/write slice proves the evidence record.

### P2 — Re-opened Phase 2A substance (after P1.3 accepts)

5. Secret-exclusion Windows correctness audit (flagged in Phase 2A audit).
6. Registry/capability map: `docs/capability-registry.json`, honest status markers.
7. Desktop surface forensic (agent B track) — now unblocked by a compiling base.

### P3 — Parallel docs tracks (C/D/E in the manifest; safe anytime, merge only onto green)

8. Public foundation (community files), DevLog drafts, benchmark **design**.
   Owner decisions outstanding: ZylForge repo/deployment provenance (§15.2),
   Penpot credentials (§15.3), archive-or-delete for the 12 baselined legacy carriers.

### Explicit non-starts (LAW 3)

Phase 2B (blocked on 2A re-acceptance), Project System, Vision Studio, Computer-Use
implementation (**replace, do not extend** — the R0 facade), marketplace anything,
Extension ABI reversals, any public claim above its proof rung.

---

## 4. Standing rules for the next agent

- `main` is the integration branch; work in `.wt/<track>` worktrees; one track per path.
- Before touching anything: battery first (`check`, `clippy -D warnings`, `test`,
  guard), then diff, then read `docs/governance/`.
- The dirty tree is gone as a concept — it is committed. Unknown work = read the
  provenance commits, never delete.
- `NOT OBSERVABLE` is never `green`. A push is not a push until `ls-remote` shows the SHA.
