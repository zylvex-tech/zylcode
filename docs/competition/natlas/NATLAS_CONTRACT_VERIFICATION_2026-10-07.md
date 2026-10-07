# N-ATLAS CONTRACT VERIFICATION (Phase 2)

- **Date:** 2026-10-07
- **Method:** primary sources only. Every contract below was read from a shipped artefact
  (a client bundle or a server source file) or observed from a live HTTP exchange. **Nothing is
  inferred from a blog post, a guess, or a convention.**

Classification: `VERIFIED` · `INFERRED` · `UNKNOWN` · `BLOCKED`.

---

## Contract A — Official demo engine (`publicaai`)

**Source of truth:** the shipped client bundle of the official N-ATLAS demo at
`https://n-atlas.ncair.nitda.gov.ng/` → `assets/index-C1pungEH.js` (retrieved 2026-10-07).
This is a primary artefact: the code the official demo actually runs.

| Field | Value | Class |
|---|---|---|
| Base URL | `https://natlas-engine.publicaai.com` | VERIFIED |
| Method / path | `POST /chat/{language}` | VERIFIED |
| `language` ∈ | `english`, `yoruba`, `igbo`, `hausa`, `chat` | VERIFIED |
| Request header | `Content-Type: application/json` | VERIFIED |
| Request body | `{"session_id": "<string>", "message": "<string>"}` | VERIFIED |
| Response body | JSON object; the reply text is the `response` field | VERIFIED |
| Auth | none observed in the client | VERIFIED (none) |
| **Service liveness** | **HTTP 502 on every path** (`/`, `/healthz`, `/chat/*`, `/v1/*`) | **DOWN** |

Extracted client code (verbatim):

```js
yE = "https://natlas-engine.publicaai.com"
// ...
const o = `${yE}/chat/${r}`;
const s = await fetch(o, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ session_id: n, message: e })
});
if (!s.ok) throw new Error(`API request failed with status ${s.status}`);
return (await s.json()).response
// language map: { en:"english", yo:"yoruba", ig:"igbo", ha:"hausa", multi:"chat" }
```

**Status:** contract `VERIFIED`; runtime `BLOCKED_SERVICE_UNAVAILABLE` (502).
The endpoint is currently unusable, but its shape is known and must not be re-invented.

---

## Contract B — OpenAI-compatible N-ATLAS engine (sovereign replica)

**Source of truth:** `app.py` (1 051 lines) of the Space `samuelolubukun/NATLaS-Sovereign-Engine`
(retrieved 2026-10-07), plus a live `/healthz` call. The Space loads the official
`NCAIR1/N-ATLaS` weights in bf16 on ZeroGPU.

| Field | Value | Class |
|---|---|---|
| Base URL | `https://samuelolubukun-natlas-sovereign-engine.hf.space` | VERIFIED |
| Health | `GET /healthz` → `{"status":"healthy","service":"natlas-engine","model":"NCAIR1/N-ATLaS", …}` | VERIFIED (observed 200) |
| Models | `GET /v1/models` | VERIFIED |
| Chat | `POST /v1/chat/completions` — OpenAI schema, SSE streaming, stop sequences, tool calling | VERIFIED (from source) |
| Completions | `POST /v1/completions` | VERIFIED (from source) |
| Audio | `POST /v1/audio/transcriptions` (OpenAI Whisper multipart) | VERIFIED (from source) |
| Auth | `Authorization: Bearer <NATLAS_API_KEY>`; **503 if no key configured, 401 on mismatch** | VERIFIED |
| Observed unauth result | `GET /v1/models` → **401** `Unauthorized: Invalid NATLAS API key or Bearer token.` | VERIFIED (observed) |
| Licence | Apache-2.0 (Space); weights remain under the N-ATLaS licence | VERIFIED |

Observed `/healthz` (abridged):

```json
{"status":"healthy","service":"natlas-engine","model":"NCAIR1/N-ATLaS",
 "engine":"PyTorch-Transformers","gpu":"ZeroGPU (shared NVIDIA GPU)",
 "supported_models":["NCAIR1/Hausa-ASR","NCAIR1/Igbo-ASR","NCAIR1/NigerianAccentedEnglish","NCAIR1/Yoruba-ASR"],
 "attribution":"N-ATLaS is an initiative of the Federal Ministry of Communications, Innovation and Digital Economy, and powered by Awarri Technologies."}
```

**Status:** contract `VERIFIED`; runtime `LIVE` but `BLOCKED_CREDENTIALS` for us (key unknown).
The README documents self-deployment; the Space is duplicable.

---

## Contract C — Public Gradio inference API

**Source of truth:** `GET {space}/gradio_api/info` (declares `api_visibility: "public"`) plus a
live call.

| Field | Value | Class |
|---|---|---|
| Space | `https://calvaryyy-n-atlas-chat.hf.space` | VERIFIED |
| Model | `NCAIR1/N-ATLaS`, `torch.float16`, `device_map="auto"` | VERIFIED (from `app.py`) |
| Submit | `POST /gradio_api/call/generate` body `{"data":["<prompt>"]}` → `{"event_id":"…"}` | VERIFIED |
| Fetch result | `GET /gradio_api/call/generate/{event_id}` → SSE; `event: complete` / `data: ["<text>", null]` | VERIFIED |
| Auth | none (public) | VERIFIED |
| Quota | ZeroGPU anonymous quota; error event when exceeded | VERIFIED (observed) |

**Status:** contract `VERIFIED`; runtime `LIVE` but `RATE_LIMITED` (ZeroGPU quota).

---

## What was NOT invented

- No endpoint was guessed. Every URL above was read from shipped code or observed.
- No auth scheme was assumed. Contract B's scheme is stated in its own source.
- No response schema was assumed. Contract C's SSE shape was observed end-to-end.
- No model id was assumed: `NCAIR1/N-ATLaS` is reported by the server itself, not borrowed
  from configuration.

## Consequence for the transport layer

`LocalNatlasTransport` already speaks **Contract B**'s protocol (`POST /v1/chat/completions`,
OpenAI schema, model identity taken from the reply). It is therefore *not* a "local-only"
transport in principle — it is an **OpenAI-compatible N-ATLAS transport** that can point at any
endpoint implementing Contract B (local Ollama, a remote Space, or a hosted engine). The name is
now a misnomer; the capability is broader than the name. This is recorded as a rename candidate,
**not** silently renamed.
