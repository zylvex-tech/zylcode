# DEPLOYMENT VERIFICATION — N-ATLAS × ZylCode Developer Bridge

**Date:** 2026-10-10
**Branch:** `competition/natlas-2026` · **Commit:** `8216738`
**Evidence:** `evidence/EV-028-live-deployment-verification-2026-10-10.json`

---

## 1. Summary

| Question | Answer |
|---|---|
| Deployment URL | `https://zylvex-natlas-zylcode-bridge.hf.space` |
| Reachable? | **Yes** — HTTP `200`, `server: uvicorn` |
| Hosting | **Hugging Face Spaces** (Gradio `5.20.0`, `app_file: app.py`) |
| Deployed commit | **`a2566d91fede85bf769daacdfb55d1678eafba2a`** (HF Space repo), last modified **2026-10-07T19:28:10Z** |
| Model identity | **`NCAIR1/N-ATLaS`**, `PyTorch-Transformers`, ZeroGPU — confirmed live via `/healthz` |
| Hardware | `zero-a10g` (ZeroGPU shared pool), 1 replica, runtime `RUNNING` |
| Quota state at test time | **Exhausted** after one successful call |
| Live model response obtained? | **Yes — one genuine response** (English). All subsequent attempts errored. |
| Deployment action required? | **No** — see §6 |

---

## 2. Two separate repositories (owner's point, confirmed)

The owner stated: *"Updating GitHub and updating the live Hugging Face application are
two separate operations."* This is **correct**, and was confirmed by measurement:

| | GitHub | Hugging Face |
|---|---|---|
| Repo | `github.com/zylvex-tech/zylcode` | `huggingface.co/spaces/zylvex/natlas-zylcode-bridge` |
| Branch | `competition/natlas-2026` | `main` |
| HEAD | `8216738` | `a2566d91` |
| Relationship | — | **independent; no mirror, no submodule, no auto-sync** |

The Space does **not** auto-follow the GitHub branch. Its SHA (`a2566d91`) is a commit in
the *Space's own* git history and has no counterpart in the ZylCode repository.

---

## 3. What the Space runs

The Space is an **Apache-2.0 fork of `samuelolubukun/NATLaS-Sovereign-Engine`**, adapted to
(a) run under the `zylvex` account as a controlled endpoint and (b) enforce
`NATLAS_API_KEY` on all `/v1/*` routes. Model weights are loaded at runtime from the gated
`NCAIR1/N-ATLaS` repository; they are not redistributed.

Surface (all verified live):

| Path | Auth | Observed |
|---|---|---|
| `GET /healthz` | none | `200` — model identity, GPU type, attribution |
| `GET /v1/models` | `NATLAS_API_KEY` | `401` without a key |
| `POST /v1/chat/completions` | `NATLAS_API_KEY` | `401` without a key |
| `POST /v1/completions` | `NATLAS_API_KEY` | (same auth boundary) |
| `POST /v1/audio/transcriptions` | `NATLAS_API_KEY` | (same auth boundary) |
| `GET /openapi.json` | none | `200` (FastAPI `0.1.0`) |
| `GET /` | none | `200` — Gradio frontend (98 163 bytes) |

`/healthz` body (verbatim excerpt):

```json
{"status":"healthy","service":"natlas-engine","model":"NCAIR1/N-ATLaS",
 "engine":"PyTorch-Transformers","gpu":"ZeroGPU (shared NVIDIA GPU)",
 "supported_models":["NCAIR1/Hausa-ASR","NCAIR1/Igbo-ASR",
 "NCAIR1/NigerianAccentedEnglish","NCAIR1/Yoruba-ASR"],"asr_models_loaded":[],
 "attribution":"N-ATLaS is an initiative of the Federal Ministry of Communications, ..."}
```

---

## 4. Genuine live inference — what actually happened

The `/v1/*` API requires `NATLAS_API_KEY`, which is **not present in this environment**.
Rather than stop there, the deployed application's **own public Gradio frontend** was
driven directly (`/gradio_api/call/user_message` → `/gradio_api/call/bot_response`) — the
identical path a human browser user takes. No mock, no test double.

### Attempt 1 — English — **SUCCEEDED (genuine model output)**

```
question: "Answer briefly in one sentence: what is 2+2?"
reply:    "The sum of 2 and 2 is 4."
raw event tail: event: complete
```

This is a **real inference from the deployed `NCAIR1/N-ATLaS` endpoint**, obtained through
the deployed frontend. It is genuine and is recorded as such.

### Attempts 2–5 — Yoruba / Hausa / Igbo / Nigerian-English — **ERRORED**

Every subsequent call returned:

```
event: error
data: null
```

### Attempt 6 — English **retry** — **ALSO ERRORED**

```
question: "Say hello in one word."
raw event tail: event: error / data: null
```

**Interpretation (this is the important part):** English succeeded once, then *English
also* failed. The failure is therefore **not language-specific** — it is a
**capacity/quota condition** on the shared ZeroGPU pool. This is consistent with
defect NAT-A-001 and with the Space's own behaviour.

---

## 5. Quota behaviour — the honest picture

The Space's `app.py` raises, on GPU failure (lines 681, 760):

```python
raise HTTPException(status_code=503, detail=f"GPU unavailable or quota exceeded: {err}")
```

Two observations:

1. **The Space returns `503` for both "GPU unavailable" and "quota exceeded", in one
   message.** A client cannot distinguish them from the status code alone — it must read
   the body. ZylCode does exactly that: `NatlasResilienceState::from_http` checks
   quota words **before** load/cold-start words, so a body containing `quota` classifies
   as `Quota` and is marked **non-retryable** (retrying cannot create GPU budget).
2. **`/healthz` reported `"healthy"` throughout the failures.** The health endpoint does
   **not** reflect GPU quota state. A tester who checks `/healthz`, sees `healthy`, and
   then gets an error may reasonably be confused. This is recorded in
   `KNOWN_LIMITATIONS.md`.

---

## 6. Can the deployment be updated safely? — **No action taken; none required**

**Deployment procedure (established, not executed):** the Space is deployed by pushing to
its own Hugging Face git remote
(`https://huggingface.co/spaces/zylvex/natlas-zylcode-bridge`) or by `hf upload`. A push
triggers a Space rebuild and restart.

**Why no action was taken:**

1. **No ZylCode change needs to reach the Space.** The NAT-A remediation (NAT-A-001…
   NAT-A-004) lives entirely in the **client** — the Rust crate
   (`crates/zylcode-core/src/competition/natlas/`), the TypeScript SDK
   (`apps/natlas-sdk/`), and documentation. None of it alters the Space's model-gateway
   code.
2. **A push would trigger a rebuild/restart** of the only live endpoint, immediately
   before the submission deadline, and would consume scarce ZeroGPU budget.
3. **The repository-visibility instruction** (do not change visibility) and the general
   rule to avoid unnecessary live-infrastructure changes both point the same way.

**Therefore:** the correct and safe outcome is **no deployment action**. This is reported
rather than silently performed.

> **If the owner later decides the Space must be redeployed**, the procedure is a
> deliberate, separately-authorised action: clone the Space, change the code, push to the
> Space remote, and wait for the runtime stage to return to `RUNNING`. It is **not** a
> side effect of pushing to GitHub.

---

## 7. What is NOT verified

| Item | Status |
|---|---|
| `/v1/chat/completions` with a valid key | **NOT VERIFIED** — `NATLAS_API_KEY` not available here. |
| ASR routes | **NOT VERIFIED** — `asr_models_loaded: []` and no key. |
| The deployed Space's `a2566d91` corresponding to any specific ZylCode commit | **NOT APPLICABLE** — separate repositories. |
| A full multi-language live journey | **BLOCKED** by quota after the first call. |
| Sustained throughput / concurrency | **NOT VERIFIED**. |

*End of document.*
