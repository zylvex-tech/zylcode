# COMPETITION FINAL RELEASE REPORT

**Product:** N-ATLAS × ZylCode Developer Bridge
**Challenge:** NAIC 2026 — PS1 Developer Infrastructure (Track B)
**Branch:** `competition/natlas-2026`
**Base commit:** `8216738` (this report is added in the release commit that follows it)
**Date:** 2026-10-10
**Author:** Zylvex Technologies Limited (engineering)

---

## 0. Executive summary

| | |
|---|---|
| **Overall classification** | 🟡 **AMBER** |
| Reason | All engineering gates are green and the live deployment is genuine and reachable. Three submission artefacts (video, team profile, CAC certificate) are **owner-supplied** and not produced; multilingual validation remains **PARTIAL**; the live endpoint is quota-blocked after one call. |
| Nothing is production-approved | Correct — no module is claimed as production-ready. |

**What "AMBER" means here:** the *engineering* is release-ready; the *submission package*
is not complete, and the incompleteness is **external**, not technical.

---

## 1. A–R classification

| Id | Area | Classification | Basis |
|---|---|---|---|
| **A** | Repository safety | 🟢 **GREEN** | Working tree clean; no untracked files; no stash; branch in sync with remote (`0/0`). |
| **B** | Branch integrity | 🟢 **GREEN** | `competition/natlas-2026` exists locally and remotely; local HEAD == remote HEAD == `8216738`. |
| **C** | `main` untouched | 🟢 **GREEN** | `main` not modified, not pushed, not merged. (Local `main` is 6 commits ahead of `origin/main` — pre-existing owner work, preserved, **not** pushed.) |
| **D** | Fresh Rust gates | 🟢 **GREEN** | 498 lib / 68 competition / 0 failed — all re-run today, not reused. |
| **E** | Fresh SDK gates | 🟢 **GREEN** | `tsc --noEmit` exit 0; smoke 16 passed / 0 failed. |
| **F** | Lint + guards | 🟢 **GREEN** | `clippy -D warnings` exit 0; retracted-claims guard OK. |
| **G** | Evidence reconciliation | 🟢 **GREEN** | Both spreadsheets read and reconciled; tester's commit verified; 18-pass claim reproduced. |
| **H** | Tester PII protection | 🟢 **GREEN** | Emails withheld from all deliverables; names retained only as already disclosed. |
| **I** | Live deployment reachable | 🟢 **GREEN** | HTTP 200; `/healthz` 200; canonical HF Space confirmed. |
| **J** | Model identity | 🟢 **GREEN** | `/healthz` reports `NCAIR1/N-ATLaS`, PyTorch-Transformers, ZeroGPU. |
| **K** | Genuine live inference | 🟡 **AMBER** | **One genuine reply obtained** ("The sum of 2 and 2 is 4."). Subsequent calls quota-blocked. Partial by environment, not by defect. |
| **L** | Quota behaviour | 🟡 **AMBER** | Confirmed exhausted after first call; English retry also failed (proving capacity, not language). `/healthz` misleadingly `healthy`. Documented. |
| **M** | Deployment separation | 🟢 **GREEN** | Confirmed: HF Space is a **separate** repository (SHA `a2566d91`); no auto-follow. No deployment action required or taken. |
| **N** | Multilingual claims | 🟡 **AMBER** | A/B/C claim separation added. Comprehension evidenced (EV-015) but native-speaker sign-off absent; live re-check quota-blocked. |
| **O** | Documentation completeness | 🟢 **GREEN** | All nine required documents present (README, quickstart, external beta report, multilingual matrix, test evidence, deployment verification, known limitations, submission checklist, this report). |
| **P** | Submission matrix | 🟡 **AMBER** | 18 items: 14 PASS, 1 PARTIAL, 3 BLOCKED (all three are owner-supplied artefacts). |
| **Q** | Video package | 🟡 **AMBER** | Full production package produced; **video NOT recorded, NOT uploaded** — a human step. |
| **R** | Release commit & push | 🟢 **GREEN** | Committed and pushed to `competition/natlas-2026`; `main` untouched. |

**Roll-up:** 12 GREEN · 6 AMBER · 0 RED.

---

## 2. What was executed (not merely written)

| Phase | Action | Outcome |
|---|---|---|
| 0 | Repository safety — 12 checks | Clean; branch in sync; visibility unchanged; `Cargo.lock` finding recorded |
| 1 | 10-gate fresh battery | All green; log `EV-027` |
| 2 | Two spreadsheets reconciled | 3 responses, 2 distinct testers; `EXTERNAL_BETA_TEST_REPORT.md` |
| 3 | Live deployment verification | Endpoint verified; 1 genuine inference; quota finding; `EV-028` |
| 4 | Multilingual claim separation | A/B/C separation added to the matrix |
| 5 | Nine documents produced/updated | All present |
| 6 | 18-item submission matrix | 14 PASS / 1 PARTIAL / 3 BLOCKED |
| 7 | Video production package | Script, shot list, checklist, timings |
| 8 | Commit and push | See §4 |
| 9 | This report | A–R classification above |

---

## 3. The four material findings

These are the things a reviewer or the owner most needs to know.

### 3.1 · The live endpoint ran out of GPU quota after a single call
First call: genuine success. Every call after it: error. **English also failed on retry**,
so the cause is capacity, not language. This is NAT-A-001's failure mode, now observed
first-hand. The client classifies it `QUOTA` and refuses to retry — correct, but it cannot
create budget.

### 3.2 · `/healthz` says `healthy` while generation fails
Measured directly. The health endpoint does not reflect quota state. This can mislead a
reviewer. Documented as L-2 and surfaced in the tester quickstart.

### 3.3 · GitHub and Hugging Face are two separate repositories
The owner's statement is **confirmed**: the Space (`zylvex/natlas-zylcode-bridge`,
SHA `a2566d91`) is an independent git repository with no mirror, submodule, or auto-sync
to `github.com/zylvex-tech/zylcode`. **No deployment action was required** — the NAT-A
remediation is entirely client-side. The deployment procedure is documented but was
deliberately **not** executed, because it would rebuild/restart the only live endpoint and
consume scarce quota before the deadline.

### 3.4 · The external testers validated the commit *before* the remediation
Sheet B cites `5599392`, which is the **direct parent** of `HEAD` (`8216738`). The testers
therefore exercised pre-remediation code. Their checkable claim (18 passed / 0 failed) was
reproduced by us at `HEAD`, so the tested behaviour did not regress — but the remediation
itself is **newer than any external validation**.

---

## 4. Repository state

| Item | Value |
|---|---|
| Branch | `competition/natlas-2026` |
| Base commit | `8216738` |
| Remote | `origin` → `https://github.com/zylvex-tech/zylcode.git` |
| `main` | Untouched (not pushed, not merged) |
| Visibility | **PUBLIC** — pre-existing; **not changed** per instruction |
| `Cargo.lock` | **Not tracked** (`.gitignore` line 4) — reported, not silently changed |

---

## 5. What is NOT done

| Item | Owner | Blocking? |
|---|---|---|
| Demonstration video (record + upload) | Owner | Yes — submission component 5 |
| Team profile (names, affiliations, roles) | Owner | Yes — component 6 |
| CAC certificate (Track B) | Owner | Yes — component 7 |
| Native-speaker-authored target prompts | Owner / native speaker | Recommended |
| Linux CI execution | Engineering | No |
| `/v1/*` verification with a real key | Owner (key holder) | No |

---

## 6. Honest statement of readiness

- The **engineering** is complete for this release: every gate is green, the live endpoint
  is genuine, and the evidence is reproducible.
- The **submission** is **not** complete: three artefacts require human action, and one
  validation axis (multilingual) is partial.
- **No capability is claimed that was not measured.** Where something was not run, it is
  labelled *NOT RUN*. Where something is environment-blocked, it is labelled *BLOCKED*, not
  *failed*.
- **P1B / further work is not started** — this release stops where instructed.

*End of report.*
