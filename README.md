# ZylCode — Software Creation Operating System

<p align="center">
  <img src="docs/branding/zylcode-logo.png" alt="ZylCode logo" width="240">
</p>

[![CI](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/zylvex-tech/zylcode/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

**Project status: 6 November 2026** · branch `fix/r05-input-validation` · head `fc69e4c` · pre-release

ZylCode is an engineering workbench developed by **Zylvex Technologies Ltd.** It
turns an instruction into a plan, executes the plan through approval-gated tools,
checks the result, and records evidence for every technical claim it makes.

It runs on your machine and uses your own model keys: the desktop app, the CLI and
the local intelligence service are all local processes, and model calls go to
providers you configure yourself.

> **Early access:** the [Developer Preview waitlist](https://site-0522a86bd09e48629a1e90f115711758.freebuff.page)
> is open and pre-release / invite-only. No pricing is published during public
> development ([`docs/governance/ZYLCODE_COMMERCIAL_MODEL.md`](docs/governance/ZYLCODE_COMMERCIAL_MODEL.md)).

![ZylCode workbench with a live backend — repository explorer, terminal running real git commands, mission list, capability spotlight and the status bar reporting branch and version](docs/media/zylcode/14-workbench-live.png)

---

## What's new — 6 November 2026

The model layer stopped pretending. Before this milestone the "streaming" path was a
fake line-chunker in `router.rs` that emitted `{index, delta, done}` and nothing else;
provider responses were parsed by string matching in the router, and there was no way
to tell a real stream from an offline imitation.

**Providers now speak their own protocols.** Three commits on this branch (`ebd5098`,
`a202a08`, `fc69e4c`) replace that with a provider boundary:

- **Real wire parsing, per provider.** Anthropic SSE (`content_block_delta`,
  `message_delta` usage, `message_stop`), OpenAI-compatible SSE (`data:` frames and
  `[DONE]`) and Ollama NDJSON are parsed in
  [`anthropic.rs`](crates/zylcode-core/src/providers/anthropic.rs),
  [`openai_compatible.rs`](crates/zylcode-core/src/providers/openai_compatible.rs) and
  [`ollama.rs`](crates/zylcode-core/src/providers/ollama.rs), with shared framing in
  [`sse.rs`](crates/zylcode-core/src/providers/sse.rs).
- **One neutral contract.** [`streaming.rs`](crates/zylcode-core/src/providers/streaming.rs)
  defines `StreamChunk` with `ChunkKind`, `FinishReason`, `Usage`, provider metadata,
  structured `ProviderError`, and an explicit `synthetic` flag, so a consumer never has
  to guess whether a chunk came from a model or from the offline path.
- **Cancellation that actually aborts.** `CancelToken` is polled alongside the response
  body and drops the `reqwest::Response` (closing the connection) instead of leaving the
  request running behind a stopped reader.
- **`router.rs` is orchestration again.** Failover, selection and the offline degrade
  live there; request bodies, headers and completion parsing moved behind
  `parse_completion` in each provider module.
- **Capabilities are data, not marketing.** One table
  ([`capabilities.rs`](crates/zylcode-core/src/providers/capabilities.rs)) describes all
  eight providers, and it says `tools: false` for every one of them (see
  [Limitations](#limitations-and-non-claims)).

81 provider tests cover that boundary, including a loopback HTTP server
([`test_server.rs`](crates/zylcode-core/src/providers/test_server.rs)) that speaks the
providers' real protocols over real sockets.

---

## What it does today

| Stage          | What happens                                                                                                       | State on this branch                                           |
| -------------- | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| **Understand** | Repository index: files, symbols, dependency graph; `zylcode repo-context` answers "what would this change touch?" | Working; benchmark 0.61 against a 0.60 floor (2026-10-29)      |
| **Plan**       | Intent becomes a plan with explicit steps and tool intents                                                         | Working in the core; agent-loop e2e suites green               |
| **Act**        | Tool calls run through the permission gate in `zylcode-mcp`; consequential ones need approval                      | Working; approval-enforcement tests green                      |
| **Observe**    | Live terminal, source control, and streaming model output                                                          | Working; the captures below execute real commands              |
| **Verify**     | Results are checked and recorded; a failed check stays visible as failed                                           | Working; the ledger keeps failures, it does not tidy them away |
| **Evidence**   | Hash-chained records under `.zylcode/`; a broken chain is an error, never an empty screen                          | Working; ledger and tamper tests green                         |

The desktop UI reaches all of that through 39 registered Tauri commands; the same core is
also served to the browser preview by `zylcode serve-intel` on loopback.

---

## Capability matrix

Every row states where it is proven, not just that code exists. Nothing here is a
promise about a hosted service.

| Capability                                      | Status                             | Evidence                                                                                                                                                            |
| ----------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Repository explorer over a live index           | **Verified**                       | Capture `14` (live tree); `serve-intel` warm index: 764 files / 3,187 symbols / 8 packages                                                                          |
| Terminal executing real commands                | **Verified**                       | Capture `16` (`cargo test … providers::` → 81 passed); 51 terminal tests                                                                                            |
| Provider streaming, Anthropic + OpenAI-style    | **Implemented, protocol-tested**   | `providers::anthropic`, `providers::openai_compatible`; loopback SSE tests                                                                                          |
| Provider streaming, Ollama (local)              | **Implemented, protocol-tested**   | `providers::ollama`; loopback NDJSON tests                                                                                                                          |
| Stream cancellation aborts the request          | **Implemented, tested**            | `test_server::Scenario::BodyThenHold` reports socket teardown when the client cancels                                                                               |
| Provider failover across the whole chain        | **Implemented, tested**            | Router failover suites; capture `17` shows the live chain and measured outcomes                                                                                     |
| Offline/synthetic output labelled as such       | **Implemented, tested**            | `StreamChunk.synthetic`, `providers::synthetic`; the offline provider advertises `streaming: false`                                                                 |
| Evidence ledger, hash-chained                   | **Verified**                       | Capture `18` (40 entries, 13 sessions, chain intact); ledger tests                                                                                                  |
| Mission compose → plan → gated tools → evidence | **Implemented and tested in core** | `agent_loop_e2e`, `factory_job_e2e`; the UI journey needs the desktop runtime, so the browser captures show composition only                                        |
| Provider key editing / connection test UI       | **Partial**                        | Backend commands exist (`test_provider_connection`); editing is desktop-only — capture `17` is the read-only chain in preview                                       |
| Native provider tool calling                    | **Not implemented**                | `tools: false` for all 8 providers; no request builder emits `tools`                                                                                                |
| External MCP client (connect to MCP servers)    | **Not implemented**                | `zylcode-mcp` is an in-process registry + permission gate + executors; no JSON-RPC transport exists                                                                 |
| Vision input / log probabilities                | **Not implemented**                | `vision: false`, `logprobs: false`; no request builder sends or surfaces them                                                                                       |
| Computer-use automation                         | **Not implemented**                | Independent status report records the engine as simulation ([`docs/governance/MAIN_STATUS_REPORT_2026-10-11.md`](docs/governance/MAIN_STATUS_REPORT_2026-10-11.md)) |

The authoritative row-by-row ledger, including which of these are disputed, is
[`docs/capability-registry.json`](docs/capability-registry.json): 37 entries —
22 GREEN, **12 DISPUTED**, 2 PARTIAL, 1 externally blocked. Builders do not certify
their own work ([`CONTRIBUTING.md`](CONTRIBUTING.md)).

---

## More of the workbench

Real captures only. All five are the same `v0.2.3` build at `fc69e4c`, running in
browser-preview mode against a live `zylcode serve-intel`, at 1440×900 (2× DPR).
Environment, redaction and per-asset reproduction steps are in
[`docs/media/zylcode/README.md`](docs/media/zylcode/README.md).

**A real engineering workflow — intent typed into the Mission Composer, with the live
repository and terminal beside it:**

![Mission Composer with a real typed intent](docs/media/zylcode/15-mission-intent.png)

**Terminal and tool execution — the provider suite running for real, 81 tests, their
names visible, not a mock-up:**

![Terminal running the provider test suite](docs/media/zylcode/16-terminal-provider-tests.png)

**Provider surface — the live fallback chain, models, which keys are required and which
entries are enabled, plus the in-process tool runtime with its executors bound:**

![Dev Tools providers tab with the live fallback chain and tool catalogue](docs/media/zylcode/17-provider-chain.png)

**Verification and evidence — 40 hash-chained entries across 13 sessions, including the
rows that failed:**

![Evidence ledger with verified, recorded, approved and failed entries](docs/media/zylcode/18-evidence-ledger.png)

Earlier captures — `v0.2.3` (2026-10-29/31) and `v0.2.1` (2026-10-26), including a
workbench with **no** backend attached where panels show their honest `BLOCKED` state —
are listed in the capture record. Selected ones:

| Surface                               | Capture                                                          |
| ------------------------------------- | ---------------------------------------------------------------- |
| Workbench, live backend (batch 3)     | ![Workbench](docs/media/zylcode/11-workbench-live-terminal.png)  |
| Missions + live terminal (batch 3)    | ![Missions](docs/media/zylcode/12-missions-pipeline-live.png)    |
| Source control on the real branch     | ![Source control](docs/media/zylcode/13-source-control-live.png) |
| Workbench without a backend (batch 2) | ![Workbench](docs/media/zylcode/06-mission-lifecycle.png)        |
| Arctic Light theme (current build)    | ![Arctic Light](docs/media/zylcode/10-theme-arctic-light.png)    |

Earlier recordings (2026-10-26 build): [live
theming](docs/media/zylcode/preview-theming.mp4) ·
[panel/dock navigation](docs/media/zylcode/preview-navigation.mp4) ·
[mission composer](docs/media/zylcode/preview-mission-composer.mp4).

---

## Architecture

```
zylcode/
├── apps/
│   ├── zylcode-desktop/        Tauri v2 + React desktop application
│   │   ├── src/                Panels: agent workspace, tool runner, repo
│   │   │                       navigation & intel, terminal, git, settings
│   │   └── src-tauri/          39 registered backend commands (process_intent,
│   │                           execute_tool, repo_context, nav_query,
│   │                           git_status, file_content, …)
│   └── natlas-sdk/             TypeScript SDK + playground for the N-ATLAS
│                               bridge (a separate integration workstream)
├── crates/
│   ├── zylcode-core/           Agent loop, planning, provider boundary,
│   │   │                       router/failover, repository intelligence,
│   │   │                       factory runner, evidence ledger, terminal,
│   │   │                       crash recovery
│   │   └── src/providers/      streaming.rs (neutral contract) · anthropic.rs
│   │                           · openai_compatible.rs · ollama.rs · sse.rs
│   │                           · compat.rs · capabilities.rs · synthetic.rs
│   ├── zylcode-mcp/            In-process tool registry, permission gate,
│   │                           evidence + telemetry, file/git/shell/search
│   │                           executors
│   ├── zylcode-nav/            Repository scanner, symbol index, dependency
│   │                           graph, navigation query engine
│   └── zylcode-cli/            `zylcode` binary: repo-context, serve-intel,
│                               missions, build helpers
├── docs/                       Architecture, governance, roadmap, evidence
└── .github/workflows/          CI (check, clippy, full test suite, build)
```

**The provider boundary** — the part that changed this week:

```
router.rs            selects a provider, walks the fallback chain, degrades offline
   │                 (no wire formats, no JSON field names)
   ▼
providers::anthropic │ openai_compatible │ ollama │ synthetic
   │  builds the request, parses SSE / NDJSON / JSON, maps errors
   ▼
StreamChunk { index, kind, done, finish_reason, synthetic, tool_state, usage,
              metadata, provider_ref, error }        ← the contract
   │
   ├─▶ agent loop / consumers that need the full chunk
   └─▶ compat::StreamEvent { index, delta, done }     ← lossy IPC projection
```

That last hop is deliberately lossy: the desktop IPC payload predates this work and
still carries only `{index, delta, done}`. `StreamChunk` is the source of truth and the
projection is derived from it (`From<&StreamChunk>`), so finish reason, usage,
tool-call deltas, structured errors and synthetic provenance exist on the chunk but are
**not** currently visible in the UI.

How the pieces relate:

- **Application** — the desktop app drives the core over Tauri IPC (39 registered
  commands) and, for the browser preview, over the loopback-only `serve-intel` HTTP
  service.
- **Execution and verification** — tool calls run through the permission gate in
  `zylcode-mcp`; results are verified and written to the hash-chained ledger under
  `.zylcode/`.
- **Repository intelligence** — `zylcode-nav` builds the persisted index feeding agent
  context, `zylcode repo-context`, and the navigation panels.
- **Model integration** — the provider boundary above, with failover and your own keys.
  N-ATLAS is one integration workstream, not the product's identity.

---

## Verification — what was actually run

All rows measured on this head (`fc69e4c`) on 6 November 2026, on Windows, unless the
row says otherwise.

| Check                                                                                    | Result                                                                     |
| ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `cargo test -p zylcode-core --lib`                                                       | ✅ **694 passed, 0 failed**                                                |
| `cargo test -p zylcode-core --lib providers::`                                           | ✅ **81 passed, 0 failed**                                                 |
| `cargo test --workspace --all-targets`                                                   | ✅ **1,069 passed, 0 failed** across 31 test binaries                      |
| `cargo clippy --workspace --all-targets -- -D warnings`                                  | ✅ exit 0                                                                  |
| `cargo fmt --all -- --check`                                                             | ✅ clean                                                                   |
| `pnpm --filter zylcode-desktop build` (tsc + vite)                                       | ✅ exit 0                                                                  |
| `pnpm --filter zylcode-desktop test`                                                     | ✅ **764 passed in 67 files**, exit 0                                      |
| `pnpm --filter zylcode-desktop test:coverage`                                            | ✅ 97.07 / 90.72 / 96.2 / 97.07 (stmts / branches / funcs / lines), exit 0 |
| Rust coverage — `cargo llvm-cov … --fail-under-lines 78` (CI `Coverage` job, 2026-11-04) | ✅ 80.96% lines (42,381 total, 8,070 missed)                               |

Two honesty notes on the rows above:

1. **Protocol tests, not vendor calls.** The streaming suites talk real HTTP to a
   loopback server that emits each provider's documented wire format; `reqwest`, framing
   and socket teardown are real, but no request leaves the machine. No test in this
   repository has yet exercised a live Anthropic, OpenAI or Ollama endpoint, so streaming
   is proven against the protocol, not against a paid vendor account.
2. **Rows are dated.** The Rust coverage figure and the repository-intelligence
   benchmark (0.61, 2026-10-29) come from CI logs of their run date and were not
   re-measured for this head.

---

## Getting started

Prerequisites: Rust stable (MSRV **1.90**, `Cargo.toml`), Node ≥ 18 (CI uses 22),
pnpm ≥ 9 (locked at 9.12.0). Linux desktop builds need the usual Tauri libraries
(GTK/WebKit).

```bash
git clone https://github.com/zylvex-tech/zylcode.git
cd zylcode
pnpm install --frozen-lockfile

pnpm --filter zylcode-desktop dev        # browser preview (Vite)
pnpm --filter zylcode-desktop tauri dev  # desktop window (first run compiles Rust)

cargo run -p zylcode-cli -- serve-intel  # read-only intelligence API on :17630
```

The last two are what the browser preview needs together: the UI proxies `/api` to the
loopback service, which is how the explorer, terminal and evidence views in the captures
above report real state. Without it they show `BLOCKED` rather than inventing data.

A container with the whole toolchain is included
(`docker compose run --rm dev`), plus a [`.devcontainer`](.devcontainer/devcontainer.json)
for VS Code / Cursor. More detail:
[`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md),
[`docs/TUTORIALS.md`](docs/TUTORIALS.md),
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md),
[`docs/VERIFICATION.md`](docs/VERIFICATION.md). Platform installation notes for
Windows, macOS and Linux are in [`docs/INSTALLATION_GUIDE.md`](docs/INSTALLATION_GUIDE.md).

### Tests and quality gates

```bash
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python scripts/check_retracted_claims.py   # keeps retracted claims from returning

pnpm --filter zylcode-desktop build
pnpm --filter zylcode-desktop test
pnpm --filter zylcode-desktop test:coverage
```

These are floors, not aspirations — CI fails below them, and each floor sits just under
its measured value so a new untested file turns the run red instead of being absorbed.

| Gate                                   | Floor                | Measured                                         |
| -------------------------------------- | -------------------- | ------------------------------------------------ |
| Frontend coverage — lines & statements | 70%                  | **97.07%** (764 tests, 67 files — 2026-11-06)    |
| Frontend coverage — branches           | 83%                  | **90.72%** (2026-11-06)                          |
| Frontend coverage — functions          | 80%                  | **96.2%** (2026-11-06)                           |
| Rust coverage — lines                  | 78%                  | **80.96%** (CI `Coverage` job, 2026-11-04)       |
| Repository-intelligence benchmark      | 0.60 precision floor | **0.61** (recall 0.95 — 2026-10-29)              |
| Workspace tests                        | 0 failures           | **1,069 passed** across 31 binaries (2026-11-06) |

Gates live in [`.github/workflows/coverage.yml`](.github/workflows/coverage.yml) (Rust
`coverage`, vitest `frontend-coverage`) and
[`apps/zylcode-desktop/vitest.config.ts`](apps/zylcode-desktop/vitest.config.ts);
[`ci.yml`](.github/workflows/ci.yml) runs tests, clippy, fmt and the
retracted-claim guard. The benchmark floor sits in
[`crates/zylcode-core/tests/repo_intelligence_benchmark.rs`](crates/zylcode-core/tests/repo_intelligence_benchmark.rs).

---

## Limitations and non-claims

Things a reader might reasonably assume, which this repository does **not** do yet:

- **No native tool calling.** `tools: false` for every provider. The APIs support it;
  this client builds no `tools` block and translates no tool-call stream. Tool execution
  happens through ZylCode's own in-process gate, which is a different (and working) thing.
- **No external MCP client.** `zylcode-mcp` is a tool registry, permission gate and
  executor set inside the process. There is no JSON-RPC transport, no server discovery and
  no `initialize` handshake anywhere in the tree. You cannot point ZylCode at an MCP server.
- **Streaming is not fully surfaced.** The IPC projection carries only
  `{index, delta, done}`; finish reason, usage, tool deltas, structured errors and the
  `synthetic` flag stop at the boundary. The UI cannot yet show why a response ended.
- **Provider editing is desktop-only.** The browser preview renders the live chain
  read-only (capture `17`); changing keys or testing a connection needs the Tauri runtime.
- **Acceptance is not self-granted.** 12 of 37 registry entries are disputed and Phase 2A
  re-acceptance is open; independent acceptance remains outstanding.
- **Computer use is simulation.** Nothing drives a screen
  ([`docs/governance/MAIN_STATUS_REPORT_2026-10-11.md`](docs/governance/MAIN_STATUS_REPORT_2026-10-11.md)).
- **No signed installers.** Source checkout is the supported path today.
- **No published website or pricing.** The waitlist link above is the only public URL in
  this repository; nothing else should be inferred from it.

---

## Project status and roadmap

- **Now:** a working desktop shell and CLI over a tested core: provider streaming with
  real wire parsing, request-aborting cancellation, failover, an evidence ledger that
  keeps its failures; 1,069 Rust tests, 764 frontend tests, clippy/fmt clean on this head.
- **In progress:** Phase 2A re-acceptance evidence
  ([`docs/governance/PHASE_2A_REACCEPTANCE_EVIDENCE_2026-09-18.md`](docs/governance/PHASE_2A_REACCEPTANCE_EVIDENCE_2026-09-18.md));
  surfacing the full `StreamChunk` across IPC; honest distribution.
- **Planned:** everything in
  [`docs/roadmap/ZYLCODE_ROADMAP_V2.md`](docs/roadmap/ZYLCODE_ROADMAP_V2.md) —
  repository reasoning (Phase 2B, blocked on 2A), project system, design canvas,
  extension platform. None of it is claimed as working here.
- **Early access:** [Developer Preview waitlist](https://site-0522a86bd09e48629a1e90f115711758.freebuff.page)
  — free / invite-only for the first cohort.

Full phase-by-phase status: [`docs/PROJECT_MASTER_STATUS.md`](docs/PROJECT_MASTER_STATUS.md).

---

## N-ATLAS integration

The **N-ATLAS developer bridge** is a specific integration and competition workstream,
not the product's identity: it routes instructions (English, Yoruba, Hausa, Igbo — with
Nigerian Pidgin validated as a non-target bonus) through the genuine `NCAIR1/N-ATLaS`
model into the approval-gated pipeline, recording the chain as evidence.

- **Dedicated branch:** <https://github.com/zylvex-tech/zylcode/tree/competition/natlas-2026>
- **Documentation and evidence on this branch:** [`docs/competition/natlas/`](docs/competition/natlas/),
  including [`KNOWN_LIMITATIONS.md`](docs/competition/natlas/KNOWN_LIMITATIONS.md) and
  [`LOCAL_RUNTIME_FALLBACK.md`](docs/competition/natlas/LOCAL_RUNTIME_FALLBACK.md).
- **SDK and playground:** [`apps/natlas-sdk/`](apps/natlas-sdk/).

**Attribution and model licence.** The N-ATLaS model (`NCAIR1/N-ATLaS`, a Llama-3 8B SFT
fine-tune) was developed by **Awarri Technologies** with **FMCIDE** and **NCAIR** under
the supervision of **NITDA**, under the _N-ATLaS Open-Source Research and Innovation
License_ (≤ 1,000 active end-users; attribution to Awarri + FMCIDE required). **No model
weights are committed here**, and the model's licence is separate from this codebase's.

---

## Contributing, security, licensing

- **Contributing:** [`CONTRIBUTING.md`](CONTRIBUTING.md) — including the evidence rule:
  builders do not certify their own work.
- **Code of conduct:** [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).
- **Security:** [`SECURITY.md`](SECURITY.md) · **Support:** [`SUPPORT.md`](SUPPORT.md).
- **License:** **Apache License 2.0** — see [`LICENSE`](LICENSE). (Earlier revisions of
  this README said "MIT"; the licence file and GitHub metadata both say Apache-2.0.)

ZylCode is developed by **Zylvex Technologies Ltd.** There are no customer logos,
endorsements or partnership claims on this page, because none are verified in this
repository.

<p align="center">
  <strong>Built with evidence. Verified with honesty.</strong><br>
  <em>ZylCode — Software Creation Operating System · Zylvex Technologies Ltd.</em>
</p>
