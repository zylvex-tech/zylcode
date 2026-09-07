# ZylCode

> AI software synthesis engine & intent workspace featuring real-time artifact previews, dynamic MCP bridges, and zero token waste.

[![Release](https://img.shields.io/badge/release-v0.2.0-blue.svg)](https://github.com/zylvex-tech/zylcode/releases)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Architecture Overview

ZylCode is a high-performance modular Rust workspace with a Tauri desktop shell.

```
               +-----------------------------------+
               |      apps/zylcode-desktop         |
               |   (Tauri v2 + React Frontend)     |
               +-----------------+-----------------+
                                 | IPC
               +-----------------+-----------------+
               |        crates/zylcode-core        |
               |  - Token Router                   |
               |  - Decision Engine (permission)   |
               |  - Speculative Cache (LRU)        |
               |  - Context Compression Engine     |
               |  - Vector Cache (cosine sim)      |
               |  - Memchr Zero-Alloc Parser       |
               +-----------------+-----------------+
                                 |
               +-----------------+-----------------+
               |         crates/zylcode-mcp        |
               |  - Async Tool Execution Pipeline  |
               |  - Structured Telemetry           |
               |  - Audit Log (9 event types)      |
               |  - Stdio / SSE / WS Transports    |
               +-----------------------------------+
```

---

## Features

- **Token Router with Failover**: Typed `ProviderKind` (Anthropic, OpenRouter, Ollama, SyntheticOffline) with `ProviderConfig` fallback chains and `telemetry:provider_failover` logging.
- **Decision Engine**: Pure, side-effect-free permission checking (`check_permission()`) with typed `Decision` enum (`Allow`, `DenyNoRule`, `DenyExplicit`, `DenySession`, `DenyRateLimited`). Foundation for formal verification (Phase 11–12).
- **Zero-Allocation Streaming Parser**: Core pipeline utilizes `memchr` slice extraction to parse `<artifact>` and code fence blocks without runtime regex allocation overhead.
- **Speculative Router Cache**: In-memory `ahash` + `lru` entry caching layer eliminates redundant LLM verification roundtrips.
- **Context Compression**: Three-stage pipeline (`LosslessCommentsStripper` → `ASTOutlineExtractor` → `TokenWindowCompactor`) with budget targeting and `telemetry:compression` logging.
- **Vector Cache**: SQLite-backed `vector_cache` with deterministic `mock_embed` + cosine similarity ≥0.88 threshold, `telemetry:cache_hit` short-circuit in `TokenRouter::dispatch_prompt`.
- **Structured Telemetry & Audit**: 9 audit event types (`ToolCall`, `ToolDenied`, `PlanCreated`, `ProviderFailover`, `CacheHit`, `CacheMiss`, `Compression`, `VerificationRung`, `FormalProofAttempt`) with per-session run-level aggregation.
- **Dynamic MCP Integration**: Native support for Model Context Protocol servers via `mcp.tools.yaml` with runtime filesystem hot-reloading (`notify`).
- **Cross-Platform Bundles**: Native binaries for Windows (`.msi`, `.exe`), macOS (`.dmg`), and Linux (`.AppImage`, `.deb`).

---

## Workspace Structure

```
zylcode/
├── apps/
│   └── zylcode-desktop/       # Tauri v2 desktop app
│       ├── src/               # React frontend (components, hooks, lib)
│       └── Cargo.toml         # Tauri build manifest
├── crates/
│   ├── zylcode-core/          # Core engine
│   │   └── src/
│   │       ├── router.rs      # Token router + provider failover
│   │       ├── router/
│   │       │   ├── cache.rs   # SpeculativeCache (LRU)
│   │       │   └── decision.rs # Decision engine (pure permission checks)
│   │       ├── compression.rs # ContextCompressor + strategies
│   │       ├── cache.rs       # VectorCacheStore (SQLite, cosine sim)
│   │       ├── planner.rs     # Plan builder (includes verification flag)
│   │       ├── pipeline.rs    # Artifact parser
│   │       └── marketplace.rs # Plugin marketplace
│   └── zylcode-mcp/           # MCP registry & executor
│       └── src/
│           ├── audit.rs       # AuditEvent enum (9 types)
│           ├── telemetry.rs   # Structured telemetry
│           ├── registry.rs    # Tool registry
│           ├── executor.rs    # Async tool execution
│           └── config.rs      # mcp.tools.yaml loader
├── docs/                      # Documentation suite
├── mcp.tools.yaml             # Local tool configuration
└── Cargo.toml                 # Root workspace manifest
```

---

## Quickstart

### Prerequisites

* [Rust 1.75+](https://rustup.rs/)
* [Node.js 20+](https://nodejs.org/) & [pnpm 9+](https://pnpm.io/)

### Local Development

1. Clone the repository:
   ```bash
   git clone https://github.com/zylvex-tech/zylcode.git
   cd zylcode
   ```

2. Install frontend dependencies:
   ```bash
   pnpm install
   ```

3. Run the test suite across all workspace crates:
   ```bash
   cargo test --workspace --release
   ```

4. Launch the Tauri desktop environment in dev mode:
   ```bash
   pnpm --filter zylcode-desktop dev
   ```

---

## Configuring MCP Tools (`mcp.tools.yaml`)

Define Model Context Protocol integrations in `mcp.tools.yaml` at the root of the application:

```yaml
tools:
  - id: "filesystem_search"
    command: "node"
    transport: "stdio"
    enabled: true
    description: "Search workspace paths using regex patterns"
    env:
      NODE_ENV: "production"

  - id: "remote_synthesis_node"
    command: "https://mcp.zylvex.tech/v1"
    transport: "sse"
    enabled: true
    description: "SSE bridge to remote synthesis agent"
```

The engine watches `mcp.tools.yaml` and reloads registered tools in real time without restarting the process.

---

## Core Modules (v0.2.0)

| Module | Crate | Purpose |
|--------|-------|---------|
| **Token Router** | `zylcode-core::router` | Multi-provider routing with typed fallback chains and failover telemetry |
| **Decision Engine** | `zylcode-core::router::decision` | Pure permission checks — `check_permission()` returns typed `Decision` enum |
| **Speculative Cache** | `zylcode-core::router::cache` | LRU entry cache eliminating redundant LLM verification roundtrips |
| **Context Compressor** | `zylcode-core::compression` | Three-stage pipeline (strip comments → extract outline → compact tokens) |
| **Vector Cache** | `zylcode-core::cache` | SQLite-backed semantic cache with cosine similarity ≥0.88 threshold |
| **Planner** | `zylcode-core::planner` | Plan builder with `requires_formal_verification` flag |
| **Audit Log** | `zylcode-mcp::audit` | 9 audit event types with per-session aggregation |
| **Telemetry** | `zylcode-mcp::telemetry` | Structured tracing across execution trees |
| **Tool Executor** | `zylcode-mcp::executor` | Async tool execution with transient retry and timeout control |

---

## Documentation

| Doc | Purpose |
|-----|---------|
| `docs/ARCHITECTURE.md` | Workspace graph, lifecycle, milestone map (v0.2.0) |
| `docs/PROVIDERS.md` | ProviderKind/Config, failover, Tauri IPC, ProviderSettings |
| `docs/COMPRESSION.md` | Strategies, integration, tests, knobs |
| `docs/API.md` | Tauri IPC commands & events, core Rust API (includes vector cache IPC) |
| `docs/GETTING_STARTED.md` | Install, dev, CLI, tests |
| `docs/TUTORIALS.md` | 4 tutorials: provider setup, failover, compression tuning, MCP extensions |
| `docs/STRATEGIC_PLAN.md` | Single source of truth for project roadmap (Phases 0–14) |

---

## Benchmarks

```bash
cargo bench --workspace
```

---

## License

Licensed under the [Apache License 2.0](LICENSE).
