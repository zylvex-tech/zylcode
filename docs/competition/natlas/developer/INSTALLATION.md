# INSTALLATION

How to install the ZylCode × N-ATLAS Developer Bridge toolkit on your machine.

---

## 1. Prerequisites

| Tool | Version | Check |
|---|---|---|
| Rust | 1.97+ | `rustc --version` |
| Node.js | ≥ 18 | `node --version` |
| pnpm | ≥ 9 | `pnpm --version` |
| Ollama | latest | `ollama --version` |
| Hugging Face CLI | latest | `hf --version` |

Install the Hugging Face CLI if you do not have it:

```bash
pip install -U "huggingface_hub[cli]"
```

---

## 2. Get the repository

```bash
git clone https://github.com/zylvex-tech/zylcode.git
cd zylcode
git checkout competition/natlas-2026
```

> The competition work lives on `competition/natlas-2026`. If the branch is not yet published,
> use the artefact your organiser provided.

---

## 3. Build the Rust core

```bash
cargo build -p zylcode-core
```

Run the competition test suites to confirm your build is sound:

```bash
cargo test -p zylcode-core --lib competition::
cargo test -p zylcode-core --test natlas_boundary
cargo test -p zylcode-core --test natlas_runtime
```

All three should pass. They do **not** require a model — they use test doubles and a local stub
server, and they are labelled as such.

---

## 4. Install the TypeScript SDK

The SDK lives at `apps/natlas-sdk` and is part of the pnpm workspace.

```bash
pnpm install
cd apps/natlas-sdk
../../node_modules/.bin/tsc --noEmit      # typecheck
node --experimental-strip-types test/smoke.mjs
```

The smoke test should print `10 passed, 0 failed`. It uses an in-process stub server — again,
**not** N-ATLAS.

---

## 5. Get and serve the model

This is the only step that needs the network and your Hugging Face account. Follow
**[NATLAS_SETUP.md](NATLAS_SETUP.md)**.

---

## 6. Verify the installation

```bash
# 1. the runtime answers
curl http://127.0.0.1:11434/api/tags

# 2. the SDK sees it
cd apps/natlas-sdk
node --experimental-strip-types -e "
import('./src/index.ts').then(async (m) => {
  const b = m.NatlasBridge.configure({ baseUrl:'http://127.0.0.1:11434', requestPath:m.OPENAI_CHAT_PATH, model:'n-atlas:latest' });
  console.log(await b.checkStatus());
});"
```

---

## Disk and memory budget

| Item | Size |
|---|---|
| Repository + Rust `target/` | ~2–4 GB |
| Node `node_modules` | ~200 MB |
| Official N-ATLAS safetensors | **~15.3 GB** |
| …or quantized GGUF (Q4_K_M) | ~4.6 GB |

Keep at least **20 GB free** for the official route.

---

## Uninstalling

```bash
ollama rm n-atlas          # removes the model from Ollama
rm -rf path/to/safetensors # removes the downloaded weights
```

The repository itself is disposable — nothing outside it is modified except the Ollama model you
explicitly created.
