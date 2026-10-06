# NATLAS_SETUP — obtaining and serving N-ATLAS legitimately

Everything here is about getting a **genuine** N-ATLAS model running locally. Nothing here
invents an interface: the source, the licence and the access conditions are all stated, with the
evidence used to establish them.

---

## 1. What N-ATLAS is

| Field | Value |
|---|---|
| Name | **N-ATLaS** — the Nigerian Atlas for Languages & AI at Scale |
| Steward | NCAIR (National Centre for AI and Robotics), NITDA, Federal Republic of Nigeria |
| Technical partner | Awarri Technologies |
| Base model | **Llama-3 8B**, supervised fine-tune |
| Languages | Yoruba · Hausa · Igbo · Nigerian-accented English |
| Official source | `https://huggingface.co/NCAIR1/N-ATLaS` |
| Format | safetensors (4 shards, **~15.3 GB** total) |
| Documented context | 8,092 tokens |

---

## 2. The licence — read before downloading

The model is released under the **"Open-Source Research and Innovation License"** (N-ATLaS Terms
of Use v1.0, September 2025), governed by Nigerian law.

**You may:** use, run, modify and redistribute it, for permitted purposes.

**You must:**

1. **Attribute** Awarri Technologies **and** the Federal Ministry of Communications, Innovation
   and Digital Economy.
2. Keep **derivative works under the same licence** (copyleft-style).
3. If you rename a derivative, carry the suffix **"Powered by Awarri"**.
4. Stay within **1,000 active end-users** per organisation (a commercial licence is required
   above that).

**Permitted:** research, accessibility, language preservation, civic tech, education, training,
community innovation.

**Prohibited:** surveillance, discriminatory profiling, disinformation/impersonation/synthetic
fraud, military/intelligence deployment, unlawful use.

> **This prototype is squarely inside the permitted set** and is far below the user cap.
> It is a developer tool. It is not surveillance and not a weapon.

**ZylCode's own source code is Apache-2.0. The model weights are a separate work under the
N-ATLaS licence. Do not relicense the weights, and never commit them to Git.**

---

## 3. Access is gated — the one manual step

Measured behaviour:

```
GET https://huggingface.co/NCAIR1/N-ATLaS/resolve/main/config.json  ->  HTTP 401
GET https://huggingface.co/NCAIR1/N-ATLaS/resolve/main/README.md    ->  HTTP 200
```

The model card is public; the **weights are gated**. You must:

1. Log in to Hugging Face.
2. Open `https://huggingface.co/NCAIR1/N-ATLaS` and **accept the terms**.
3. Authenticate your machine:

```bash
hf auth login
# paste a READ token when prompted — never paste it into source code or a chat
```

Verify:

```bash
hf auth whoami
```

---

## 4. Download the weights

```bash
mkdir -p ~/models/n-atlas && cd ~/models/n-atlas

hf download NCAIR1/N-ATLaS --local-dir .
```

This fetches `config.json`, the tokenizer files and four `.safetensors` shards (~15.3 GB).

> **Alternative (not the default).** An ungated community GGUF quantization exists
> (`QuantFactory/N-ATLaS-GGUF`, Q4_K_M ≈ 4.6 GB). It is a derivative of the same weights and the
> same licence applies. Use it only if you cannot accept the official terms, and disclose it in
> your evidence as a third-party quantization.

---

## 5. Serve it with Ollama

Ollama imports safetensors directly — no conversion tool required.

Create a `Modelfile` next to the weights:

```dockerfile
FROM ./n-atlas

PARAMETER temperature 0
PARAMETER num_ctx 8192
PARAMETER stop <|eot_id|>
```

Then:

```bash
ollama create n-atlas -f Modelfile
ollama list
```

You should see `n-atlas` in the list.

> Keep `num_ctx` at or below the model's documented **8,092** tokens.

---

## 6. Verify — genuinely

```bash
curl http://127.0.0.1:11434/api/tags
```

Then a real completion:

```bash
curl http://127.0.0.1:11434/v1/chat/completions \
  -H "content-type: application/json" \
  -d '{"model":"n-atlas:latest","messages":[{"role":"user","content":"Reply with the single word: ready"}],"stream":false,"temperature":0}'
```

The reply must include a `model` field naming the model. If it does not, the toolkit will
**reject** it — an answer that does not say which model produced it is not evidence.

---

## 7. Hardware guidance

| VRAM | Recommended |
|---|---|
| 0 (CPU only) | works; expect slow generation on an 8B model |
| 4 GB | partial offload; keep most layers on CPU |
| 8 GB+ | comfortable partial offload |
| 24 GB | full offload |

Keep total RAM usage under your system RAM: an 8B model at Q4 needs roughly 7–8 GB at runtime
plus context.

---

## 8. What NOT to do

- Do **not** commit `.safetensors`, `.gguf` or model caches to Git.
- Do **not** paste your Hugging Face token into source code, a Modelfile, or a chat message.
- Do **not** point `NATLAS_BASE_URL` at a different provider and call the result N-ATLAS.
- Do **not** describe a stub or test-double reply as N-ATLAS output.
