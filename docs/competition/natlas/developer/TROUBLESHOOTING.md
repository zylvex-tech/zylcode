# TROUBLESHOOTING

Every problem below is reported honestly by the toolkit. Nothing is smoothed over.

---

## `NOT_CONFIGURED`

**Symptom:** `N-ATLAS is not configured; missing environment variable(s): NATLAS_BASE_URL, ...`

**Cause:** a required variable is absent. There are deliberately **no defaults** — every one is an
interface detail we will not invent.

**Fix:** set the variables.

```bash
export NATLAS_BASE_URL=http://127.0.0.1:11434
export NATLAS_REQUEST_PATH=/v1/chat/completions
export NATLAS_MODEL=n-atlas:latest
export NATLAS_API_KEY=            # may be empty for a local runtime, but must be present
```

---

## `TRANSPORT` — could not reach the runtime

**Symptom:** `could not reach the local N-ATLAS runtime at http://127.0.0.1:11434/...`

**Checks:**

1. Is Ollama running? `ollama list`
2. Does the port answer? `curl http://127.0.0.1:11434/api/tags`
3. Is the URL correct (scheme, host, port)?

> The local transport bypasses any `HTTP_PROXY` on purpose. If you have a proxy set and still see
> this error, the runtime genuinely is not listening.

---

## The runtime answers, but the model is not there

**Symptom:** the probe reports `reachable: true` but `model_present: false`.

**Cause:** the model name does not match.

**Fix:** `ollama list` shows the exact name. If it is `n-atlas:latest`, use that — the probe
accepts either `n-atlas` or a `n-atlas:*` tag.

---

## `HTTP_STATUS 404`

**Cause:** the request path is wrong. The OpenAI-compatible path is `/v1/chat/completions`
(Ollama and llama.cpp). A bare Ollama-native endpoint is `/api/generate`, which is **not** what
this transport speaks.

---

## `HTTP_STATUS 401` / `403`

**Cause:** the runtime requires authentication and the key is missing or wrong.

**Fix:** set `NATLAS_API_KEY`. For a plain local Ollama this is normally unnecessary.

---

## `MALFORMED_RESPONSE`

Several distinct causes, all stated in the detail:

| Detail | Meaning |
|---|---|
| `reply was not valid JSON` | the runtime returned non-JSON (often an HTML error page) |
| `reply has no choices` | not an OpenAI-compatible chat reply |
| `reply content is empty` | the model returned nothing |
| `did not report a model identity` | **the reply is rejected on purpose** — identity cannot be borrowed |

---

## `TIMEOUT`

**Cause:** the request exceeded `NATLAS_TIMEOUT_MS` (default 60 000).

An 8B model on CPU can be slow. Raise the timeout for a large prompt:

```bash
export NATLAS_TIMEOUT_MS=180000
```

---

## The model is very slow

Expected on CPU-only hardware. Options, in order of preference:

1. Reduce the context you send (fewer/smaller excerpts).
2. Lower `num_ctx` in the Modelfile (stay ≤ 8,092).
3. Offload more layers to GPU if you have VRAM.
4. Use a quantized build (Q4_K_M) if you accepted that route.

---

## Hugging Face download fails with 401

**Cause:** the official weights are **gated**.

**Fix:** accept the terms at `https://huggingface.co/NCAIR1/N-ATLaS`, then `hf auth login`.
Confirm with `hf auth whoami`.

---

## `ollama create` fails on safetensors

**Checks:**

1. The `Modelfile` `FROM` path points at the **directory** containing `config.json` and the
   `.safetensors` shards.
2. All four shards are present — a partial download fails.
3. `config.json` is readable (it is not gated once you are authenticated).

---

## The tests fail with a lock-file warning

```
warning: error deleting lock file for incremental compilation session directory ... Access is denied
```

This is an **environmental** Windows issue with the incremental-compilation lock, not a test
failure. It does not affect results.

---

## A router test fails intermittently

`router::tests::d1_vector_cache_cross_prompt_contamination_guard` is a **pre-existing**
intermittent flake caused by a process-global counter shared across parallel tests. It is
documented in `NATLAS_VALIDATION_PROTOCOL_2026-10-06.md` §6.1 and is **not** caused by the
competition work. Run that suite single-threaded to avoid it:

```bash
cargo test -p zylcode-core --lib -- --test-threads=1
```

---

## The playground shows "TEST DOUBLE"

That is the **offline demo**, not N-ATLAS. Press **Check runtime status** to leave demo mode, or
read the badge: a genuine result never carries the `TEST DOUBLE` badge.
