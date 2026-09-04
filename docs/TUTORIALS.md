# Tutorials — End-to-End

## 01 — Provider Setup (5 min)

**Goal:** Get Anthropic + OpenRouter + local Ollama into the fallback chain.

### Prerequisites

```bash
cargo --version # 1.77+
pnpm --version  # 9+
```

### 1) Env

```bash
export ANTHROPIC_API_KEY=sk-ant-…
export OPENROUTER_API_KEY=sk-or-…
# Ollama — start daemon
ollama serve
ollama pull llama3.1
```

Alternatively set `extra` in `EngineConfig`:

```rust
let mut extra = HashMap::new();
extra.insert("anthropic_api_key".into(), sk.clone());
ZylCodeEngine::new(EngineConfig { workspace_root: ".".into(), verbose: false, extra });
```

### 2) Verify routing

```bash
cargo test -p zylcode-core router -- --nocapture
pnpm --filter zylcode-desktop dev
# Type any prompt → Run → check TokenMetricsWidget fallback ×N
```

### 3) Override at invoke time

```ts
await invoke<IntentResult>("process_intent", { prompt: "build a todo app", model: "anthropic/claude-3.5-sonnet" });
```

Model override is per-call, ephemeral.

---

## 02 — Failover Testing (10 min)

**Goal:** Trigger and observe `telemetry:fallback` + `telemetry:provider_failover`.

### 1) Force retryable failure

Set an invalid key but valid endpoint → 401 → retryable → fallback.

```bash
export ANTHROPIC_API_KEY=invalid
export OPENROUTER_API_KEY=invalid   # keep Ollama running
cargo test -p zylcode-core router::tests::synthetic_offline_dispatch_returns_parseable_payload -- --nocapture
# logs: telemetry:fallback {provider=anthropic} → telemetry:provider_failover {from=anthropic,to=ollama}
```

### 2) Frontend indicators

`apps/zylcode-desktop/src/components/ProviderSettings.tsx` subscribes:

```ts
await listen<Failover>("telemetry:provider_failover", (ev) => setFailovers(p => [ev.payload, ...p]));
```

Trigger via `Stream` button; failover list populates with `↻ #1 anthropic → ollama`.

### 3) Disable Ollama mid-chain

```ts
await invoke("set_provider_config", { kind: "ollama", enabled: false });
```

Fallback skips disabled? Current dispatch uses `primary_provider`/`fallback_provider` pair; `enabled` gate is UI-level source of truth—custom chains in next step.

### 4) Reorder to local-first

```ts
await invoke("reorder_provider_chain", { order: ["ollama","anthropic","open_router","synthetic_offline"] });
await invoke("get_provider_configs").then(cs => console.table(cs.map(c => [c.kind, c.fallback_order])));
```

---

## 03 — Compression Tuning (15 min)

**Goal:** Keep large prompts under `context_window_tokens` budget.

### 1) Measure baseline

```rust
let c = ContextCompressor::new(8192);
let prompt = std::fs::read_to_string("crates/zylcode-core/src/router.rs").unwrap();
let (p, s, m) = c.compress(&prompt, "You are a code assistant.");
println!("{:.2}% {}/{} {}", m.compression_ratio*100.0, m.compressed_tokens, m.original_tokens, m.strategy);
```

### 2) Isolate strategies

```rust
let strip = LosslessCommentsStripper.compress(src, 9999);
let outline = ASTOutlineExtractor.compress(&strip, 500);
let window = TokenWindowCompactor.compress(&outline, 200);
```

### 3) Hook into dispatch

Already active: `router.rs:dispatch_prompt` checks `est(prompt)+est(system) > budget` → `ContextCompressor::new(budget).compress`. Tail traces:

```bash
cargo run -- -i  # then type prompt
# logs: telemetry:compression original_tokens=… compressed_tokens=… compression_ratio=…
RUST_LOG=info cargo test -- --nocapture 2>&1 | grep telemetry:compression
```

### 4) Budget sweeps

```rust
for budget in [256, 512, 1024, 4096, 8192] {
  let (p, _, m) = ContextCompressor::new(budget).compress(&big_prompt, &system);
  assert!(ContextCompressor::estimate_tokens(&p) + ContextCompressor::estimate_tokens(&s) <= budget + slack);
}
```

---

## 04 — Building Extensions & MCP Tools (10 min)

### 1) Marketplace registration

```ts
import { ExtensionRegistry } from "zylcode-core/marketplace";
const reg = new ExtensionRegistry();
reg.register({ id: "my-plugin", /* PluginManifest */ });
await invoke("marketplace_search", { query: "my-plugin" });
```

### 2) MCP tool yaml

`mcp.tools.yaml` in root:

```yaml
tools:
  - id: fs_search
    command: node
    transport: stdio
    enabled: true
    description: "Search workspace"
```

Engine hot-reloads via `notify` → `ZylCodeEngine::watch_tools_config`.

### 3) Provider-aware tool

Tool that selects provider based on task:

```rust
let cfg = RouterConfig { primary_provider: ModelProvider::Anthropic, ..Default::default() };
engine.with_router_config(EngineConfig::default(), cfg)?.process_intent(intent).await?;
```

---

## Exercises

1. Add a new `ProviderKind::AzureOpenAI` — implement `default_base_url`, `api_key_for_kind` (`AZURE_OPENAI_KEY`), update `default_provider_configs`, add `KIND_LABEL/DOT`, test fallback.
2. Replace `ASTOutlineExtractor` with `syn`-backed parser that truly parses Rust and keeps `#[tauri::command]` functions verbatim.
3. Persist provider chain to SQLite (`provider_configs` table) and rehydrate into `EngineState` on boot.
