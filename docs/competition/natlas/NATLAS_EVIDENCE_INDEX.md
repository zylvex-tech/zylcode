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
| EV-010 | A Zylvex-controlled N-ATLAS Space can be created (B1 option a) | 2026-10-07 | `bece9e9` | Win11 + local proxy | `POST /api/repos/create` `{type:space,name:natlas-zylcode-bridge}` **and** `hf repo create natlas-zylcode-bridge --repo-type space --space_sdk gradio` | 201 + repo id `zylvex/natlas-zylcode-bridge` | **HTTP 403** "You don't have the rights to create a space under the namespace 'zylvex'"; token `auth.accessToken.role = "read"` (displayName `zylcode-natlas`) | **BLOCKED** | local: `natlas_bridge/create_resp3.json`, `whoami.json` |
| EV-011 | B1 fork artefacts prepared and ready to deploy once a write token exists | 2026-10-07 | `bece9e9` | local | Copied source `app.py` verbatim (Apache-2.0), `requirements.txt`; fetched canonical Apache-2.0 `LICENSE`; generated 32-byte URL-safe `NATLAS_API_KEY` (local gitignored only) | deployable fork | app.py `verify_api_key` reads `os.environ["NATLAS_API_KEY"]` (L470-490: 503 if unset, 401 on mismatch); secret never committed/printed | **READY (not applied)** | local temp `natlas_bridge/` |
| EV-012 | Option 1 (write token) is actually usable in this session | 2026-10-07 | `bece9e9` | Win11 + local proxy | After owner authorised a write credential: re-ran `hf repo create`; probed env + token files for the write token | write token reachable via `HF_TOKEN`/cached file | **403 persists** at that point; only reachable token was `~/.cache/huggingface/token` with `role: read`; no `HF_TOKEN`/`HUGGING_FACE_HUB_TOKEN` env or other token file existed **at that probe**. | **BLOCKED (resolved by EV-013)** | `whoami2.json` + `hf repo create` 403 (session) |
| EV-013 | A Zylvex-*controlled* N-ATLAS endpoint is deployed and serves the genuine gated weights | 2026-10-07 ~19:26 GMT+1 | `bece9e9` | Win11 + local proxy | Space `zylvex/natlas-zylcode-bridge` (ZeroGPU `zero-a10g`) created via REST; Apache-2.0 fork (app.py/requirements.txt/LICENSE/README) deployed via `huggingface_hub` staging commit `a2566d91`; `HF_TOKEN` Space secret set (gated-model download) + `NATLAS_API_KEY` secret set (endpoint auth) | `healthz` 200 naming `NCAIR1/N-ATLaS` | `GET /healthz` → 200, `model: NCAIR1/N-ATLaS`, `service: natlas-engine`, attribution FMCIDE/Awarri; build downloaded + loaded the 15 GB gated model | **PASS** | Space `zylvex-natlas-zylcode-bridge.hf.space`; deploy commit `a2566d91` |
| EV-014 | B1 inference gate: genuine N-ATLAS returns the challenge token through *our* endpoint | 2026-10-07 ~19:27 GMT+1 | `bece9e9` | Win11 + local proxy | `POST /v1/chat/completions` → `zylvex-natlas-zylcode-bridge.hf.space` with `Authorization: Bearer <NATLAS_API_KEY>`, prompt *"Return exactly: NATLAS_ZYLCODE_OK"* | model returns the token | HTTP 200; `choices[0].message.content == "NATLAS_ZYLCODE_OK"` | **PASS (L3 — genuine invocation, real model, no mock)** | `gate.py` run (session terminal); no secret-bearing artefacts committed |
| EV-015 | Genuine multilingual N-ATLAS developer-assistance through our endpoint (C3 seed) | 2026-10-07 ~19:35 GMT+1 | `bece9e9` | Win11 + local proxy | `POST /v1/chat/completions` ×4 (EN/YO/HA/IG) with programming-concept prompts to the controlled endpoint | coherent, language-appropriate answers | EN: Rust trait; YO: 'function' in Yoruba; HA: 'variable' in Hausa (+Python example); IG: 'loop' in Igbo — all HTTP 200, real model output, correct target language | **PASS (L3)** | `c3_multiling.py` run (session terminal) |

> **Deployment method note (EV-013).** `git push` to Hugging Face fails in this sandbox: git's
> HTTPS transport returns no refs through the local proxy (`git ls-remote` empty), even though
> `curl` reaches HF fine. The deploy therefore used the Hugging Face REST API / `huggingface_hub`
> staging protocol instead of git. The Space is git-backed on HF regardless; this is purely a
> local transport workaround.

---

## B1 resolution (recorded 2026-10-07, supersedes the B1 blocker)

B1 option (a) is **COMPLETE**. The owner supplied a **write-role HF token** for `zylvex`; it was
used via the `Authorization` header (never printed, never committed). The credential gate
(`role: write`, identity `zylvex`) passed. Controlled Space `zylvex/natlas-zylcode-bridge` was
created (ZeroGPU `zero-a10g`), the Apache-2.0 fork was deployed (commit `a2566d91`), and the
`HF_TOKEN` + `NATLAS_API_KEY` secrets were applied. The build downloaded and loaded the gated
`NCAIR1/N-ATLaS` weights; `healthz` returns 200; the B1 inference gate (EV-014) returned the
challenge token from the real model. See `B1_STATUS_2026-10-07.md` for the full item-by-item update.

**Residual risk (unchanged):** the endpoint runs on shared ZeroGPU. Sustained/repeatable calls are
quota-limited (cf. EV-002/003/004 on the public engine). One genuine round trip is now recorded
against *our own* endpoint; for demo-day repeatability we may need to re-warm the Space or fall
back to B1(c) public path. This is a scheduling risk, not a capability gap.

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
- No claim of sustained N-ATLAS availability. A genuine round trip is now recorded against *our own*
  controlled endpoint (EV-014, L3), but it runs on shared ZeroGPU — repeatable/demo-day calls are
  quota-limited (cf. EV-002/003/004). This is a scheduling risk, not a capability gap.
- No screenshot standing in for a log.
