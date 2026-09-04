# Providers — Multi-Model Routing & Failover (Phase 7.2 + 7.3)

## ProviderKind

```rust
pub enum ProviderKind {
    Anthropic,        // https://api.anthropic.com → /v1/messages, x-api-key
    OpenRouter,       // https://openrouter.ai/api/v1 → /chat/completions, Bearer + referer headers
    Ollama,           // http://localhost:11434 → /api/chat, no key
    SyntheticOffline, // "" — deterministic XML, no egress
}
impl ProviderKind {
    fn default_base_url(&self) -> &'static str { … }
    fn completions_path(&self) -> &'static str { … }
}
```

### ProviderConfig

```rust
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub model: String,        // e.g. anthropic/claude-3.5-sonnet, llama3.1
    pub endpoint: String,     // override base URL (empty = default)
    pub timeout_ms: u64,      // 1000..300000
    pub enabled: bool,
    pub fallback_order: u32,  // 0 = first attempt
    pub requires_api_key: bool,
}
```

Default chain (fallback_order 0→3):
`Anthropic (0) → Ollama (1) → OpenRouter (2) → SyntheticOffline (3)`

`RouterConfig` stores `provider_configs: Vec<ProviderConfig>` with `#[serde(default = "default_provider_configs")]`.
Legacy `primary_provider`/`fallback_provider`/`primary_model`/`fallback_model` kept for compat.

### API Key Resolution

`RouterConfig::api_key(&ModelProvider)` and `api_key_for_kind(&ProviderKind)` check `api_keys: HashMap<String,String>` then env:

| Provider | Env |
|----------|-----|
| Anthropic | `ANTHROPIC_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Ollama | none |

Engine extra keys `anthropic_api_key`/`openrouter_api_key`/`deepseek_api_key` are folded into `api_keys`.

### Failover Semantics

`TokenRouter::dispatch_prompt(prompt, system)`:
1. `ContextCompressor` if over budget (Phase 8.1).
2. Cache probe `hash(prompt, system, primary_model)`.
3. If no keys and both providers need keys → synthetic immediately.
4. `call_provider(primary)`:
   - Success → insert cache, return.
   - `is_retryable` (401/429/5xx + connect failures) or `429`/`rate` substring → `record_fallback()` + `tracing::info!(event="telemetry:fallback", provider, error)` → continue.
   - Other → return error.
5. `max_retries == 0` → return last_err.
6. `record_fallback()` + `tracing::info!(event="telemetry:provider_failover", from, to)`.
7. `call_provider(fallback)`:
   - Success → insert cache under both keys, return.
   - Failure + offline (both keys None) → warn + synthetic.
   - Failure → error `"all providers failed — primary: …, fallback (…): …"`.

`is_retryable` checks `status=401|429|500|502|503|504|529` + `failed to connect`/`connection refused`/`http request failed`.

`TokenMetrics::fallback_count` counts each retryable primary failure + fallback attempt.

### Tauri IPC — Phase 7.3

State: `EngineState { engine, provider_configs: Arc<RwLock<Vec<ProviderConfig>>> }` seeded from `engine.pipeline().router().config().provider_configs`.

#### `get_provider_configs() -> Vec<ProviderConfig>`

Reads RwLock, returns clone sorted by `fallback_order`.

```ts
const configs = await invoke<ProviderConfig[]>("get_provider_configs");
```

#### `set_provider_config(kind, endpoint?, timeout_ms?, enabled?, model?) -> Vec<ProviderConfig>`

Validates `timeout_ms` 1000..300000, non-empty model, known kind. Mutates `provider_configs` RwLock, returns sorted snapshot.

```ts
await invoke("set_provider_config", { kind: "ollama", endpoint: "http://gpu:11434", timeoutMs: 30000, enabled: true, model: "qwen2.5:14b" });
```

#### `reorder_provider_chain(order: ProviderKind[]) -> Vec<ProviderConfig>`

`order` is authoritative prefix; assigns `fallback_order 0..order.len()-1` to those kinds, pushes remainder (previous relative order) to tail. Validates non-empty, no duplicates, all known.

```ts
await invoke("reorder_provider_chain", { order: ["ollama","anthropic","open_router","synthetic_offline"] });
```

### Frontend — ProviderSettings

`apps/zylcode-desktop/src/components/ProviderSettings.tsx`:

- Fetches `get_provider_configs()` on mount.
- Cards per provider: dot (violet/sky/emerald/zinc), `API key`/`disabled` badges, `fallback_order`, ↑/↓ reorder (calls `reorder_provider_chain`), enabled checkbox (calls `set_provider_config`), inputs for `model`/`endpoint`/`timeout_ms` + Save.
- Live indicators: `listen("telemetry:provider_failover", Failover => …)` capped at 20.

Payload:

```ts
type Failover = { timestamp: string; from_provider: string; to_provider: string; reason: string; attempt_number: number };
```

`App.tsx` mounts `<ProviderSettings />` between ArtifactViewer and Verify section.

### Tuning Checklist

- [ ] Set `ANTHROPIC_API_KEY` and `OPENROUTER_API_KEY` env or engine extra.
- [ ] For air-gapped CI, leave both empty → synthetic; fallback still exercised via `max_retries`.
- [ ] Ollama host: `set_provider_config(kind="ollama", endpoint="http://…:11434")`.
- [ ] Validate timeout: `get_provider_configs` after set; check `timeout_ms` persisted.
- [ ] Drag chain to `Ollama → Anthropic → OpenRouter → Synthetic` for local-first.
