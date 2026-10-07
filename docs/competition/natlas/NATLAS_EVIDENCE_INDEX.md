# N-ATLAS EVIDENCE INDEX

- **Maintained by:** ZylForge (audit pass)
- **Opened:** 2026-10-07
- **Branch:** `competition/natlas-2026`

Every item states the claim, the action, the expected vs observed result, a status, and an
artefact path. Statuses: `PASS` · `FAIL` · `BLOCKED`.

> **Screenshots are not used as substitutes for technical evidence.** Where a log, an HTTP
> exchange or a test run can establish the claim more strongly, that is what is recorded.

> **Artefact location note.** Raw captures live in `docs/competition/natlas/evidence/`, which is
> `.gitignore`d (runtime evidence may embed repository content and must not be published raw).
> This index is tracked; the raw files are local. Where a capture exists only in a session
> transcript this is stated explicitly in the row.

---

| ID | Claim | Date/time | Build | Environment | Action performed | Expected | Observed | Status | Artefact |
|---|---|---|---|---|---|---|---|---|---|
| EV-000 | A live endpoint genuinely serves the official `NCAIR1/N-ATLaS` weights | 2026-10-07 13:05 GMT+1 | `2a6035e` | Win11 + local proxy | `GET …/healthz` on `samuelolubukun-natlas-sovereign-engine.hf.space` | JSON naming the model | `{"status":"healthy","service":"natlas-engine","model":"NCAIR1/N-ATLaS", …, attribution: FMCIDE/Awarri}` | **PASS** | `evidence/EV-000-engine-healthz.json` |
| EV-001 | A genuine N-ATLAS inference returns real model output | 2026-07-07 (session) | `2a6035e` | Win11 + local proxy | `POST …/gradio_api/call/generate` prompt *"Reply with exactly the word PONG and nothing else."* | The model replies | SSE `event: complete` / `data: ["PONG", null]`; `event_id 342899c534814f67be9fb33e4b704150` | **PASS** | `evidence/EV-001-genuine-invocation-pong.txt` (transcribed from session terminal) |
| EV-002 | N-ATLAS answers an English arithmetic prompt | 2026-10-07 | `2a6035e` | Win11 + local proxy | Same endpoint, prompt *"What is 17 multiplied by 23?"* | A number | `event: error` — `ZeroGPU quota exceeded (180s requested vs. 173s left)` | **BLOCKED** | `evidence/EV-002-english-arithmetic-BLOCKED.txt` |
| EV-003 | N-ATLAS answers a Yoruba prompt | 2026-10-07 | `2a6035e` | Win11 + local proxy | Same endpoint, Yoruba prompt | Yoruba text | `ZeroGPU quota exceeded` | **BLOCKED** | `evidence/EV-003-yoruba-BLOCKED.txt` |
| EV-004 | N-ATLAS answers an Igbo prompt | 2026-10-07 | `2a6035e` | Win11 + local proxy | Same endpoint, Igbo prompt | Igbo text | `ZeroGPU quota exceeded` | **BLOCKED** | `evidence/EV-004-igbo-BLOCKED.txt` |
| EV-005 | The competition unit suite passes | 2026-10-07 | `2a6035e` | `cargo 1.97.1`, `--test-threads=1` | `cargo test -p zylcode-core --lib competition::` | 29 pass | **29 passed; 0 failed; 0 ignored** | **PASS** | session log `/tmp/natlas_test_lib.log` |
| EV-006 | The N-ATLAS integration suites pass | 2026-10-07 | `2a6035e` | `cargo 1.97.1`, `--test-threads=1` | `cargo test -p zylcode-core --test natlas_boundary --test natlas_runtime` | 17 + 12 pass | **17 passed; 0 failed; 0 ignored** and **12 passed; 0 failed; 0 ignored** | **PASS** | `/tmp/natlas_test_integration.log` |
| EV-007 | The official demo engine is reachable | 2026-10-07 | `2a6035e` | Win11 + local proxy | `GET/POST natlas-engine.publicaai.com/*` | 200 | **HTTP 502 on every path** | **FAIL** | `NATLAS_CONTRACT_VERIFICATION_2026-10-07.md` §A |
| EV-008 | The gated official weights are downloadable | 2026-10-07 | `2a6035e` | Win11 + local proxy | `GET NCAIR1/N-ATLaS/resolve/main/config.json` with HF token | 200 | **HTTP 200** (was 401 at Phase C1) | **PASS** | `NATLAS_CURRENT_STATE_2026-10-07.md` §5 |
| EV-009 | The official weights can be acquired in-session | 2026-10-07 | `2a6035e` | Win11 + local proxy | Timed range downloads (1 conn, 6 conn, direct) | usable rate | **~144 KB/s / ~238 KB/s / ~45 KB/s** → 5.7–17.6 h | **FAIL** (impractical) | `NATLAS_CURRENT_STATE_2026-10-07.md` §5 |

---

## Integration suites (measured 2026-10-07)

```
cargo test -p zylcode-core --lib competition::            -> 29 passed; 0 failed; 0 ignored
cargo test -p zylcode-core --test natlas_boundary         -> 17 passed; 0 failed; 0 ignored
cargo test -p zylcode-core --test natlas_runtime          -> 12 passed; 0 failed; 0 ignored
                                                             -----------------------------
                                                             Rust competition total: 58 / 58
```

This **independently reproduces** the Phase C1 report's `58 / 0 / 0` claim. The claim was true.

## What this index does NOT contain

- No fabricated tester rows (there are **zero** external testers).
- No claim of sustained N-ATLAS availability. One genuine round trip is recorded; repeatable
  evidence requires a controlled endpoint (blocker B1).
- No screenshot standing in for a log.
