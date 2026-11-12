# Native tool calling — lifecycle, surfaces, and what is actually verified

This note records the real path a provider tool call takes through ZylCode, and
which surfaces and providers have been exercised against it. It exists because
"the provider accepts a `tools` field" and "the tool actually ran" are different
claims, and only the second one is worth shipping.

## The lifecycle

```
provider declares tools (from the real catalogue, executables only)
  → model returns a native tool call, addressed by a wire-safe name (`fs_read`)
  → the wire name resolves to a canonical id (`fs.read`); unknown names fail closed
  → arguments must be a JSON object, or the call is refused before the gate
  → real_tools::dispatch — the same permission gate and ToolRegistry every other
    surface uses, plus the evidence sink
  → the tool result is sent back as a provider-native tool result carrying the
    call id the provider issued
  → the conversation continues; the model answers from what it actually received
```

There is one native loop (`zylcode-core::native_tools::NativeToolRunner`) and
one executor (`zylcode_mcp::real_tools::dispatch`). The CLI and the desktop app
both call the loop; neither has its own copy.

The older `AgentLoop` remains for providers that cannot be handed tool
declarations. It reaches the same gate and registry, but it asks the model to
format a decision as JSON in prose. That is the fallback, and it is reported as
such: a prompt-JSON tool call is not a native tool call. A model-requested call is held to the
same policy as a human-requested one — `approval_required` is false for a
model-requested call, so the restrictive gate keeps refusing anything above
`Read` instead of being handed an approval nobody gave.

## Surfaces

| Surface      | Entry point                     | Status                | Evidence                                                                                                                                                                                                                               |
| ------------ | ------------------------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CLI          | `zylcode ask "<task>"`          | `VERIFIED_RUNTIME`    | Run twice against a local OpenAI-compatible endpoint: 20 declarations, `fs_read` → `fs.read`, gate `allow`, file content returned, ledger written, second round answered from it                                                       |
| Desktop chat | `process_intent_stream` (Tauri) | `VERIFIED_LOCAL_ONLY` | `native_chat::tests::desktop_session_runs_the_gated_tool_lifecycle` drives the same session function through a real executor, real gate, real ledger and a scripted provider. Not yet driven through Tauri IPC in a running app window |
| Fallback     | `AgentLoop` / `dispatch_prompt` | `PARTIAL`             | For providers that cannot be handed declarations. Same gate, same registry, but the model is asked to emit a JSON decision in prose; the transport carries no tool schema. A prompt-JSON call is not a native call                     |

## Providers

`tools: true` in the capability table is an **implementation claim**: a request
builder emits the declaration and a decoder reads the call back. It has never
been a runtime claim, and reporting it as one was the defect this section
corrects. The table below reports each stage of the lifecycle separately, and the
same values live in code (`providers::capabilities::tool_call_evidence`) so the
document cannot drift from the implementation.

| Stage                      | OpenAI-compatible       | Anthropic               | Ollama                  | Synthetic |
| -------------------------- | ----------------------- | ----------------------- | ----------------------- | --------- |
| Tool declaration           | `UNIT_TESTED`           | `UNIT_TESTED`           | `UNIT_TESTED`           | —         |
| Call decoding              | `UNIT_TESTED`           | `UNIT_TESTED`           | `UNIT_TESTED`           | —         |
| Streamed call reassembly   | `UNIT_TESTED`           | `UNIT_TESTED`           | `UNIT_TESTED`           | —         |
| Canonical resolution       | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Execution                  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Permission gate            | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Evidence recording         | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Tool result return         | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Continuation round         | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | `UNIT_TESTED` (shared)  | —         |
| Whole turn on the wire     | `IMPLEMENTED_ONLY`      | `IMPLEMENTED_ONLY`      | `IMPLEMENTED_ONLY`      | —         |
| Through the desktop window | `UNIT_TESTED` (session) | `UNIT_TESTED` (session) | `UNIT_TESTED` (session) | —         |

Reading that table honestly:

- **No provider is `LIVE_VERIFIED`.** No recorded run against a real Anthropic,
  OpenAI, OpenRouter, DeepSeek, Respan or Ollama endpoint exists in this
  repository. The CLI proof in the surface table ran against a local server that
  speaks the OpenAI wire, which tests this client's translation and the loop, not
  the vendor's behaviour.
- **No surface is `DESKTOP_VERIFIED`.** The desktop test stops at the session
  function; Tauri IPC and the window are untested.
- Anthropic and Ollama are not weaker than OpenAI-compatible in evidence — they are
  identical. Every difference previously drawn between them in this document was
  overstatement, and the honest fix is to flatten it rather than defend it.
- The three wire stages are tested per provider against synthetic frames. The
  stages after them are provider-independent code with one test suite, so they are
  reported as shared.
- Synthetic offline is `NOT_IMPLEMENTED` at every stage by design: it has no wire,
  so declaring tools would promise a call it cannot make. It is deterministic and
  local, which is why it is the provider used for offline tests, not a tool host.

Promotion rule, enforced in code (`evidence_tests`): a stage may say
`LIVE_VERIFIED` only for a provider listed in `LIVE_RUNS_RECORDED`, and
`DESKTOP_VERIFIED` only for one listed in `DESKTOP_RUNS_RECORDED`. Both lists are
empty today, and the test fails if a row is promoted without an entry.

Vision input is `NOT_IMPLEMENTED` on every provider. Tool calling must not be
read as multi-modal support.

## Evidence

Every executed call — including a refusal — writes an evidence record with the
tool id, the actor, the gate's decision, the invocation id and the outcome.

- CLI: `ZYLCODE_EVIDENCE_LOG`, else `.zylcode/evidence.jsonl` relative to the
  process working directory.
- Desktop: the application settings directory
  (`%APPDATA%\ZylCode\evidence.jsonl` on Windows, `ZYLCODE_CONFIG_DIR`
  overrides). A packaged app's working directory is wherever the launcher
  happened to be, so the desktop sink does not use it.

The remaining CWD-relative default is a known defect of the CLI sink and belongs
to the unified-data-layer work, not to this seam.

## The desktop seam

`apps/zylcode-desktop/src-tauri/src/native_chat.rs` is thin on purpose:

1. the capability table decides whether the provider may be handed tools;
2. the plan's system prompt and compiled prompt feed the loop;
3. the tool context names the workspace and the actor `desktop:chat`;
4. the evidence sink points at the application directory;
5. each executed call is emitted on `zylcode://tool-call` with the canonical id,
   the gate decision and the invocation id.

The renderer never decides whether a tool may run. It receives an already-made
decision and displays it; a refusal arrives as `status: "failed"` with the gate's
reason, so the UI cannot render a denial as a success.

Cancellation is a shared slot (`CancelSlot`). The token is checked before each
provider round and between tool calls, so cancelling stops the next provider
continuation rather than leaving one in flight.
