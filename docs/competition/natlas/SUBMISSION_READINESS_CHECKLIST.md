# SUBMISSION READINESS CHECKLIST

**Product:** N-ATLAS × ZylCode Developer Bridge
**Challenge:** NAIC 2026 — PS1 Developer Infrastructure (Track B)
**Date:** 2026-10-10 · **Branch:** `competition/natlas-2026` · **Commit:** `8216738`

**Legend:** `PASS` · `PARTIAL` · `BLOCKED` · `NOT REQUIRED` · `NEEDS VERIFICATION`

---

## The 18-item matrix

| # | Item | Status | Evidence / note |
|---|---|---|---|
| 1 | Controlled N-ATLAS endpoint is deployed and reachable | **PASS** | `https://zylvex-natlas-zylcode-bridge.hf.space` → HTTP 200; `/healthz` → `NCAIR1/N-ATLaS`. EV-028. |
| 2 | Endpoint is Zylvex-controlled and gated | **PASS** | Space under `zylvex` account; `/v1/*` returns 401 without `NATLAS_API_KEY`. |
| 3 | ZylCode integrates N-ATLAS through a dedicated seam | **PASS** | `NatlasTransport` boundary trait; 14 bridge tests + 17 boundary tests green. |
| 4 | N-ATLAS is genuine — not a wrapper of a general-purpose model | **PASS** | Real gated weights loaded at runtime; `NatlasTransport` deliberately separate from the product `ModelProvider` router. Disqualification rule addressed. |
| 5 | Attribution and licence compliance | **PASS** | Space is Apache-2.0 fork; weights not redistributed; FMCIDE/Awarri attribution in README + `/healthz`. |
| 6 | N-ATLAS integration evidence — inference gate | **PASS** | EV-014 (`NATLAS_ZYLCODE_OK`), EV-000 (`/healthz`). |
| 7 | N-ATLAS integration evidence — multilingual round trip | **PARTIAL** | EV-015 evidenced Yoruba/Hausa/Igbo comprehension; prompts team-authored, not native-speaker-authored. Live re-check 2026-10-10 blocked by quota (EV-028). |
| 8 | N-ATLAS integration evidence — repo-changing C3 journey | **PASS** | EV-024: Yoruba instruction → real model → repair → `helloworld.py` written → `python helloworld.py` exit 0. |
| 9 | Strict JSON contract + bounded repair | **PASS** | EV-023; `parse_with_repair`; 18 intent tests green. |
| 10 | Real-world validation — live acceptance journey | **PASS** | EV-024 (C3 LIVE RUNTIME VERIFIED). |
| 11 | Real-world validation — ≥2 external testers | **PASS (on distinct-person count)** | 2 distinct people (Ibrahim Abdulrahman, Auwal). `EXTERNAL_BETA_TEST_REPORT.md` §8. Caveat: neither session reproduced by us (L-11). |
| 12 | Real-world validation — external feedback actioned | **PASS** | NAT-A-001…004 remediated and regression-tested; `TRACK_A_TESTER_001_RELEASE_2026-10-09.md`. |
| 13 | Technical documentation — README | **PASS** | Root `README.md`, incl. "Live Demo & External Testing". |
| 14 | Technical documentation — architecture write-up | **PASS** | `ARCHITECTURE_BRIDGE.md`. |
| 15 | Technical documentation — setup/usage docs for judges | **PASS** | `BETA_TESTER_QUICKSTART.md`, `LOCAL_RUNTIME_FALLBACK.md`, `docs/competition/natlas/developer/`. |
| 16 | Video demonstration (3–5 min) | **BLOCKED** | Script + storyboard exist (`VIDEO_SCRIPT_2026-10-07.md`); **recording and upload are a human step**. Not recorded, not uploaded. |
| 17 | Team profile | **BLOCKED** | Owner-supplied (names, affiliations, roles). Human input. |
| 18 | Endorsement / CAC registration (Track B) | **BLOCKED** | Owner-supplied certificate. Human input. |

**Roll-up:** 14 PASS · 1 PARTIAL · 3 BLOCKED · 0 NOT REQUIRED · 0 NEEDS VERIFICATION.

---

## Cross-cutting gates (verified today)

| Gate | Command | Result |
|---|---|---|
| Rust tests | `cargo test -p zylcode-core --lib` | 498 passed / 0 failed |
| Competition tests | intent+boundary+runtime+bridge+multilingual | **68 / 68 passed** |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| SDK types | `npx tsc --noEmit` | exit 0 |
| SDK tests | `node --experimental-strip-types test/smoke.mjs` | 16 passed / 0 failed |
| Retracted-claims guard | `python scripts/check_retracted_claims.py` | OK |

Full log: `evidence/EV-027-competition-verification-2026-10-10.log`.

---

## The three BLOCKED items

All three require **human action** and cannot be produced by an agent:

| # | Item | What is needed | Owner |
|---|---|---|---|
| 16 | Video demonstration | Record a 3–5 min screen capture of the live journey; upload. | Owner |
| 17 | Team profile | Names, affiliations, roles for Zylvex Technologies Limited. | Owner |
| 18 | Endorsement / registration | CAC certificate (Track B). | Owner |

None of these is a code defect. Each is an externally-supplied artefact.

---

## What is deliberately NOT claimed as ready

- The video is **not** recorded or uploaded.
- The team profile and CAC certificate are **not** supplied.
- Multilingual validation is **PARTIAL**, not complete.
- No item is marked production-approved.

*End of checklist.*
