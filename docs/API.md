# API — Tauri IPC & Core

## Tauri Commands (`apps/zylcode-desktop/src-tauri/src/main.rs`)

All invoked via `@tauri-apps/api/core` `invoke`:

```ts
import { invoke } from "@tauri-apps/api/core";
```

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `process_intent` | `{prompt: string, model?: string\|null}` | `IntentResult` | Non-streaming pipeline run |
| `process_intent_stream` | `{prompt: string, model?: string\|null}` | `IntentResult` | Emits `intent:chunk` stream + `telemetry:provider_failover` + `intent:done` |
| `preview_execution_plan` | `{prompt: string}` | `ExecutionPlan` | Dry-run planner |
| `token_metrics` | — | `TokenSnapshot {input_tokens, output_tokens, verification_saved_tokens, fallback_count}` | Atomic snapshot |
| `verify_logic` | — | `VerificationReport` | `workspace_root_present` + `workspace_root_exists` |
| `register_mcp_bridge` | `{id, endpoint, transport?}` | `()` | `transport ∈ {stdio,sse,websocket}` |
| `list_mcp_bridges` | — | `McpBridgeDescriptor[]` | |
| `list_tools` | — | `ToolDescriptor[]` | registry snapshot |
| `execute_tool` | `{id, params: Json}` | `Json` | with recovery (timeout 30s, 2 retries) |
| `marketplace_search` | `{query: string}` | `MarketplaceExtension[]` | |
| `get_provider_configs` | — | `ProviderConfig[]` | sorted by `fallback_order` (Phase 7.3) |
| `set_provider_config` | `{kind: ProviderKind, endpoint?: string\|null, timeoutMs?: number\|null, enabled?: boolean\|null, model?: string\|null}` | `ProviderConfig[]` | validates timeout 1000..300000 |
| `reorder_provider_chain` | `{order: ProviderKind[]}` | `ProviderConfig[]` | authoritative prefix, rest appended |

## Events (listen via `@tauri-apps/api/event`)

```ts
import { listen } from "@tauri-apps/api/event";
const unlisten = await listen<StreamDelta>("intent:chunk", (ev) => …);
const unlisten2 = await listen<Failover>("telemetry:provider_failover", (ev) => …);
```

| Event | Payload | Source |
|-------|---------|--------|
| `intent:chunk` | `StreamDelta {index, delta, done, phase?}` | `process_intent_stream` background + terminal chunk |
| `intent:done` | `IntentResult {summary, artifacts[], success}` | pipeline result |
| `telemetry:provider_failover` | `ProviderFailoverPayload {timestamp, from_provider, to_provider, reason, attempt_number}` | fallback delta detection after pipeline |
| `telemetry:fallback` | (tracing) `event, provider, error` | `router.rs` dispatch |
| `telemetry:provider_failover` | (tracing) `event, from, to` | `router.rs` fallback hop |
| `telemetry:compression` | (tracing) `original_tokens, compressed_tokens, compression_ratio` | `compression.rs` ContextCompressor |

## Core Rust API (`crates/zylcode-core`)

### ZylCodeEngine

```rust
ZylCodeEngine::new(EngineConfig { workspace_root, verbose, extra })
ZylCodeEngine::with_defaults()
ZylCodeEngine::with_router_config(EngineConfig, RouterConfig) -> Result<Self>
engine.pipeline() -> &ArtifactPipeline
engine.pipeline().router().snapshot() -> TokenSnapshot
engine.pipeline().router().config() -> &RouterConfig
engine.process_intent(Intent { prompt, context, correlation_id }) -> Result<IntentResult>
engine.process_intent_with_model(Intent, Option<String>) -> Result<IntentResult>
engine.verify_logic() -> Result<VerificationReport>
engine.register_mcp_bridge(McpBridgeDescriptor) -> Result<()>
engine.list_mcp_bridges() -> Vec<McpBridgeDescriptor>
engine.list_tools() -> Vec<ToolDescriptor>
engine.execute_tool(id, Json) -> Result<Json>
```

### ArtifactPipeline

```rust
ArtifactPipeline::from_config(RouterConfig) -> Result<Self>
pipeline.execute_for_engine(Intent, Vec<McpBridgeDescriptor>, String) -> Result<IntentResult>
ArtifactPipeline::parse_artifacts(raw: &str) -> Vec<Artifact>
ProofMetrics { artifacts_generated, artifacts_verified, verification_passed, verification_duration_ms, verification_checks, tokens: TokenSnapshot }
```

### Router

```rust
RouterConfig::default() // OpenRouter→llama fallback + 4 ProviderConfigs
RouterConfig::from_env() // reads ZYLCODE_PRIMARY_MODEL, ZYLCODE_FALLBACK_MODEL
router.api_key(provider: &ModelProvider) -> Option<String>
router.api_key_for_kind(kind: &ProviderKind) -> Option<String>
router.base_url(provider: &ModelProvider) -> String
trim_to_window(prompt, system, window_tokens, ContextTrim) -> (String, String)
TokenRouter::new(cfg) / with_metrics / with_cache -> Result<Self>
router.dispatch_prompt(prompt, system) -> Result<String> // compression + cache + failover + synthetic
router.dispatch_stream(prompt, system, |StreamEvent|) -> Result<String>
TokenMetrics::{snapshot, record_usage, record_saved, record_fallback}
SpeculativeCache::new(capacity, ttl) / with_defaults() / hash_key / get / insert
```

### Compression (Phase 8.1)

```rust
ContextCompressor::new(budget_tokens) / default() // 8192
ContextCompressor::estimate_tokens(&str) -> usize // len.div_ceil(4)
compressor.compress(prompt, system) -> (String, String, CompressionMetrics)
compressor.compress_single(input) -> (String, CompressionMetrics)
LosslessCommentsStripper / ASTOutlineExtractor / TokenWindowCompactor : CompressionStrategy
CompressionMetrics { original_tokens, compressed_tokens, compression_ratio, strategy }
```

### MCP (`crates/zylcode-mcp`)

```rust
register_from_config_file(&ToolRegistry, &Path) -> Result<usize>
register_from_default_location(&ToolRegistry) -> usize
config::install_global_watcher(Path, Arc<Fn(McpConfigFile)>)
ToolRegistry::clear / register_many / list / get
execute_with_recovery(Tool, Json, ExecuteOptions) -> Result<Json>
```

## Error Mapping

Tauri commands map `anyhow::Error` → `String` via `e.to_string()` for IPC. `call_provider` annotates HTTP errors as `"{err} [status={status}]"` for `is_retryable`.
