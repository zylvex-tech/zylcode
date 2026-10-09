# Beta Release Candidate 1 — RC1

**RC Name:** BETA RELEASE CANDIDATE 1 — RC1  
**Exact SHA:** `1f80322520dcb92c95fffdf8ba01d51afcdb4314`  
**Date/Time:** 2026-10-08 03:21 GMT+1  
**Branch:** `competition/natlas-2026`  
**Test Status:** 61/61 competition tests passing (hermetic)  
**Live Endpoint:** `https://zylvex-natlas-zylcode-bridge.hf.space/healthz` → 200, model `NCAIR1/N-ATLaS`  

## Known Limitations

- ZeroGPU quota may cause 503 on sustained endpoint use; hermetic tests require no endpoint
- Native-speaker-authored prompts recommended for Hausa/Igbo final validation
- Pidgin comprehension proven; generation-in-Pidgin untested
- Model still emits triple-quoted strings despite strengthened preamble; repair layer handles deterministically
- First `cargo test` compilation takes ~3 minutes; subsequent runs are fast
- `HTTP_PROXY`/`HTTPS_PROXY` may require endpoint-specific config for Rust live tests (Python scripts use proxy correctly)

## Tester Instructions Path

`docs/competition/natlas/BETA_TESTER_QUICKSTART.md`

## GitHub Repository/Branch URL

`https://github.com/zylvex-tech/zylcode/tree/competition/natlas-2026`

## What Testers Must Do

1. Clone the branch at the exact SHA above
2. Run the hermetic test suites (61 tests, no credentials needed)
3. Verify endpoint health (`curl` healthz, no credentials needed)
4. Capture screenshots and submit feedback via the quickstart instructions

## What Testers Must NOT Do

- Do not submit secrets, personal data, or proprietary code
- Do not bypass endpoint authentication
- Do not claim live inference proven merely from healthz or hermetic tests

## Push Status

**Local HEAD is ready. Remote push requires owner-supplied git credentials.**

---

*Frozen at RC1 SHA. Any subsequent change requires a new RC number.*
