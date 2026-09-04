# ZylCode Architecture — Deep Dive

> Source of truth for workspace layout, data flow, and milestone map through Phase 8.1.

## 1. Workspace Graph

```
zylcode/
├── apps/zylcode-desktop/          # Tauri 2 + React + Vite
│   ├── src-tauri/src/main.rs      # IPC commands, process_intent_stream, provider failover
│   └── src/
│       ├── App.tsx                # top-level layout + ProviderSettings mount
│       ├── components/
│       │   ├── ProviderSettings.tsx   # Phase 7.3 — reorder, endpoint, timeout, live failover
│       │   ├── ArtifactViewer.tsx
│       │   ├── TokenMetricsWidget.tsx
│       │   └── ...
│       └── lib/events.ts          # intent:chunk / telemetry:provider_failover / telemetry:compression
├── crates/zylcode-core/           # pure Rust engine
│   ├── src/lib.rs                 # ZylCodeEngine, re-exports
│   ├── src/router.rs              # ModelProvider, ProviderKind, ProviderConfig, TokenRouter
│   ├── src/router/cache.rs        # SpeculativeCache (ahash + lru)
│   ├── src/compression.rs         # Phase 8.1 — ContextCompressor + 3 strategies
│   ├── src/pipeline.rs            # ArtifactPipeline, parse_artifacts (memchr)
│   └── src/planner.rs             # IntentPlanner
└── crates/zylcode-mcp/            # MCP registry, executor, telemetry
```

## 2. Request Lifecycle

```
React invoke("process_intent_stream", {prompt, model})
  → main.rs:process_intent_stream
    → spawns background TokenRouter::dispatch_stream for intent:chunk UI
    → snapshots fallback_count
    → ZylCodeEngine::process_intent_with_model
      → ArtifactPipeline::execute_for_engine
        → IntentPlanner::build_execution_plan(prompt, bridges) → {compiled_prompt, system_prompt}
        → TokenRouter::dispatch_prompt
          → [Phase 8.1] ContextCompressor::compress if est_tokens > budget
            → telemetry:compression {original, compressed, ratio}
          → SpeculativeCache probe
          → call_provider(primary)
            → on 429/5xx/timeout: info!(telemetry:fallback) + record_fallback + retry fallback
          → telemetry:provider_failover tracing
        → parse_artifacts (memchr)
        → verify_logic
        → IntentResult + ProofMetrics
    → delta fallback_count → emit telemetry:provider_failover ProviderFailoverPayload[] via app.emit
    → emit intent:done + terminal chunk
```

## 3. State & Concurrency

- `ZylCodeEngine` is `Clone` via `Arc<RwLock<…>>` for bridges/marketplace/pipeline/tool_registry.
- `EngineState` holds `engine: ZylCodeEngine` + `provider_configs: Arc<RwLock<Vec<ProviderConfig>>>`.
- `TokenMetrics` uses `AtomicU64` (Relaxed) for input/output/saved/fallback.
- `SpeculativeCache` is `Arc` + `RwLock<LruCache>` with TTL sweep.

## 4. Milestone Map

| Phase | Scope | Key Files | Status |
|-------|-------|-----------|--------|
| 6.1 | Native IPC events | `main.rs` `token_metrics`, `verify_logic` | ✅ |
| 6.2 | React telemetry hooks | `useArtifactStream`, `TokenMetricsWidget` | ✅ |
| 6.3 | SQLite + bundler | `init_telemetry_db`, `tauri.conf.json` | ✅ |
| 7.1 | CI/CD | `.github/workflows/release.yml` | ✅ |
| 7.2 | Multi-model routing & failover | `router.rs` ProviderKind/Config + telemetry | ✅ |
| 7.3 | Provider Settings UI | `ProviderSettings.tsx` + 3 Tauri commands | ✅ |
| 8.1 | Context compression | `compression.rs` + dispatch integration | ✅ |

## 5. Invariants

- Offline-first: no API keys → synthetic XML response, parseable by `parse_artifacts`.
- Cache key = hash(prompt, system, model).
- System prompt preserved (30% budget reserved).
- No `unwrap` on provider HTTP; all errors annotated with `[status=…]` for `is_retryable`.
