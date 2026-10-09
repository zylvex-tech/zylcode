# C4 Pre-Publication Release Report

**Date:** 2026-10-08  
**Branch:** `competition/natlas-2026`  
**Final HEAD:** `f0fcd011f7ce3955d86bc287480eb5a081f445f2`  
**Author:** ZylForge (independent audit / architecture owner)

---

## 1. Branch

`competition/natlas-2026`

## 2. Starting HEAD (before this run)

`c8e473d3a672cb1690e64b7500e78b221c04513c`  
`docs(natlas): challenge-specific README + C3 verification report`

## 3. Final HEAD

`f0fcd011f7ce3955d86bc287480eb5a081f445f2`  
`docs(natlas): beta tester quickstart + architecture guide + README corrections`

## 4. Competition Commits Proposed for Publication

| SHA | Subject | Files |
|---|---|---|
| `bece9e9` | docs(natlas): add evidence index and submission readiness matrix | docs/competition/natlas/NATLAS_EVIDENCE_INDEX.md, NATLAS_SUBMISSION_READINESS_2026-10-07.md |
| `9f76915` | docs(natlas): complete B1 controlled endpoint + C3 multilingual evidence and submission assets | docs/competition/natlas/ARCHITECTURE_BRIDGE.md, B1_STATUS_2026-10-07.md, FLAGSHIP_ENGINEERING_JOURNEY_REPORT.md, MULTILINGUAL_VALIDATION_MATRIX.md, NATLAS_CHALLENGE_RUBRIC_2026-10-07.md, SUBMISSION_CHECKLIST_2026-10-07.md, VIDEO_SCRIPT_2026-10-07.md, developer/BETA_TEST_EVIDENCE_TEMPLATE.md, developer/BETA_TEST_INVITE.md |
| `246a136` | docs(natlas): record EV-016 Pidgin comprehension + fix multilingual matrix honesty | docs/competition/natlas/MULTILINGUAL_VALIDATION_MATRIX.md |
| `591d35a` | feat(natlas): engineering-bridge journey — real intent→factory execution with approval gate | crates/zylcode-core/src/competition/natlas/intent.rs, types.rs; crates/zylcode-core/tests/natlas_bridge.rs, natlas_live.rs |
| `dadb5d9` | feat(natlas): C3 live runtime verification — genuine Yoruba→N-ATLAS→mutation→test | crates/zylcode-core/src/competition/natlas/intent.rs; crates/zylcode-core/tests/natlas_bridge.rs, natlas_live.rs; docs/competition/natlas/C3_LIVE_RUNTIME_VERIFICATION_REPORT.md |
| `c8e473d` | docs(natlas): challenge-specific README + C3 verification report | README.md, docs/competition/natlas/C3_LIVE_RUNTIME_VERIFICATION_REPORT.md |
| `f0fcd01` | docs(natlas): beta tester quickstart + architecture guide + README corrections | README.md, docs/competition/natlas/ARCHITECTURE_BRIDGE.md, docs/competition/natlas/BETA_TESTER_QUICKSTART.md |

**Total: 7 commits. Diverges from `main` at `b4d13ab`.**

## 5. README Status

✅ **Challenge-specific, judge-facing.** 16 sections covering challenge statement, solution architecture, flagship demonstration, differentiation, N-ATLAS integration, multilingual validation, safety invariants, evidence table, testing, repository structure, validation status, team/attribution, licence, and submission status.

## 6. README Corrections Made

| Correction | Reason |
|---|---|
| Tagline: "Nigerian Pidgin" clarified as comprehension-only bonus | Owner directive: do not represent Pidgin as having equivalent end-to-end validation |
| Removed "carries 5 competition-specific commits" | Volatile statement — becomes stale immediately after next commit |
| Replaced `ARCHITECTURE_BRIDGE.md` blocker with `BETA_TESTER_QUICKSTART.md` | ARCHITECTURE_BRIDGE.md is present and complete; quickstart is the new deliverable |

## 7. Architecture Guide Status

✅ **Complete and updated.** `docs/competition/natlas/ARCHITECTURE_BRIDGE.md` documents:
- Full verified pipeline (instruction → SYSTEM contract → NatlasTransport → endpoint → N-ATLAS → bounded repair → TaskGraph → approval → FactoryRunner → evidence)
- Why N-ATLAS cannot silently degrade (dedicated boundary trait, no fallback state)
- C3 live acceptance journey (EV-024, 12-step chain)
- Bounded JSON repair layer scope and honesty note
- Updated "proven vs pending" reflecting live journey closure

## 8. Beta Tester Quickstart Status

✅ **Created.** `docs/competition/natlas/BETA_TESTER_QUICKSTART.md` contains:
- What the tester is validating (real engineering bridge, not chat)
- Prerequisites (Rust, Git, Python, internet; no API key for hermetic tests)
- Setup (clone, verify branch, health check)
- Exact hermetic test journey with expected counts
- Live endpoint verification (no key needed)
- Human-approval behaviour explanation
- Failure/troubleshooting table (ZeroGPU quota, compile errors, proxy issues)
- Evidence capture instructions (screenshots, OS/version, timing)
- Feedback questions
- Privacy/safety warning (do not submit secrets, personal data, proprietary code)

## 9. Clean-Room Environment Used

- **First clean worktree:** `C:/Temp/zylcode-cleanroom` (detached HEAD `c8e473d`)
- **Final clean worktree:** `C:/Temp/zylcode-final` (detached HEAD `f0fcd01`)
- Both created with `git worktree add` from the local repository — representing exactly what a `git clone` would produce

## 10. Exact Clean-Room Setup Commands

```bash
# What a tester would run:
git clone --branch competition/natlas-2026 https://github.com/zylvex-tech/zylcode.git
cd zylcode

# Verify branch
git log --oneline -3

# Health check (no credentials)
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz

# Hermetic tests (no network, no credentials)
cargo test -p zylcode-core --lib competition::natlas::intent
cargo test -p zylcode-core --test natlas_boundary
cargo test -p zylcode-core --test natlas_runtime
cargo test -p zylcode-core --test natlas_bridge
```

## 11. Exact Fresh Competition Test Counts

Measured from clean worktree `C:/Temp/zylcode-cleanroom` (first compilation from scratch):

| Suite | Count | Result | Duration |
|---|---|---|---|
| `cargo test -p zylcode-core --lib competition::natlas::intent` | **18** | ✅ pass | 0.01s |
| `cargo test -p zylcode-core --test natlas_boundary` | **17** | ✅ pass | 0.66s |
| `cargo test -p zylcode-core --test natlas_runtime` | **12** | ✅ pass | 4.06s |
| `cargo test -p zylcode-core --test natlas_bridge` | **14** | ✅ pass | 4.14s |
| **Total** | **61 / 61** | ✅ **all green** | ~9s (after 3m 10s initial compilation) |

> The only difference between the clean-worktree commit (`c8e473d`) and final HEAD (`f0fcd01`) is documentation files (README.md, ARCHITECTURE_BRIDGE.md, BETA_TESTER_QUICKSTART.md). No Rust source files changed. Test counts are definitive for final HEAD.

## 12. Live Endpoint Health Result

```
GET https://zylvex-natlas-zylcode-bridge.hf.space/healthz
→ HTTP 200
→ model: NCAIR1/N-ATLaS
→ status: healthy
→ attribution: N-ATLaS is an initiative of the Federal Ministry of Communications, Innovation and Digital Economy, and powered by Awarri Technologies.
```

✅ **PASS** — Endpoint is live and serving the genuine model.

## 13. Genuine N-ATLAS Inference Result

Health check response confirms `model: NCAIR1/N-ATLaS`. The endpoint is the controlled Space `zylvex/natlas-zylcode-bridge` running the Apache-2.0 fork of the sovereign engine, which downloads and serves the genuine gated `NCAIR1/N-ATLaS` weights (15.3 GB, 4 safetensors shards). No wrapper model is involved.

Previous genuine inference gates (EV-014, EV-015, EV-016, EV-024) all exercised this same endpoint with real model output.

✅ **PASS** — Genuine N-ATLAS verified.

## 14. Canonical External Tester Journey Result

**Tester journey from BETA_TESTER_QUICKSTART.md:**

1. Clone branch (2 min)
2. Health check — verify `model: NCAIR1/N-ATLaS` (30s)
3. Run 4 hermetic test suites — 61 tests, all green (10 min including first compilation)
4. Capture screenshots and submit feedback

**Prerequisites:** Rust toolchain, Git, Python 3, internet access. **No API key required** for the hermetic tests or health check.

**What testers prove:**
- The bridge mechanics work deterministically without a live endpoint
- Path traversal is rejected, secrets are redacted, approval gates work
- JSON repair handles model serialization defects
- The endpoint is live and serving the genuine model

**If ZeroGPU quota is exhausted:** The hermetic tests still pass. The health check may return 503. Testers are instructed to retry later.

✅ **PASS** — Journey is safe, bounded, self-contained, and requires no owner secrets.

## 15. Licence Verification Table

| Claim | Primary Source | Exact Supported Meaning | README Wording | Verdict |
|---|---|---|---|---|
| N-ATLaS model licence name | `NATLAS_SETUP.md` §2; `NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md` §3.5; `PHASE_C1_OWNER_REPORT_2026-10-06.md` §13 | "Open-Source Research and Innovation License" (N-ATLaS Terms of Use v1.0, September 2025), governed by Nigerian law | "N-ATLaS Open-Source Research and Innovation License (September 2025)" | ✅ PASS |
| Attribution obligations | Same sources | Required: Awarri Technologies AND Federal Ministry of Communications, Innovation and Digital Economy | "Attribution to Awarri + FMCIDE preserved" | ✅ PASS |
| Active-user limitation | Same sources | ≤ 1,000 active end-users per organisation/project (rolling 30 days) | "endpoint enforces the N-ATLaS ≤1000-active-user limit" / "Usage of the controlled endpoint is limited to ≤1000 active end-users" | ✅ PASS |
| Derivative naming requirement | Same sources | Renamed derivative must carry suffix "Powered by Awarri" | "'Powered by Awarri' if renamed" | ✅ PASS |
| Engine repository licence | `B1_STATUS_2026-10-07.md` §1; `NATLAS_CONTRACT_VERIFICATION_2026-10-07.md` | `samuelolubukun/NATLaS-Sovereign-Engine` is Apache-2.0 | "Apache-2.0 fork of the sovereign engine" | ✅ PASS |
| Space fork statement | `B1_STATUS_2026-10-07.md` §1, §2 | `zylvex/natlas-zylcode-bridge` is an Apache-2.0 fork deployed via Hugging Face Space | "The controlled endpoint is an Apache-2.0 fork of the sovereign engine" | ✅ PASS |
| Weight redistribution | `NATLAS_SETUP.md` §8; `PHASE_C1_OWNER_REPORT_2026-10-06.md` §13 | Weights are gated; never committed to Git; endpoint downloads from HF at runtime | "gated weights never redistributed" / "No model weights are committed to this repository" / "The endpoint downloads weights from Hugging Face gated storage at runtime" | ✅ PASS |

## 16. Current-Tree Secret Scan

**Method:** `grep` for `(api_key|token|secret|password|credential|bearer|auth|hf_token|natlas_api_key|sk-[a-zA-Z0-9]{20,})` across all tracked files.

**Result:** ✅ **PASS**

- No actual API keys, tokens, or credentials in any tracked file.
- `NATLAS_API_KEY` appears only as an environment variable name in setup instructions.
- `HF_TOKEN` appears only as documentation of the Space secret configuration (value never printed).
- Test fixtures (`sk-the-real-secret`, `abcDEFsecret`, `sk-abcdef…`) are deliberately synthetic strings used to prove redaction works.
- `YOUR_OPENROUTER_API_KEY` and `placeholder-credential-…` are explicitly documented as placeholders.

## 17. Publication-History Secret Scan

**Method:** `git log -p f0fcd01~7..f0fcd01` searched for bearer tokens, Authorization headers with values, API keys with values, passwords, HF token values, private keys, personal Windows paths.

**Result:** ✅ **PASS**

- No secret values found in any of the 7 competition commits.
- `HF_TOKEN` and `NATLAS_API_KEY` appear only as environment variable names in documentation.
- `a2566d91` is a deployment commit hash, not a secret.
- No personal Windows paths (`C:/Users/`, `AppData/`, `MBARIESERVICESLTD`) in code or documentation.
- No backup files, model weights, or `.env` files in the commit range.

## 18. Foreign Dirty-Tree State

```
 M .github/workflows/ci.yml
 M .github/workflows/release.yml
 M crates/zylcode-core/src/agent.rs
 M crates/zylcode-core/src/cli.rs
 M crates/zylcode-core/src/lib.rs
 M crates/zylcode-core/src/router.rs
 M crates/zylcode-core/src/terminal.rs
?? .git-msg.txt
?? COMPLETE_AUDIT_REPORT.html
?? crates/zylcode-core/src/project/filesystem.rs.backup
?? docs/governance/RECONCILIATION_FORENSIC_REPORT.md
?? run_tests.bat
?? tasks/
?? vc_inspect.py
```

**7 modified + 8 untracked = 14 foreign entries.**

**Status:** ✅ **Untouched.** None staged, none committed, none deleted. The proposed publication commit range (`f0fcd01~7..f0fcd01`) contains only competition-owned files.

## 19. Files Included in Proposed Publication

All files in the 7-commit range above:
- `README.md` (challenge-specific, rewritten)
- `crates/zylcode-core/src/competition/natlas/intent.rs`
- `crates/zylcode-core/src/competition/natlas/types.rs`
- `crates/zylcode-core/tests/natlas_bridge.rs`
- `crates/zylcode-core/tests/natlas_live.rs`
- `docs/competition/natlas/ARCHITECTURE_BRIDGE.md`
- `docs/competition/natlas/B1_STATUS_2026-10-07.md`
- `docs/competition/natlas/BETA_TESTER_QUICKSTART.md`
- `docs/competition/natlas/C3_LIVE_RUNTIME_VERIFICATION_REPORT.md`
- `docs/competition/natlas/FLAGSHIP_ENGINEERING_JOURNEY_REPORT.md`
- `docs/competition/natlas/MULTILINGUAL_VALIDATION_MATRIX.md`
- `docs/competition/natlas/NATLAS_CHALLENGE_RUBRIC_2026-10-07.md`
- `docs/competition/natlas/NATLAS_EVIDENCE_INDEX.md`
- `docs/competition/natlas/NATLAS_SUBMISSION_READINESS_2026-10-07.md`
- `docs/competition/natlas/SUBMISSION_CHECKLIST_2026-10-07.md`
- `docs/competition/natlas/VIDEO_SCRIPT_2026-10-07.md`
- `docs/competition/natlas/developer/BETA_TEST_EVIDENCE_TEMPLATE.md`
- `docs/competition/natlas/developer/BETA_TEST_INVITE.md`

Plus pre-existing tracked files on the branch (workspace files, `LICENSE`, `.gitignore`, etc.).

## 20. Files Explicitly Excluded

All 14 foreign dirty-tree entries (see §18). Notably:
- `.github/workflows/*.yml` (owner CI work)
- `crates/zylcode-core/src/{agent,cli,lib,router,terminal}.rs` (owner modifications)
- `crates/zylcode-core/src/project/filesystem.rs.backup` (backup file)
- `COMPLETE_AUDIT_REPORT.html`, `vc_inspect.py`, `run_tests.bat` (untracked foreign files)
- `docs/governance/RECONCILIATION_FORENSIC_REPORT.md` (untracked foreign doc)
- `tasks/` (untracked foreign directory)

## 21. Remaining Known Limitations

| Limitation | Impact | Mitigation |
|---|---|---|
| ZeroGPU quota | Endpoint may return 503 after sustained use | Hermetic tests require no endpoint; retry health check later |
| Native-speaker prompts not yet used | Hausa/Igbo prompts team-authored; quality bar not final | Recruit native speakers for final submission |
| Pidgin generation untested | Model comprehends Pidgin but replies in English | Documented as bonus only, not competition target |
| Model still emits triple-quoted strings despite strengthened preamble | Repair layer required | Documented honestly; repair is deterministic and tested |
| `HTTP_PROXY`/`HTTPS_PROXY` may block Rust live tests | `LocalNatlasTransport` uses `.no_proxy()` | Live journey verified via Python; hermetic tests unaffected |
| First compilation ~3 minutes | New testers wait for dependency build | Documented in quickstart; subsequent runs are fast |

## 22. Remaining Submission Blockers

| Blocker | Owner Action | Hard Deadline |
|---|---|---|
| ≥2 external beta testers | Recruit + collect feedback | 12 Oct 2026 |
| Team profile | Names, affiliations, roles | 12 Oct 2026 |
| CAC certificate (Track B) | Upload | 12 Oct 2026 |
| Final video screen-capture | 3–5 min end-to-end demo recording | 12 Oct 2026 |

## 23. Proposed GitHub Push Command

```bash
GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=echo git push origin competition/natlas-2026
```

**NOT AUTHORIZED. Execute only after owner review and explicit authorization.**

## 24. Exact Commit SHA Testers Should Test After Publication

`f0fcd011f7ce3955d86bc287480eb5a081f445f2`

---

## Gate Classifications

| Gate | Verdict |
|---|---|
| PUBLIC_BUILD_REPRODUCIBLE | ✅ PASS |
| PUBLIC_TESTS | ✅ PASS (61/61) |
| LIVE_ENDPOINT | ✅ PASS (HTTP 200, model NCAIR1/N-ATLaS) |
| GENUINE_NATLAS_INFERENCE | ✅ PASS (healthz + prior EV-014/015/016/024) |
| TESTER_QUICKSTART | ✅ PASS (self-contained, no secrets, bounded, safe) |
| SECRET_SCAN_CURRENT | ✅ PASS |
| SECRET_SCAN_HISTORY | ✅ PASS |
| LICENCE_ATTRIBUTION | ✅ PASS (all 7 claims verified against primary sources) |
| FOREIGN_WORK_EXCLUDED | ✅ PASS (14 foreign entries untouched, not in commit range) |

---

## Final Verdict

**READY_FOR_FIRST_PUBLIC_BETA_PUSH**

The `competition/natlas-2026` branch is self-contained, testable by external users without owner secrets, contains no leaked credentials in current tree or commit history, honours all N-ATLaS licence obligations, and includes a clear tester entry point. The foreign dirty tree is preserved untouched.

---

**READY FOR OWNER AUTHORIZATION — FIRST PUBLIC BETA PUSH**

Do not push until the owner explicitly authorizes it.
