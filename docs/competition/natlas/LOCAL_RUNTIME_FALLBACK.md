# LOCAL RUNTIME FALLBACK — when the shared GPU quota is exhausted

**Status:** PROCEDURE (documented, not yet executed end-to-end on this machine)
**Audience:** a developer or tester who hit `QUOTA` on the live Space
**Related defect:** NAT-A-001 (ZeroGPU quota exhaustion), NAT-A-003 (human-readable errors)

---

## Why this document exists

The live demonstration endpoint runs on Hugging Face **ZeroGPU**, which is a
*shared, quota-metered* pool. When the pool's budget for the day is spent, the
endpoint answers with an HTTP `503` whose body names the quota (for example,
*"You have exceeded your GPU quota"*).

That is **not a bug in ZylCode and not a bug in N-ATLAS**. It is a statement
about a shared resource. The client classifies it as `QUOTA` and — deliberately
— does **not** retry, because retrying cannot create budget that does not exist.

The message a tester now sees is:

> The endpoint's shared GPU quota is exhausted. No code change fixes this — wait
> for the quota to reset, or run the bridge against a local OpenAI-compatible
> runtime (see this document).

This document is the second half of that sentence.

---

## What "local runtime fallback" means — and does not mean

It means: point the **same client**, over the **same `NatlasTransport` seam**,
at a runtime on your own machine that speaks the OpenAI-compatible
chat-completions protocol.

It does **not** mean substituting a different model and calling the result
N-ATLAS. If you run a local runtime with a **non-N-ATLAS** model, every result
is that model's output, and it must be labelled as such — it is **not**
competition evidence for N-ATLAS integration. The fallback exists to keep the
*harness* exercisable while the shared quota resets, not to manufacture a
N-ATLAS result.

> **Rule.** A local run against a non-N-ATLAS model is evidence about ZylCode's
> pipeline. It is never evidence about N-ATLAS. Do not record it as such.

---

## Prerequisites

- A local runtime exposing an OpenAI-compatible endpoint. Both of these work:
  - **Ollama** — default at `http://127.0.0.1:11434`, exposes `/api/tags` and
    `/v1/chat/completions`.
  - **llama.cpp server** (`llama-server`) — exposes `/v1/chat/completions` and
    `/v1/models`.
- A model pulled into that runtime. Name it exactly as the runtime reports it
  (see step 2 — do not guess the name).

---

## Procedure

### 1. Confirm the runtime answers

```bash
# Ollama
curl -s http://127.0.0.1:11434/api/tags | head
# OpenAI-compatible
curl -s http://127.0.0.1:11434/v1/models | head
```

### 2. Point the client at it — by environment variable

The Rust boundary reads these variables; there are **no invented defaults**:

```bash
export NATLAS_BASE_URL=http://127.0.0.1:11434
export NATLAS_REQUEST_PATH=/v1/chat/completions
export NATLAS_MODEL=<the exact name the runtime reported in step 1>
export NATLAS_API_KEY=            # leave empty for a local runtime
export NATLAS_TIMEOUT_MS=120000   # optional; defaults to 60000
```

The JavaScript SDK reads the **same** names via `configFromEnv()`, and the
browser playground exposes the same four fields in its *Runtime configuration*
panel (its default is already `http://127.0.0.1:11434`).

### 3. Probe, then invoke

```bash
# Rust: probe + a hermetic boundary run
cargo test -p zylcode-core --test natlas_runtime
```

```bash
# JavaScript SDK
node --experimental-strip-types apps/natlas-sdk/test/smoke.mjs
```

Or open `apps/natlas-sdk/playground/index.html` in a browser, press
**Check runtime status**, then **Invoke N-ATLAS**. The runtime status panel
shows exactly which model the server reported — the page never borrows the
configured name.

---

## What you will observe

| State | Meaning | What to do |
|---|---|---|
| `OK` | The runtime answered and the model reported its identity | Proceed |
| `WARMING` / `LOADING` | The runtime is starting up | Wait; retry |
| `QUOTA` | Shared GPU budget spent (live Space only) | Use this fallback, or wait for reset |
| `AUTH_FAILURE` | 401/403 | Check `NATLAS_API_KEY` |
| `UNAVAILABLE` | Nothing answered (DNS, refused, proxy) | Check the base URL and proxy settings |
| `FAILED` | A non-transient error | Inspect the recorded body excerpt |

---

## Honest status of this document

- The **classification** described here is implemented and tested:
  `NatlasResilienceState::from_http` (Rust), `classifyHttp` (TypeScript), and
  the playground's `classifyHttp` — each covered by a regression test that
  proves a quota `503` is classified `QUOTA` and **not** `WARMING`.
- The **end-to-end local-runtime procedure above has not been executed on this
  machine as part of Track A remediation.** It is written from the transports'
  actual configuration surface. Treat it as a documented procedure, not as a
  captured result, until someone runs it and records the output.
