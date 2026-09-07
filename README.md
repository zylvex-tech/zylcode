# ZylCode

> AI software synthesis engine & intent workspace featuring real-time artifact previews, dynamic MCP bridges, and zero token waste.

[![CI](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml/badge.svg)](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml)
[![Release](https://github.com/zylvex-tech/zylcode/actions/workflows/release.yml/badge.svg)](https://github.com/zylvex-tech/zylcode/actions/workflows/release.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Architecture Overview

ZylCode is structured as a high-performance modular Rust workspace with a Tauri desktop shell.

```
              +-----------------------------------+
              |      apps/zylcode-desktop         |
              |   (Tauri v2 + React Frontend)     |
              +-----------------+-----------------+
                                | IPC
              +-----------------+-----------------+
              |        crates/zylcode-core        |
              |  - Speculative Cache (LRU)        |
              |  - Context Compression Engine     |
              |  - Memchr Zero-Alloc Parser       |
              +-----------------+-----------------+
                                |
              +-----------------+-----------------+
              |         crates/zylcode-mcp        |
              |  - Dynamic Config Hot-Reload      |
              |  - Async Tool Execution Pipeline  |
              |  - Stdio / SSE / WS Transports    |
              +-----------------------------------+
```

---

## Features

- **Zero-Allocation Streaming Parser**: Core pipeline utilizes `memchr` slice extraction to parse `<artifact>` and code fence blocks without runtime regex allocation overhead.
- **Speculative Router Cache**: In-memory `ahash` + `lru` entry caching layer eliminates redundant LLM verification roundtrips.
- **Dynamic MCP Integration**: Native support for Model Context Protocol servers via `mcp.tools.yaml` with runtime filesystem hot-reloading (`notify`).
- **Resilient Tool Execution**: Automated transient error retries, configurable call timeouts, and structured tracing across execution trees.
- **Cross-Platform Bundles**: Native binaries for Windows (`.msi`, `.exe`), macOS (`.dmg`), and Linux (`.AppImage`, `.deb`).

---

## Workspace Structure

```
zylcode/
├── apps/
│   └── zylcode-desktop/    # Tauri v2 application & frontend UI
├── crates/
│   ├── zylcode-core/       # Core synthesis engine, token router, & parser
│   └── zylcode-mcp/        # Model Context Protocol registry & executor
├── mcp.tools.yaml          # Local tool configuration file
└── Cargo.toml              # Root workspace manifest
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

3. Run cargo test suite across all workspace crates:
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

## Providers, Compression & Vector Cache (v0.2.0)

- **Phase 7.2/7.3 Multi-Model Routing** — typed `ProviderKind` (Anthropic, OpenRouter, Ollama, SyntheticOffline), `ProviderConfig` fallback chain, `telemetry:provider_failover` + `ProviderSettings` UI. See `docs/PROVIDERS.md` and tutorial `docs/TUTORIALS.md#02`.
- **Phase 8.1 Context Compression** — `ContextCompressor` (`LosslessCommentsStripper` → `ASTOutlineExtractor` → `TokenWindowCompactor`) with budget targeting and `telemetry:compression`. See `docs/COMPRESSION.md` and `docs/TUTORIALS.md#03`.
- **Phase 8.2 Vector Cache** — SQLite `vector_cache` with deterministic `mock_embed` + cosine similarity `≥0.88` threshold, `telemetry:cache_hit` short-circuit in `TokenRouter::dispatch_prompt`, `clear_vector_cache` / `get_cache_stats` IPC. See `docs/ARCHITECTURE.md` and `docs/API.md`.

## Documentation Hub

| Doc | Purpose |
|-----|---------|
| `docs/ARCHITECTURE.md` | Workspace graph, lifecycle, milestone map (v0.2.0) |
| `docs/PROVIDERS.md` | ProviderKind/Config, failover, Tauri IPC, ProviderSettings |
| `docs/COMPRESSION.md` | Strategies, integration, tests, knobs |
| `docs/API.md` | Tauri IPC commands & events, core Rust API (includes vector cache IPC) |
| `docs/GETTING_STARTED.md` | Install, dev, CLI, tests |
| `docs/TUTORIALS.md` | 4 tutorials: provider setup, failover, compression tuning, MCP extensions |

## Benchmarks

To execute internal router and pipeline micro-benchmarks:

```bash
cargo bench --workspace
```

---

## License

Licensed under the [Apache License 2.0](LICENSE).
