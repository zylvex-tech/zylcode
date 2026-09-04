# Getting Started

## Install

```bash
git clone https://github.com/zylvex-tech/zylcode.git
cd zylcode
pnpm install
```

## Run Desktop

```bash
pnpm --filter zylcode-desktop dev
```

Opens Vite dev server + Tauri window. Prompt → Stream → see ArtifactViewer tabs (GeneratedView.tsx, src/generated.rs).

## Run CLI Interactive

```bash
cargo run -p zylcode-desktop -- --interactive
# or
cargo run -- -i --manifest-path apps/zylcode-desktop/src-tauri/Cargo.toml
```

Commands inside: `help`, `tools`, `bridges`, `verify`, `tokens`, `<prompt>`.

## Test Suite

```bash
cargo test --workspace
cargo test -p zylcode-core compression -- --nocapture
cargo clippy --all-targets -- -D warnings
pnpm --filter zylcode-desktop build
```

All 70+ tests expected green (core 30, mcp 12, fault_injection 16, telemetry 19).

## Provider Quickstart

See `docs/PROVIDERS.md` and `docs/TUTORIALS.md#01`.

## Compression Quickstart

See `docs/COMPRESSION.md` and `docs/TUTORIALS.md#03`.

For architecture map see `docs/ARCHITECTURE.md`; for IPC reference see `docs/API.md`.
