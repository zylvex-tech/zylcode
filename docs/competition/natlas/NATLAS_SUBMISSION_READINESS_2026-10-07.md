# N-ATLAS SUBMISSION READINESS MATRIX (Phase 13)

- **Date:** 2026-10-07
- **Branch:** `competition/natlas-2026` @ `2a6035e`
- **Deadline:** 2026-10-12 23:59 WAT — **5 days remaining**
- **Statuses:** `PASS` · `FAIL` · `BLOCKED` · `NOT TESTED` · `NOT APPLICABLE`

> A row is `PASS` only when a reproducible artefact supports it. Code existing, a test file being
> present, or a UI rendering do **not** earn a `PASS`.

---

| # | Requirement | Status | Evidence / note |
|---|---|---|---|
| 1 | Competition eligibility | `NOT TESTED` | Portal rules not yet re-read against the current build |
| 2 | CAC / company evidence | `BLOCKED` | Owner-held document; not in this repository |
| 3 | Team | `BLOCKED` | Owner to name the submission team |
| 4 | Working artifact | `PASS` (bounded) | Real Rust + TS SDK + playground; 58 Rust tests + 10 SDK tests pass |
| 5 | N-ATLAS integration | `PASS` (bounded) | Genuine invocation captured (EV-001); sustained path blocked on B1 |
| 6 | Developer Infrastructure fit | `PASS` | Provider seam + task graph + evidence graph; not a chat wrapper |
| 7 | External beta tester #1 | `FAIL` | **Zero external testers.** Package prepared, unpopulated |
| 8 | External beta tester #2 | `FAIL` | **Zero external testers.** |
| 9 | Technical documentation | `PARTIAL` | 23 docs + 3 new 2026-10-07 docs; sections awaiting real evidence |
| 10 | Architecture diagram | `NOT TESTED` | Described in text; rendered diagram not verified |
| 11 | Demo video | `FAIL` | Not recorded |
| 12 | Repository | `BLOCKED` | Private; publication not authorised |
| 13 | Installation instructions | `PASS` | `docs/competition/natlas/developer/INSTALLATION.md` |
| 14 | Security | `PASS` | Secret redaction implemented + tested; no credentials tracked |
| 15 | Runtime evidence | `PASS` (partial) | EV-000/001/005/006/008 pass; EV-002/003/004 quota-blocked |
| 16 | Submission text | `FAIL` | Not written |
| 17 | Final upload | `BLOCKED` | Depends on 1–16 |
| 18 | Deadline | `PASS` | 5 days remaining as of 2026-10-07 |

---

## Non-PASS items — owner, action, blocker, deadline

| # | Item | Owner | Next action | Blocker | Deadline |
|---|---|---|---|---|---|
| 2 | CAC / company evidence | Owner | Attach registration docs | none | 2026-10-11 |
| 3 | Team | Owner | Name the team | none | 2026-10-10 |
| 7 | Beta tester #1 | Owner | Recruit + run `BETA_TEST_GUIDE.md` | Needs a runnable endpoint | 2026-10-11 |
| 8 | Beta tester #2 | Owner | Same as #7 | Same | 2026-10-11 |
| 10 | Architecture diagram | Owner/Agent | Render the described diagram | none | 2026-10-10 |
| 11 | Demo video | Owner | Record from `VIDEO_DEMO_PLAN.md` | Needs a live endpoint (B1) | 2026-10-11 |
| 12 | Repository publication | Owner | Decide public/private | Policy decision | 2026-10-11 |
| 16 | Submission text | Owner/Agent | Draft from this pack | none | 2026-10-11 |
| 17 | Final upload | Owner | Submit | Items 1–16 | 2026-10-12 |

---

## The one decision that unblocks most of this

**B1 — obtain a controlled N-ATLAS endpoint.** Three routes, in order of preference:

1. **Deploy our own engine Space** (duplicate `samuelolubukun/NATLaS-Sovereign-Engine` under the
   `zylvex` account, set our own `NATLAS_API_KEY`). Gives a repeatable, credentialed,
   OpenAI-compatible endpoint. *Requires owner authorisation to create a public Space.*
2. **Download the weights and run locally** via Ollama (mirror `Q4_K_M` 4.92 GB, ≈5.7 h at the
   measured rate). Fully offline; no third party in the loop.
3. **Use the public Space** (works today, zero setup) — but rate-limited and third-party.

Until B1 is resolved, rows 5, 11, 15 and both beta-test rows cannot be completed honestly.
