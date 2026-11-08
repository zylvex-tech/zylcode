# ZylCode — Software Creation Operating System

<p align="center">
  <img src="docs/branding/zylcode-logo.png" alt="ZylCode logo" width="240">
</p>

[![CI](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

**Last updated: 8 November 2026** · version `0.2.3` · pre-release, invite-only

ZylCode is an engineering workbench by **Zylvex Technologies Ltd.** It turns an
instruction into a plan, runs the plan through tools that must pass a permission
gate first, checks what came back, and records evidence for every technical claim
it makes — including the claims that failed.

It runs on your machine. The desktop app, the CLI and the local intelligence
service are local processes, and model calls go to providers you configure with
your own keys.

> **Early access.** The [Developer Preview waitlist](https://site-0522a86bd09e48629a1e90f115711758.freebuff.page)
> is open and pre-release / invite-only. No pricing is published during public
> development ([`docs/governance/ZYLCODE_COMMERCIAL_MODEL.md`](docs/governance/ZYLCODE_COMMERCIAL_MODEL.md)).

![ZylCode workbench with a live backend — repository explorer, terminal running real git commands, mission list, capability spotlight and the status bar reporting branch and version](docs/media/zylcode/14-workbench-live.png)

---

## What ZylCode is

A workbench, not a chat wrapper. Five stages, each with a real mechanism behind it:

| Stage          | What happens                                                                                                       |
| -------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Understand** | Repository index: files, symbols, dependency graph; `zylcode repo-context` answers "what would this change touch?" |
| **Plan**       | An intent becomes a plan with explicit steps and tool intents                                                      |
| **Act**        | Tool calls run through the permission gate in `zylcode-mcp`; consequential ones need approval                      |
| **Observe**    | Live terminal, source control, provider output — the terminal in the captures runs real commands                   |
| **Evidence**   | Hash-chained records under `.zylcode/`; a failing check stays visible as failed, a broken chain is an error        |

Two surfaces drive that core: a Tauri + React desktop application, and the
`zylcode` CLI. The same core is also served to the browser preview over
`zylcode serve-intel` on loopback.

---

## Status vocabulary

Every capability claim on this page uses one of these, and nothing else:

| Term                  | Meaning                                                                          |
| --------------------- | -------------------------------------------------------------------------------- |
| `VERIFIED_RUNTIME`    | A real binary ran it end to end, and the run is recorded                         |
| `VERIFIED_LOCAL_ONLY` | Automated tests prove it locally — real code paths, no live third-party endpoint |
| `PARTIAL`             | Works, with a stated gap in coverage, surface or provider                        |
| `UI_ONLY`             | Reachable in the interface, with no backend behaviour behind it                  |
| `PLANNED`             | Designed and prioritised; no implementation                                      |
| `NOT_IMPLEMENTED`     | Absent. If it appears in the interface at all, it is labelled as such            |

Two finer rungs are used inside the provider and tool reports:

| Term               | Meaning                                                                    |
| ------------------ | -------------------------------------------------------------------------- |
| `IMPLEMENTED_ONLY` | The code path exists and compiles; no automated test exercises it          |
| `UNIT_TESTED`      | Automated tests cover it (synthetic frames, scripted turns, loopback HTTP) |

A capability never moves up a rung because of configuration, a model name, or a
vendor's public documentation. Only a recorded run promotes it.

---

## Current capabilities

| Capability                                            | Status                | Where it is proven                                                                                                                                                                                |
| ----------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository explorer over a live index                 | `VERIFIED_RUNTIME`    | Capture `14` (live tree); `serve-intel` warm index: 764 files / 3,187 symbols / 8 packages, 2026-10-29                                                                                            |
| Terminal executing real commands                      | `VERIFIED_RUNTIME`    | Capture `16` (`cargo test … providers::` → 81 passed); 51 terminal tests                                                                                                                          |
| Evidence ledger, hash-chained                         | `VERIFIED_RUNTIME`    | Capture `18` (40 entries, 13 sessions, chain intact); tamper and ledger tests                                                                                                                     |
| Provider streaming, Anthropic / OpenAI-style / Ollama | `VERIFIED_LOCAL_ONLY` | `providers::*`; loopback HTTP server speaking each provider's real wire format, 81 tests                                                                                                          |
| Stream cancellation aborts the request                | `VERIFIED_LOCAL_ONLY` | `test_server::Scenario::BodyThenHold` reports socket teardown on cancel                                                                                                                           |
| Provider failover across the chain                    | `VERIFIED_LOCAL_ONLY` | Router failover suites; capture `17` shows the live chain                                                                                                                                         |
| Offline / synthetic output labelled as such           | `VERIFIED_LOCAL_ONLY` | `StreamChunk.synthetic`; the offline provider advertises `streaming: false`                                                                                                                       |
| Native tool calling — CLI                             | `VERIFIED_RUNTIME`    | `zylcode ask` against a local server speaking the OpenAI tool wire: 20 declarations, `fs_read` → `fs.read`, gate `allow`, file read, ledger written, continuation answered (open PR #51)          |
| Native tool calling — desktop chat                    | `VERIFIED_LOCAL_ONLY` | `native_chat::tests::desktop_session_runs_the_gated_tool_lifecycle`: real executor, real gate, real ledger, scripted provider. Not yet driven through Tauri IPC in a running window (open PR #53) |
| Native tool calling — live provider                   | `NOT_IMPLEMENTED`     | No provider has completed a tool turn against its real endpoint; every provider sits at `UNIT_TESTED` on the wire (open PR #51)                                                                   |
| MCP client (external servers)                         | `VERIFIED_LOCAL_ONLY` | `mcp_client`: JSON-RPC 2.0 over stdio and Streamable HTTP, `2025-06-18` / `2025-03-26`, supervision, remote tools gated through the same dispatcher (open PR #50)                                 |
| Mission compose → plan → gated tools → evidence       | `PARTIAL`             | `agent_loop_e2e`, `factory_job_e2e` cover the core; the full UI journey needs the desktop runtime                                                                                                 |
| Provider key editing / connection test                | `PARTIAL`             | Backend commands exist; editing is desktop-only, the browser preview is read-only                                                                                                                 |
| Design Studio                                         | `NOT_IMPLEMENTED`     | Labelled `COMING SOON` in the shell; no indexer, canvas or inspector exists yet                                                                                                                   |
| Vision input                                          | `NOT_IMPLEMENTED`     | `vision: false` everywhere; no request builder sends image blocks                                                                                                                                 |
| Log probabilities                                     | `NOT_IMPLEMENTED`     | Nothing requests or surfaces them                                                                                                                                                                 |
| Computer-use automation                               | `NOT_IMPLEMENTED`     | The engine is simulation ([`docs/governance/MAIN_STATUS_REPORT_2026-10-11.md`](docs/governance/MAIN_STATUS_REPORT_2026-10-11.md))                                                                 |

The row-by-row ledger, including which entries are disputed, is
[`docs/capability-registry.json`](docs/capability-registry.json). Builders do not
certify their own work ([`CONTRIBUTING.md`](CONTRIBUTING.md)).

### Where these capabilities live right now

Four pull requests are open and **none of them is merged into `main`**:

| PR  | Content                                                           | State                          |
| --- | ----------------------------------------------------------------- | ------------------------------ |
| #50 | MCP client: transports, supervision, gated remote tools           | CI green, awaiting review      |
| #51 | Provider-native tool calling across the real providers            | CI green, awaiting review      |
| #52 | Secret scanner: NVIDIA, Anthropic and OpenAI project-key patterns | CI green, awaiting review      |
| #53 | Desktop chat wired to the native tool runtime (stacked on #51)    | CI green for its coverage jobs |

Rows above marked `PR #5x` describe the integration head, not `main`. A clone of
`main` today shows the pre-tool-calling state: native tool calling, the MCP
client and the desktop wiring arrive only when those PRs land. Numbers in this
README are measured on the integration head and dated.

---

## Product architecture

```
zylcode/
├── apps/
│   ├── zylcode-desktop/        Tauri v2 + React desktop application
│   │   ├── src/                Shell surfaces: workbench, agent workspace, tool
│   │   │                       runner, repository & intel, terminal, git, settings
│   │   └── src-tauri/          Tauri commands (40 on the integration head, 39 on
│   │                           `main`): process_intent, process_intent_stream,
│   │                           cancel_intent, execute_tool, repo_context, nav_query,
│   │                           terminal_exec, evidence_ledger, provider key editing, …
│   └── natlas-sdk/             TypeScript SDK + playground for the N-ATLAS bridge
│                               (a separate integration workstream)
├── crates/
│   ├── zylcode-core/           Agent loop, planning, provider boundary, native tool
│   │   │                       loop, router/failover, repository intelligence,
│   │   │                       factory runner, evidence ledger, terminal, recovery
│   │   └── src/providers/      streaming.rs (neutral contract) · anthropic.rs ·
│   │                           openai_compatible.rs · ollama.rs · sse.rs ·
│   │                           compat.rs · capabilities.rs · synthetic.rs
│   ├── zylcode-mcp/            Tool registry, permission gate, evidence, executors,
│   │                           risk classification, MCP client (PR #50)
│   ├── zylcode-nav/            Repository scanner, symbol index, dependency graph
│   └── zylcode-cli/            `zylcode` binary: ask, repo-context, serve-intel,
│                               mission, best-of-n, package, mcp-bridge, …
├── docs/                       Architecture, governance, roadmap, audit, design
└── .github/workflows/          CI (check, clippy, fmt, coverage, frontend coverage)
```

**The provider boundary.** The router selects a provider and walks the fallback
chain; it knows nothing about wire formats. Each provider module builds the
request and parses its own protocol into one neutral contract:

```
router.rs            selects, fails over, degrades offline
   ▼
providers::anthropic │ openai_compatible │ ollama │ synthetic
   │  builds the request, parses SSE / NDJSON / JSON, maps errors
   ▼
StreamChunk { index, kind, done, finish_reason, synthetic, tool_state, usage,
              metadata, provider_ref, error }        ← the contract
   │
   ├─▶ agent loop / native tool loop
   └─▶ compat::StreamEvent { index, delta, done }     ← lossy IPC projection
```

The last hop is deliberately lossy and that is a known gap: the desktop IPC
payload carries only `{index, delta, done}`, so finish reason, usage, structured
errors and the `synthetic` flag stop at the boundary. The chunk is the source of
truth and the projection is derived from it, so lifting the rest into the UI is a
projection change, not a re-plumbing.

**One dispatch path.** Every tool call — built in, MCP-remote, model-requested,
human-requested — ends in `zylcode_mcp::real_tools::dispatch`, which performs
attribution, risk classification, the permission gate, execution and evidence
recording. There is no second executor and no second gate anywhere in the tree.

---

## AI providers and configuration

Eight providers are wired: `openai`, `openai-compatible` (any compatible
endpoint, including a local one), `openrouter`, `deepseek`, `respan`,
`anthropic`, `ollama` (local) and the built-in `synthetic-offline`.

Configuration is a fallback **chain**: an ordered list of providers, each with its
own model and base URL. The router tries them in order, records the attempt, and
degrades to the offline provider rather than leaving a request hanging.

```bash
# CLI reads the same configuration the desktop app writes
export ANTHROPIC_API_KEY=…            # or OPENAI_API_KEY, OPENROUTER_API_KEY, …
zylcode ask "explain crates/zylcode-mcp/src/permission.rs"

# Local model, no account
export OLLAMA_BASE_URL=http://localhost:11434
zylcode ask --provider ollama --model llama3 "what does this crate do?"
```

Capabilities are data, not marketing. One table
([`capabilities.rs`](crates/zylcode-core/src/providers/capabilities.rs)) answers
two separate questions per provider: does this client implement the protocol, and
how far has each stage of it actually been proven. `tools: true` is an
implementation claim — the request builder emits a declaration and the decoder
reads a call back. It is never a claim that a vendor endpoint was exercised.

### BYOK and credential handling

- **Keys are yours.** ZylCode has no hosted inference, no proxy and no resale; a
  request goes from your machine to the provider you configured.
- **Where a key is stored.** The desktop writes keys to the OS credential vault —
  Windows Credential Manager today, under the account
  `zylcode/provider/<provider>`. It is deliberately not written to a JSON file
  next to the workspace, where a backup, sync folder or screenshot would leak it.
  macOS Keychain and Linux Secret Service are not implemented yet: those builds
  report the store as unsupported and fall back to environment variables.
- **What comes back out.** No command returns a key to the interface. Status rows
  carry a masked hint, and the only consumer of the plaintext is the router's
  in-memory configuration.
- **Environment variables remain first class**, and are the supported path on
  Linux and macOS today. Precedence: a stored key first, then the variable.
- **Nothing in this repository should ever contain a key.** CI runs
  [`scripts/check_hardcoded_secrets.py`](scripts/check_hardcoded_secrets.py) over
  the tree, which detects provider-key shapes (including NVIDIA `nvapi-`,
  Anthropic `sk-ant-` and OpenAI `sk-proj-`, added in PR #52).

---

## CLI

```bash
cargo build -p zylcode-cli --release      # binary at target/release/zylcode
```

| Command                                                       | What it does                                                                                                                                                    |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `zylcode ask "<task>"`                                        | Ask the configured provider, with native tool calling: calls are resolved to ZylCode ids, gated, executed, recorded, and returned to the model so it can finish |
| `zylcode repo-context --task "<task>"`                        | Repository intelligence: scanner, symbols, packages, dependency graph, entry points, git history                                                                |
| `zylcode serve-intel`                                         | Read-only intelligence API on loopback (`/healthz`, `/api/repo-intel`) for the browser preview                                                                  |
| `zylcode mission`                                             | The first mission end to end: specification → plan → real tools → real failure → repair → record → resume                                                       |
| `zylcode best-of-n`                                           | Sample N candidate patches, verify each in an isolated worktree against the real test suite, select on evidence                                                 |
| `zylcode package`                                             | Run the build pipeline and package a versioned release bundle with a SHA-256 manifest into `.zylcode/releases/`                                                 |
| `zylcode mcp-bridge register/list/remove`                     | Manage MCP bridge endpoints (the protocol client itself is PR #50)                                                                                              |
| `zylcode deploy-status`                                       | Report deploy targets that are actually commissioned                                                                                                            |
| `zylcode build` / `marketplace` / `ai-input` / `computer-use` | Project build, extension marketplace, multi-modal input, computer-use inspection                                                                                |

`zylcode ask` is the surface where native tool calling is `VERIFIED_RUNTIME`:
it has been run twice against a server speaking the OpenAI tool wire, and both
runs recorded the same lifecycle — declaration, canonical resolution, gate
decision, execution, evidence, continuation.

---

## Desktop application

Prerequisites: Rust stable (MSRV **1.90**, `Cargo.toml`), Node ≥ 18 (CI uses 22),
pnpm ≥ 9 (locked at 9.12.0). Linux builds need the usual Tauri libraries
(GTK/WebKit).

```bash
git clone https://github.com/zylvex-tech/zylcode.git
cd zylcode
pnpm install --frozen-lockfile

pnpm --filter zylcode-desktop dev          # browser preview (Vite)
pnpm --filter zylcode-desktop tauri dev    # desktop window (first run compiles Rust)
```

The browser preview needs the intelligence service beside it — that is how the
explorer, terminal and evidence views report real state:

```bash
cargo run -p zylcode-cli -- serve-intel    # loopback only, :17630
```

Without it those panels show `BLOCKED` rather than inventing data. Provider
editing, key storage and the native-tool chat path exist only in the desktop
window, because they need the Tauri runtime.

**No signed installers are published.** A source checkout is the supported path
today. A container with the whole toolchain is included
(`docker compose run --rm dev`), plus a
[`.devcontainer`](.devcontainer/devcontainer.json) for VS Code / Cursor. Platform
notes: [`docs/INSTALLATION_GUIDE.md`](docs/INSTALLATION_GUIDE.md).

---

## Native tool calling

Two loops exist, and exactly one of them is native.

```
prompt → IntentPlanner → NativeToolRunner
   → provider returns a native tool call (wire name `fs_read`)
   → resolve to the canonical id (`fs.read`); an unknown name fails closed
   → arguments must be a JSON object, or the call is refused before the gate
   → real_tools::dispatch — permission gate + ToolRegistry + evidence sink
   → the result goes back as a provider-native tool result under the same call id
   → continuation; the model answers from what it actually received
```

- **One loop, two surfaces.** `zylcode-core::native_tools::NativeToolRunner` is
  driven by both `zylcode ask` and the desktop chat command. The desktop has no
  runner, no tool list and no executor of its own.
- **The fallback is named as a fallback.** For a provider that cannot be handed
  declarations, the older `AgentLoop` still runs: same gate, same registry, but
  the model is asked to format a decision as JSON in prose. A prompt-JSON call is
  not a native call, and it is reported as `PARTIAL`.
- **Capability decides, not hope.** The loop is used only where the capability
  table declares tools. `synthetic-offline` declares none, so an offline turn
  never pretends to have called a tool.
- **Cancellation** is a shared token polled before each provider round and between
  tool calls, so a cancelled turn stops instead of leaving a continuation in
  flight.

Per-stage provider evidence is reported as data, not prose. Today: the three wire
stages are `UNIT_TESTED` per provider, the shared lifecycle stages are
`UNIT_TESTED` once, the whole-turn-on-the-wire stage is `IMPLEMENTED_ONLY` for
every provider, and nothing anywhere is `LIVE_VERIFIED`. A test fails if a row is
promoted without a recorded run behind it.

## Permission and risk controls

- **Every call passes one gate.** `PermissionGate` fails closed: the restrictive
  default denies everything above `Read`. Risk levels are derived from the tool
  and its arguments (`Read`, `Write`, `Execute`, `Network`, `GitWrite`,
  `Destructive`, `Release`), never declared by the caller.
- **A model-requested call gets no approval.** `approval_required` is false on
  that path, so the gate keeps refusing anything above `Read` rather than being
  handed consent nobody gave. The model cannot widen its own permissions.
- **Refusals are decisions, not silence.** A denied call produces an evidence
  record and a UI status of `failed` with the gate's reason; the interface never
  renders a denial as a success.
- **MCP-remote tools are not special.** A remote tool implements `RealTool` and
  goes through `dispatch_with`, so it gets the same gate, evidence and refusal
  path. A refused remote call never reaches the server (PR #50).
- **No arbitrary JavaScript execution** anywhere in the app surface; the
  workspace is a path the user chose, and the shell executor runs through the
  workspace resolver rather than the process working directory.

## Evidence recording

- **Hash-chained.** Entries live in a JSONL ledger under `.zylcode/`. A broken
  chain is reported as an error, not an empty list.
- **Failures stay.** A failed check is written as failed and stays failed.
- **Every executed tool call records** the tool id, the actor, the gate's
  decision, the invocation id and the outcome.
- **Where:** CLI uses `ZYLCODE_EVIDENCE_LOG`, else `.zylcode/evidence.jsonl`
  relative to the process working directory. The desktop writes to the
  application settings directory (`%APPDATA%\ZylCode\evidence.jsonl`, overridden
  by `ZYLCODE_CONFIG_DIR`), because a packaged app's working directory is wherever
  the launcher happened to be. The CLI's CWD-relative default is a known defect
  and belongs to the unified-data-layer work.

## MCP client

`zylcode-mcp` has always been an in-process registry, permission gate and executor
set. PR #50 adds what was missing — an actual protocol client:

| Layer       | Responsibility                                                                             |
| ----------- | ------------------------------------------------------------------------------------------ |
| `jsonrpc`   | JSON-RPC 2.0 envelopes, id correlation, malformed-message errors                           |
| `transport` | stdio (spawn, frame, capture stderr) and Streamable HTTP (POST/GET, SSE, `Mcp-Session-Id`) |
| `session`   | `initialize` → `initialized` → `tools/list` → `tools/call`, `ping`, cancellation           |
| `registry`  | start/restart with backoff, discovery cache, status snapshot                               |
| `remote`    | a discovered tool as a `RealTool`, dispatched through the same gate                        |

Implemented protocol revisions are `2025-06-18` and `2025-03-26`. The handshake-free
`2026-07-28` revision is **not** implemented: a server selecting it is refused with
the versions this client does offer, never silently downgraded. On `main` today the
MCP badge is `NOT_IMPLEMENTED`; on PR #50 it is `VERIFIED_LOCAL_ONLY`.

## Design Studio

`NOT_IMPLEMENTED`. The shell labels it `COMING SOON`, and that label is accurate:
there is no token parser, component indexer, canvas or inspector in the tree. The
intended scope is documented in
[`docs/product/ZYLCODE_PRODUCT_ARCHITECTURE_V3.md`](docs/product/ZYLCODE_PRODUCT_ARCHITECTURE_V3.md)
§23. Nothing on this page should be read as a Design Studio claim.

## Vision

`NOT_IMPLEMENTED`. No request builder constructs image content blocks, so
`vision: false` on every provider row. The model-capability registry keeps
`image_input` (transport) and `vision_interpretation` (understanding) apart on
purpose, and the interface prints `VISION_UNVERIFIED` rather than implying image
understanding. Tool calling must not be read as multi-modal support.

## Offline / synthetic mode

`synthetic-offline` is a deterministic local provider with no network. It exists so
the app, the tests and CI can run without an account, and it is the provider the
offline test suites use.

- It advertises `streaming: false` and `tools: false`, and it never fabricates a
  tool call: chunks carry `synthetic: true`, and the chunked UI feedback it
  produces is app feedback shaped for the interface, not evidence of a provider
  stream.
- When a provider fails, the router degrades to it explicitly instead of leaving a
  request hanging, and the degrade is visible.

---

## Development and testing

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python scripts/check_hardcoded_secrets.py
python scripts/check_retracted_claims.py    # keeps retracted claims from returning

pnpm --filter zylcode-desktop typecheck
pnpm --filter zylcode-desktop test
pnpm --filter zylcode-desktop test:coverage
pnpm --filter zylcode-desktop build
```

Measured on the integration head (`#50`–`#53`) on 8 November 2026, on Windows:

| Check                                                   | Result                                                                      |
| ------------------------------------------------------- | --------------------------------------------------------------------------- |
| `cargo test --workspace --all-targets`                  | **1,099 passed, 0 failed** across 31 test binaries                          |
| `cargo test -p zylcode-desktop --bins`                  | **24 passed, 0 failed**                                                     |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0                                                                      |
| `cargo fmt --all -- --check`                            | clean                                                                       |
| `pnpm --filter zylcode-desktop test`                    | **766 passed in 67 files**, exit 0                                          |
| Frontend coverage                                       | **97.07 / 90.72 / 96.2 / 97.07** (stmts / branches / funcs / lines), exit 0 |
| `pnpm --filter zylcode-desktop build`                   | exit 0 (tsc + vite)                                                         |

Gates are floors, not aspirations — CI fails below them, and each floor sits just
under its measured value so a new untested file turns the run red instead of being
absorbed. Frontend floors live in
`apps/zylcode-desktop/vitest.config.ts`, Rust coverage in
`.github/workflows/coverage.yml` (78% lines), and `ci.yml` runs tests, clippy, fmt
and both guards. The repository-intelligence benchmark floor sits in
`crates/zylcode-core/tests/repo_intelligence_benchmark.rs`.

Provider tests talk real HTTP to a loopback server that emits each provider's
documented wire format: framing, sockets and teardown are real, but no request
leaves the machine. That is why the provider rows say `VERIFIED_LOCAL_ONLY`.

---

## What it looks like

Real captures only, same `v0.2.3` build, browser-preview mode against a live
`zylcode serve-intel`, 1440×900 at 2× DPR. Reproduce steps are in
[`docs/media/zylcode/README.md`](docs/media/zylcode/README.md).

| Surface                                             | Capture                                                        |
| --------------------------------------------------- | -------------------------------------------------------------- |
| Mission Composer with a typed intent                | ![Mission](docs/media/zylcode/15-mission-intent.png)           |
| Terminal running the provider suite (81)            | ![Terminal](docs/media/zylcode/16-terminal-provider-tests.png) |
| Provider chain and in-process tool runtime          | ![Providers](docs/media/zylcode/17-provider-chain.png)         |
| Evidence ledger, failures included                  | ![Evidence](docs/media/zylcode/18-evidence-ledger.png)         |
| Workbench with no backend attached (BLOCKED states) | ![Blocked](docs/media/zylcode/06-mission-lifecycle.png)        |

---

## Known limitations

- **No live-provider verification.** No recorded run exists against a real
  Anthropic, OpenAI, OpenRouter, DeepSeek, Respan or Ollama endpoint, for
  streaming or for tool calling. Everything provider-facing is
  `VERIFIED_LOCAL_ONLY`.
- **Desktop native tools are not runtime-verified.** The session function is
  tested with a scripted provider; Tauri IPC and the window are not.
- **Streaming text on the native tool path arrives once per turn**, not token by
  token: the native loop aggregates each round. Incremental text needs a delta
  callback through the turn contract.
- **The IPC projection is lossy** — finish reason, usage, tool deltas, structured
  errors and the synthetic flag stop at the boundary.
- **Keys are stored only on Windows.** macOS Keychain and Linux Secret Service are
  not implemented; those builds use environment variables.
- **The CLI evidence log is CWD-relative** by default.
- **Cancellation has a command but no button** in the desktop UI yet.
- **Provider editing is desktop-only**; the browser preview is read-only.
- **Design Studio, vision, log probabilities and computer use are absent**, and
  are labelled as absent wherever they appear.
- **Acceptance is not self-granted.** Part of `docs/capability-registry.json` is
  disputed and independent acceptance remains outstanding.
- **No signed installers, no published pricing.**

---

## Contributing

[`CONTRIBUTING.md`](CONTRIBUTING.md) — including the evidence rule: builders do not
certify their own work. Statuses in this README are a report, not a promise; if a
change moves a capability up a rung, it needs the recorded run that proves it.

- **Code of conduct:** [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- **Security:** [`SECURITY.md`](SECURITY.md) · **Support:** [`SUPPORT.md`](SUPPORT.md)
- **Deeper reading:** [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md),
  [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md),
  [`docs/TUTORIALS.md`](docs/TUTORIALS.md),
  [`docs/VERIFICATION.md`](docs/VERIFICATION.md),
  [`docs/PROJECT_MASTER_STATUS.md`](docs/PROJECT_MASTER_STATUS.md)

### N-ATLAS integration

The **N-ATLAS developer bridge** is one integration and competition workstream,
not the product's identity: it routes instructions (English, Yoruba, Hausa, Igbo)
through the `NCAIR1/N-ATLaS` model into the approval-gated pipeline, recording the
chain as evidence. Branch
[`competition/natlas-2026`](https://github.com/zylvex-tech/zylcode/tree/competition/natlas-2026),
docs in [`docs/competition/natlas/`](docs/competition/natlas/), SDK in
[`apps/natlas-sdk/`](apps/natlas-sdk/).

The model (`NCAIR1/N-ATLaS`, a Llama-3 8B SFT fine-tune) was developed by
**Awarri Technologies** with **FMCIDE** and **NCAIR** under **NITDA**, under the
_N-ATLaS Open-Source Research and Innovation License_ (≤ 1,000 active end-users;
attribution to Awarri + FMCIDE required). No model weights are committed here, and
that licence is separate from this codebase's.

## License

**Apache License 2.0** — see [`LICENSE`](LICENSE).

ZylCode is developed by **Zylvex Technologies Ltd.** There are no customer logos,
endorsements or partnership claims on this page, because none are verified in this
repository.

<p align="center">
  <strong>Built with evidence. Verified with honesty.</strong><br>
  <em>ZylCode — Software Creation Operating System · Zylvex Technologies Ltd.</em>
</p>
