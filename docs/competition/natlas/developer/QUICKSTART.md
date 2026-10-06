# QUICKSTART — ZylCode × N-ATLAS Developer Bridge

**Goal:** in about 15 minutes, go from nothing to invoking a **real** N-ATLAS model and seeing
a structured engineering intent come back.

> **Read this first.** N-ATLAS is Nigeria's multilingual LLM (`NCAIR1/N-ATLaS`, a Llama-3 8B
> fine-tune). This toolkit does **not** fake it. If your runtime is not reachable, you will see a
> clear failure — never a pretend answer.

---

## What you need

| Requirement | Why |
|---|---|
| Windows 11 / macOS / Linux | the toolkit is cross-platform |
| ~16 GB free disk | the model is ~15 GB (official) or ~4.6 GB (quantized) |
| 16 GB+ RAM (32 GB comfortable) | an 8B model runs in RAM |
| [Ollama](https://ollama.com) installed | serves the model locally |
| Node.js ≥ 18 | for the TypeScript SDK and playground |
| A Hugging Face account | the official weights are **gated** |

---

## The five steps

### 1. Get the model

The official weights are gated. Log in to Hugging Face, accept the terms at
`https://huggingface.co/NCAIR1/N-ATLaS`, then authenticate your machine:

```bash
hf auth login          # paste your read token at the prompt
```

Full detail: **[NATLAS_SETUP.md](NATLAS_SETUP.md)**.

### 2. Serve it locally with Ollama

```bash
# from a directory containing config.json + the .safetensors shards
ollama create n-atlas -f Modelfile
ollama list            # confirm "n-atlas" appears
```

### 3. Confirm the runtime is genuinely up

```bash
curl http://127.0.0.1:11434/api/tags
```

You should see `n-atlas` in the list. **If you do not, stop** — nothing downstream will work,
and nothing downstream should pretend to.

### 4. Invoke it from the SDK

```bash
cd apps/natlas-sdk
node --experimental-strip-types test/smoke.mjs   # 10 checks, no model required
```

Then from your own code:

```ts
import { NatlasBridge, OPENAI_CHAT_PATH, INTENT_SYSTEM_PREAMBLE } from "@zylvex/natlas-bridge";

const bridge = NatlasBridge.configure({
  baseUrl: "http://127.0.0.1:11434",
  requestPath: OPENAI_CHAT_PATH,
  model: "n-atlas:latest",
});

const health = await bridge.checkStatus();
console.log(health.reachable, health.modelPresent);

const { invocation, intent } = await bridge.submitIntent({
  intent: "Add a health-check endpoint to the router and a test for it.",
  system: INTENT_SYSTEM_PREAMBLE,
});
console.log(invocation.evidence, intent);
```

### 5. Or use the playground

Open `apps/natlas-sdk/playground/index.html` in a browser. Enter your runtime URL, press
**Check runtime status**, then **Invoke N-ATLAS**.

---

## What you will see when it works

- the **model identity reported by the server** (not one we assumed);
- a measured latency;
- a structured intent (`summary` + `steps`);
- an approval gate;
- an evidence record with an id.

## What you will see when it does not

A stated failure with a code: `TRANSPORT`, `TIMEOUT`, `HTTP_STATUS`, `MALFORMED_RESPONSE`,
`NOT_CONFIGURED`. **Never** a fabricated success.

---

## Next

| I want to… | Read |
|---|---|
| install properly | [INSTALLATION.md](INSTALLATION.md) |
| understand the model and licence | [NATLAS_SETUP.md](NATLAS_SETUP.md) |
| run the full bounded workflow | [FIRST_WORKFLOW.md](FIRST_WORKFLOW.md) |
| understand the architecture | [ARCHITECTURE.md](ARCHITECTURE.md) |
| know why approval exists | [SECURITY_AND_APPROVALS.md](SECURITY_AND_APPROVALS.md) |
| find the evidence | [EVIDENCE_AND_VALIDATION.md](EVIDENCE_AND_VALIDATION.md) |
| fix a problem | [TROUBLESHOOTING.md](TROUBLESHOOTING.md) |
| beta-test the toolkit | [BETA_TEST_GUIDE.md](BETA_TEST_GUIDE.md) |
