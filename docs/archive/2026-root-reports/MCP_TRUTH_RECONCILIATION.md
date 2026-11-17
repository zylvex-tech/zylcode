# MCP Truth Reconciliation

## What Actually Exists (as of HEAD `d6ab7c8`)

### MCP Client ✅ IMPLEMENTED
The `crates/zylcode-mcp/src/mcp_client/` module contains a **real MCP client** with:

1. **Transports** (both implemented):
   - **StdioTransport**: Spawns a child process, sends/receives newline-delimited JSON-RPC on stdin/stdout
   - **StreamableHttpTransport**: POST for client→server messages, GET for optional SSE server→client stream
   
2. **JSON-RPC 2.0** message formatting/classification (`mcp_client/jsonrpc.rs`)

3. **Session lifecycle** (`mcp_client/session.rs`):
   - `initialize` → `notifications/initialized` → `tools/list` → `tools/call`
   - `ping`, cancellation support
   - Protocol version negotiation (`MCP-Protocol-Version` header)
   - Session ID capture from `initialize` response

4. **Configuration** (`mcp_client/config.rs`):
   - `mcp.servers.json` with strict validation
   - Server transport kinds: `Stdio` or `Http`
   - Startup policies: `Eager`/`Lazy`/`Manual`
   - Restart policies with exponential backoff
   - Cleartext HTTP loopback-only security boundary

5. **Remote tool adaptation** (`mcp_client/remote.rs`):
   - `RemoteTool` adapts remote calls through the **same** `PermissionGate` and evidence sink as native tools
   - Calls pass through `real_tools::dispatch_with`, same as `fs.read`

6. **What IS implemented**:
   - ✅ Stdio and streamable HTTP transports
   - ✅ JSON-RPC 2.0 envelope framing
   - ✅ Handshake-era protocol (`2025-03-26` and `2025-06-18`)
   - ✅ `tools/list` and `tools/call`
   - ✅ Session management (id, protocol version, restart/backoff)
   - ✅ Config file validation (loopback-only HTTP)
   - ✅ Remote tool → same permission gate + evidence sink

7. **What is NOT implemented** (explicitly stated in code):
   - ❌ **OAuth** — "A remote server needing it must be reached through a local gateway or configured with a static header."
   - ❌ **`resources/*` or `prompts/*`** — "Only tools are discovered."
   - ❌ **Client-side sampling, roots or elicitation**
   - ❌ **Live server connection** — no real MCP server run in tests or CI

### MCP Config Files ✅ IMPLEMENTED

1. **`mcp.tools.yaml`** — Configures ZylCode's **in-process tools** (20 executable entries):
   - `fs.read`, `fs.write`, `fs.list`, `shell.execute`, `shell.echo`, `npm.run`, `cargo.test`
   - `git.status`, `git.diff`, `git.commit`
   - `search.find`, `search.grep`
   - 8 `nav.*` repository-intelligence tools
   - **Invariant**: Every id under `tools:` has a real executor (`get_real_tool(id).is_some()`)
   - 21 definition-only entries under `proposed:` (no executor)

2. **`mcp.servers.json`** — Configures **external MCP servers** (not yet populated with real servers):
   - Server transport: `Stdio` (program + args) or `Http` (URL)
   - Validation: cleartext HTTP only for loopback hosts
   - Startup policies: `Eager`/`Lazy`/`Manual`
   - Restart policies with backoff

### Tool Catalogue ✅ IMPLEMENTED

- `crates/zylcode-mcp/src/tool_catalogue.rs` — Canonical source of truth
- **5 explicit metrics** (no aggregate "tool count"):
  - `definition_count`: 46 (20 executable + 27 bridge definitions - 1 overlap)
  - `executable_count`: 20 (12 general-purpose + 8 repository-intelligence `nav.*`)
  - `tested_execution_count`: 10 (`fs.read`, `shell.execute`, plus 8 `nav.*` tools)
  - `product_reachable_count`: 19 (`git.commit` is NOT product-reachable)
  - `r3_verified_count`: 0 (no R3 commission yet)

- **Executable entries** (all have real executors in `real_tools.rs`):
  `fs.read`, `fs.write`, `fs.list`, `shell.execute`, `shell.echo`, `npm.run`, `cargo.test`
  `git.status`, `git.diff`, `git.commit`, `search.find`, `search.grep`
  `nav.find_definition`, `nav.find_references`, `nav.find_callers`, `nav.find_callees`
  `nav.file_dependencies`, `nav.file_dependents`, `nav.symbol_impact`, `nav.repository_graph_query`

- **Definition-only entries** (no executor — preserved as metadata):
  `git.push`, `git.pull`, `npm.install`, `docker.build`, `docker.run`, `k8s.get`, `k8s.apply`
  `db.query`, `code.analyze`, `code.format`, `code.lint`, `test.run`, `test.single`
  `build.debug`, `build.release`, `perf.bench`, `docs.generate`, `security.scan`
  `slack.send`, `ai.train`, `ai.evaluate`

### Permission Gate ✅ IMPLEMENTED

- `crates/zylcode-mcp/src/permission.rs` — Full permission decision engine
- **Restrictive default**: deny above `Read`; Write/Execute require explicit approval
- **Precedence**: explicit deny > explicit allow > human approval > risk threshold
- **Evidence recording**: `ToolEvidence::approval_decision` is populated on every gate decision

### Native Tools + Executors ✅ IMPLEMENTED

- `crates/zylcode-core/src/native_tools.rs` — Runtime types (`ChatTurn`, `NativeRun`, `ExecutedCall`)
- `crates/zylcode-mcp/src/real_tools.rs` — Dispatch pipeline (`dispatch`, `dispatch_with`, `get_real_tool`)
- **7 real executors** (all traced through the gated path):
  - `fs.read` (Read), `fs.write` (Write), `fs.list` (Read)
  - `shell.execute` (Execute, unbound escape hatch), `shell.echo` (Execute)
  - `npm.run` (Execute, bound to `npm run`), `cargo.test` (Execute, bound to `cargo test`)
  - `git.status` (Read), `git.diff` (Read), `git.commit` (GitWrite)
  - `search.find` (Read), `search.grep` (Read)
- **8 repository-intelligence executors** (via `zylcode-nav`): `nav.find_definition`, etc.

### What's MISSING (the gaps)

1. **No live MCP server** — No test or CI exercise of actual MCP server connection
2. **No OAuth** — Explicitly not implemented; would require local gateway
3. **No Composio integration** — Not in the codebase at all
4. **No Floot integration** — Not in the codebase at all
5. **No per-user Composio identity** — No user_id → Composio user_id mapping
6. **No Composio tool discovery UI** — No Settings → Integrations panel
7. **No Floot MCP endpoint configuration** — No ZylCode → Floot Connect flow
8. **No mission engine** — No missions, plans, checkpoints, resumability
9. **No browser agent** — No controlled browser execution layer
10. **No deployment providers** — No Docker, SSH, Floot, or generic HTTPS deployment
11. **No application twin** — No structured project graph/intelligence

## Contradiction Resolution: Previous Report

The previous report claimed:

> "MCP client integrated — initialize → tools/list → tools/call → result"

> "No JSON-RPC transport exists"

> "Live MCP server connection has not been exercised"

**Resolution**: JSON-RPC transport **does exist** (both stdio and HTTP). The contradiction arose from conflating "no live server connection" with "no transport." The transport is implemented but untested against a real server.

**Accurate classification**:
- MCP client implementation: **IMPLEMENTED + NOT TESTED** (no live connection)
- OAuth: **NOT IMPLEMENTED** (explicitly stated)
- Resources/prompts: **NOT IMPLEMENTED** (only tools)
- Live MCP connection: **NOT TESTED** (no server in CI/tests)
