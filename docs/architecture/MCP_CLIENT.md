# MCP client

`crates/zylcode-mcp/src/mcp_client/` is ZylCode's MCP **client**: it spawns or
connects to external MCP servers, discovers their tools, and executes them
through the same permission gate and evidence trail as the built-in tools.

Before this module existed, `zylcode-mcp` held a tool catalogue, in-process
executors, a permission gate and an evidence log — but no protocol. A
`transport: Stdio` entry in configuration was a label that nothing read.

## Layers

| Layer       | Module         | Responsibility                                                                             |
| ----------- | -------------- | ------------------------------------------------------------------------------------------ |
| Framing     | `jsonrpc.rs`   | JSON-RPC 2.0 envelopes, id correlation, malformed-message errors                           |
| Wire        | `transport.rs` | stdio (spawn, frame, capture stderr) and Streamable HTTP (POST/GET, SSE, `Mcp-Session-Id`) |
| Lifecycle   | `session.rs`   | `initialize` → `initialized` → `tools/list` → `tools/call`, `ping`, cancellation           |
| Config      | `config.rs`    | persisted `mcp.servers.json`, validated as a security boundary                             |
| Supervision | `registry.rs`  | start/restart/backoff, discovery cache, status snapshot                                    |
| Adapter     | `remote.rs`    | a discovered tool as a `RealTool`, dispatched through `real_tools::dispatch_with`          |

## Protocol revision, stated honestly

Implemented: the handshake-era lifecycle for **`2025-06-18`** and
**`2025-03-26`**.

Not implemented: **`2026-07-28`**, which removed the handshake in favour of
per-request metadata. A server that selects it is refused with the versions this
client does offer (`ProtocolVersionRejected`), never silently downgraded.

## One dispatch path

A remote tool does not get its own execution framework. `RemoteTool` implements
`RealTool`, and `real_tools::dispatch_with` performs attribution, risk
classification, the permission gate, evidence recording and the refusal record —
the same code the built-in tools run through. A refusal is recorded as evidence
and the server is never contacted; an integration test proves this by asserting
the fixture server's observation log contains no `tools/call`.

## Risk is derived, never declared

MCP annotations are hints from the server, and a server may omit them or lie.
The mapping fails closed:

| Server hint                | Risk                         |
| -------------------------- | ---------------------------- |
| `destructiveHint: true`    | `Destructive`                |
| `readOnlyHint: true`       | `Read`                       |
| `openWorldHint: true`      | `Network`                    |
| _(none, or contradictory)_ | `Write` — approval-requiring |

An unannotated tool is therefore never treated as harmless because nobody said
otherwise.

## Server states

```
Disabled ──────────────────────────────┐
Stopped ──start──▶ Starting ──ok──▶ Ready
   ▲                  │              │
   │                fail           crash
   │                  ▼              ▼
   └────────────── Failed ◀──── Restarting ──ok──▶ Ready
```

`restart.max_attempts` counts **consecutive** failures, so one successful start
restores the budget. `Failed` is terminal for the current configuration; a
broken server cannot spin forever. Only **ready** servers advertise tools —
listing a tool that cannot run is the stub behaviour this repository rejects.

## No blind retry of a tool call

A transport failure during `tools/call` is ambiguous: the server may have
executed the tool and lost the response. Retrying would run a side-effecting
tool twice, which is worse than failing. A failed call therefore marks the
session dead and reconnects **for the next invocation**; the failed call is
reported as failed. `ping` is the only retried operation, because it has no
effect.

## Verification

`crates/zylcode-mcp/tests/mcp_client.rs` drives
`tests/support/mcp_test_server.rs`, a real child process that can reject
versions, page results, write junk to stdout, sleep, and exit mid-session.

Covered: handshake and server identity, `notifications/initialized` ordering,
two-page discovery, annotation-to-risk mapping (including fail-closed),
`tools/call` content and structured output, unknown-tool errors, timeout plus
wire-level cancellation, crash → reported failure → reconnect, exhausted retries
reported as `Failed`, and gated dispatch for both refusal and allowance with the
evidence file asserted.

## Not implemented

- No `resources/*` or `prompts/*`; only tools are discovered.
- No client-side sampling, roots or elicitation. Server requests for those are
  answered `-32601` and counted in the session.
- No OAuth. A remote server needing it must be reached through a local gateway
  or a static header.
