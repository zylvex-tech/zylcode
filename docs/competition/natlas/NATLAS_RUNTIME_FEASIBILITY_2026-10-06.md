# N-ATLAS Runtime Feasibility Report — Phase C1

- **Date:** 2026-10-06
- **Branch:** `competition/natlas-2026`
- **Author:** ZylForge (implementation agent)
- **Status of the runtime question:** **`BLOCKED_NATLAS_ACCESS` — one owner action required.**
- **Satisfies:** GATE C1-A (authoritative source identified) ✅ · GATE C1-B (licence/access understood) ✅ · GATE C1-C (real invocation) ⛔

> Everything below was measured on 2026-10-06 from authoritative sources or from the owner's
> machine. Nothing here is inferred, and nothing that was not run is described as having run.

---

## 1. Executive summary

N-ATLAS is **real, public, and locally runnable**. The authoritative distribution is
`NCAIR1/N-ATLaS` on Hugging Face — an official **Llama-3 8B fine-tune** covering exactly the four
languages the competition targets (Yoruba, Hausa, Igbo, Nigerian-accented English).

Two runtime routes were investigated:

| Route | Verdict |
|---|---|
| **A — official inference API** | **`BLOCKED_NATLAS_API_ACCESS`** — no documented public inference API or contract was found. |
| **B — official model executed locally** | **VIABLE.** Fits the owner's hardware with quantization. **Blocked on one owner action**: the official Hugging Face repo is *gated* and requires the owner's Hugging Face login + terms acceptance. |

Per the owner's decision (2026-10-06), the selected route is **B — official gated weights executed
locally via Ollama**.

**Nothing was downloaded.** The gate was reached and reported rather than assumed. See §8.

---

## 2. Route A — official N-ATLAS inference API

**Finding: no documented public inference API is available to this project.**

- The NCAIR site (`ncair.nitda.gov.ng/llm/`) presents N-ATLAS as an open-sourced model for
  researchers and innovators; it does **not** publish an API endpoint, authentication scheme,
  request/response schema, or a developer API key process.
- A public web presence exists at `n-atlas.ncair.nitda.gov.ng`, which is a **chat demonstration**,
  not a documented developer API.
- No official SDK, OpenAPI document, or API reference was found.

**Therefore no endpoint, path, auth scheme, model id or schema has been invented.** Route A is
recorded as **`BLOCKED_NATLAS_API_ACCESS`**.

> If the owner holds an official API contract or credential (e.g. issued through the competition),
> supplying `NATLAS_BASE_URL`, `NATLAS_REQUEST_PATH`, `NATLAS_MODEL` and `NATLAS_API_KEY`
> externally is all that is required — the boundary already accepts them.

---

## 3. Route B — official N-ATLAS model executed locally

### 3.1 Authoritative source

| Field | Value | How established |
|---|---|---|
| Organisation | `NCAIR1` — National Centre for Artificial Intelligence and Robotics | Hugging Face model card |
| Model repository | `NCAIR1/N-ATLaS` | `https://huggingface.co/NCAIR1/N-ATLaS` |
| Human name | N-ATLaS-LLM (Multilingual African Language Model) | model card |
| Base model | **Llama-3 8B**, fine-tuned (SFT) | model card |
| Revision observed | `e294476928aca9030e924ca27bb8e085e8581273` | HF API `sha` |
| Last modified | 2025-09-23 | HF API `lastModified` |
| Downloads / likes | 1,719 / 125 | HF API |

### 3.2 Architecture (from the model card)

| Parameter | Value |
|---|---|
| Model type | `LlamaForCausalLM` |
| Hidden size | 4,096 |
| Intermediate size | 14,336 |
| Layers | 32 |
| Attention heads | 32 |
| Key/value heads | 8 (GQA) |
| Head dimension | 128 |
| Vocabulary size | 128,256 |
| Max position embeddings | 131,072 |
| **Documented context length** | **8,092 tokens** |
| Training | SFT, AdamW 8-bit, lr 1e-5, ~392M multilingual instruction tokens |

### 3.3 Languages

Yoruba · Hausa · Igbo · Nigerian-accented English — **the exact four competition targets**.
The model card notes Yoruba is under "ongoing improvements" and lists dialectal/accent bias and
limited code-switching as known limitations. These are the model's own stated limits and are
carried into our multilingual matrix rather than hidden.

### 3.4 Distribution and gating — **measured**

```
HF API  NCAIR1/N-ATLaS        -> "gated": "auto",  "private": false,  tags: [safetensors, llama]
GET     .../resolve/main/README.md   -> HTTP 200   (model card is public)
GET     .../resolve/main/config.json -> HTTP 401   (weights are gated)
```

| Fact | Value |
|---|---|
| Gating | **`auto`** — requires an authenticated Hugging Face account that has accepted the terms |
| Format | **safetensors only** in the official repo (no GGUF) |
| Total weight size | **15,333 MB (~15.0 GiB)** across 4 shards |

Official files (HF API, `?blobs=true`):

| File | Size |
|---|---|
| `model-00001-of-00004.safetensors` | 4,746.1 MB |
| `model-00002-of-00004.safetensors` | 4,768.2 MB |
| `model-00003-of-00004.safetensors` | 4,688.2 MB |
| `model-00004-of-00004.safetensors` | 1,114.0 MB |
| `tokenizer.json` | 16.4 MB |
| `config.json`, `generation_config.json`, `tokenizer_config.json`, index | small |

### 3.5 Licence — "Open-Source Research and Innovation License" (N-ATLaS Terms of Use v1.0, Sept 2025)

| Aspect | Term |
|---|---|
| Character | Permissive-style (inspired by Apache-2.0 / MIT) **with additional restrictions** |
| Governing law | Federal Republic of Nigeria |
| Attribution | **Required**: Awarri Technologies **and** the Federal Ministry of Communications, Innovation and Digital Economy |
| Derivative works | Must carry the **same licence** (copyleft-style) |
| Renaming | A renamed derivative must carry the suffix **"Powered by Awarri"** |
| **End-user cap** | **≤ 1,000 active end-users** per organisation/project (rolling 30 days); above that a commercial licence is required |
| Permitted use | Academic/non-profit research; accessibility; language & cultural preservation; civic tech / public benefit; education, training, community innovation |
| Prohibited use | Surveillance; discriminatory profiling; disinformation/impersonation/synthetic fraud; military/intelligence/weaponised deployment; unlawful applications |
| Warranty | As-is, no warranty; Nigerian jurisdiction, mediation first |

**Assessment for this competition prototype:** the intended use (developer tooling, education,
public benefit, well under 1,000 users) falls squarely inside the **permitted** set. The
obligations we must honour are **attribution, same-licence derivatives, and the user cap** — all
recorded in §7.

### 3.6 Community GGUF mirror (alternative, not selected)

`QuantFactory/N-ATLaS-GGUF` — a llama.cpp quantization of `NCAIR1/N-ATLaS`.

| Fact | Value |
|---|---|
| Gating | **`gated`: false** (ungated) |
| Quantizations | Q2_K … Q8_0 |
| **Q4_K_M** | **4,692.8 MB (~4.6 GiB)** |
| Q5_K_M | 5,467.4 MB |
| Q8_0 | 8,145.1 MB |

The owner selected the **official gated weights** route, so this mirror is recorded as an
available fallback and **was not used**.

---

## 4. Owner hardware feasibility — measured

| Resource | Measured | Consequence |
|---|---|---|
| CPU | Intel, **12 logical cores** | CPU inference is viable for an 8B model |
| System RAM | 32 GB (owner-stated) | FP16 (15.3 GB) fits; Q4 (4.6 GB) is comfortable |
| GPU | **Quadro P520, 4,096 MiB (4 GB) VRAM**, driver 582.42 | Partial offload only — a handful of layers; mostly CPU inference |
| Free disk | **67 GB** on `C:` (931 GB volume, 93% used) | 15.3 GB download fits with headroom; leaves ~51 GB |
| Existing runtime | **Ollama 0.32.4 installed** (not running), model store present | No new inference stack needed |
| `llama.cpp` | **not installed** | Not required — Ollama handles safetensors import |
| Python | `torch` present; `transformers`, `huggingface_hub` **absent** | Transformers route would need new installs |

### 4.1 Memory / disk requirement table

| Configuration | Weights | Approx. runtime RAM (8k ctx) | Fits 32 GB? | Fits 4 GB VRAM? |
|---|---|---|---|---|
| Official FP16 safetensors | 15.3 GB | ~18–20 GB | yes | no (partial offload only) |
| GGUF Q4_K_M | 4.6 GB | ~7–8 GB | yes, comfortably | partially |

> **Throughput has NOT been measured.** No model has been downloaded or run. Any tokens/second
> figure would be a guess, so none is given here.

### 4.2 Recommended runtime configuration

- **Runner:** Ollama (already installed). Ollama imports safetensors directly via a `Modelfile`
  with `FROM <directory containing config.json + *.safetensors>` — no llama.cpp conversion step.
- **Serving:** Ollama exposes an **OpenAI-compatible** endpoint at
  `http://127.0.0.1:11434/v1/chat/completions` (documented protocol — see §5).
- **Context:** keep within the model's documented **8,092-token** limit.
- **Offload:** a small number of layers to the 4 GB GPU; the remainder on CPU.

---

## 5. The protocol the local transport speaks — and why it is not an invention

`LocalNatlasTransport` (implemented this phase) speaks the **OpenAI-compatible chat-completions
protocol** to a local server. This is deliberate and honest:

- The protocol is **publicly documented and widely implemented** (Ollama, llama.cpp
  `llama-server`, vLLM, and others). It is not an N-ATLAS interface detail.
- The **request/response schema we parse remains ZylCode's own** (the canonical shape from Phase
  C0). The transport only *translates* the local server's documented reply into that shape.
- **No N-ATLAS endpoint, auth scheme, model id or remote schema is invented.** The only thing
  configured is where the local server listens.

The remote **N-ATLAS API** contract remains unknown and is therefore **not** implemented
speculatively — `HttpNatlasTransport` is provided as a configurable adapter whose contract is
explicitly **UNVERIFIED** until official documentation exists.

---

## 6. Gate status

| Gate | Description | State |
|---|---|---|
| **C1-A** | Authoritative N-ATLAS source/runtime identified | ✅ `NCAIR1/N-ATLaS` (official), Llama-3 8B |
| **C1-B** | Licence/access conditions understood | ✅ read; obligations recorded (§7) |
| **C1-C** | At least one REAL N-ATLAS invocation captured | ⛔ **blocked — owner HF authentication required** |
| **C1-D** | Provider/model/runtime identity captured | ⛔ depends on C1-C |
| **C1-E** | No synthetic fallback involved | ✅ by construction (transport layer has no fallback path) |

---

## 7. Licence obligations this repository must honour

1. **Attribution** to Awarri Technologies and the Federal Ministry of Communications, Innovation
   and Digital Economy, wherever N-ATLAS is used or described.
2. **Derivative works** of the model carry the same licence.
3. **End-user cap** of 1,000 active users — the prototype is far below this.
4. **Do NOT commit model weights** to Git (`.gguf`, `.safetensors` are gitignored).
5. **Do NOT relicense** the weights under ZylCode's Apache-2.0 licence — they are separate works.
6. Report accuracy, bias and limitations transparently.

---

## 8. Owner action required — the exact gate

The official weights cannot be fetched without an authenticated Hugging Face account that has
accepted the N-ATLaS terms. Measured evidence: unauthenticated `config.json` returns **HTTP 401**,
and this machine has **no `HF_TOKEN`, no `~/.cache/huggingface/token`** (though the `hf` CLI is
installed).

**Required owner action (one of):**

1. Log in to Hugging Face and **accept the terms** on `https://huggingface.co/NCAIR1/N-ATLaS`, then
   provide a **read token** to this machine via the standard mechanism — e.g. run
   `hf auth login` and paste the token at the prompt, or export `HF_TOKEN` in the environment.

   **Do not paste the token into source code or into the chat.** Use the CLI prompt or an
   environment variable.

2. Or authorise the fallback route: the ungated community GGUF mirror
   (`QuantFactory/N-ATLaS-GGUF`, Q4_K_M ≈ 4.6 GB), which requires no Hugging Face login.

Once either is in place, the download, the local model creation, and the first genuine invocation
can proceed with no further architectural change.

---

## 9. Recommendation

1. **Take owner action in §8** (official weights + `hf auth login`), as chosen.
2. Then: download the official safetensors (~15.3 GB), `ollama create n-atlas` from the model
   directory, and run the **first genuine invocation** to close GATE C1-C.
3. Keep the model files **out of Git**; record attribution in the README and evidence.
4. Treat the remote API as `BLOCKED_NATLAS_API_ACCESS` unless official credentials arrive.

---

## 10. What was NOT done, and why

| Not done | Reason |
|---|---|
| Downloaded any model weights | §5 of the directive: download only after source, licence, disk, RAM/VRAM and a documented recommendation — and the official route additionally needs the owner's HF authentication. |
| Contacted any N-ATLAS endpoint | Route A has no documented contract; inventing one is forbidden. |
| Claimed any runtime success | No genuine invocation has occurred. |
| Measured throughput | No model has been run. |
