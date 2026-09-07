# Fault Injection Test Results

> **Date**: 2026-09-07
> **Test crate**: `crates/zylcode-mcp/tests/fault_injection.rs`
> **Suite**: 16 tests across 5 categories
> **Status**: All passing

## Summary

The MCP integration layer undergoes systematic fault injection to validate resilience against real-world failure modes. Every test uses mock tools with deterministic failure triggers — no external processes, no flaky network calls.

| Category | Tests | Status |
|----------|-------|--------|
| Transport Drop | 2 | ✅ All pass |
| SSE Reconnection | 2 | ✅ All pass |
| Malformed Payloads | 4 | ✅ All pass |
| Config Hot-Reload | 2 | ✅ All pass |
| Retry/Backoff/Timeout | 4 | ✅ All pass |
| Concurrent Stress | 2 | ✅ All pass |

---

## Category 1: Transport Drop (SIGKILL / Process Termination)

**Scenario**: A stdio subprocess is killed mid-operation (e.g., OOM killer, user kill, crash).

### `transport_drop_stdio_graceful_cleanup`
- **Setup**: `StdIoDropTool` with `should_drop: Arc<RwLock<bool>>` flag
- **Flow**: Call 1 succeeds → enable drop flag → Call 2 returns `Err("SIGKILL: process terminated unexpectedly")`
- **Assertion**: Error contains `"SIGKILL"` or `"process terminated"` — no panic, no unwrap-crash
- **Verdict**: ✅ Graceful error propagation confirmed

### `transport_stdio_timeout_retries_then_fails`
- **Setup**: `StdioTimeoutTool` with 2s internal delay, 50ms timeout, 2 retries
- **Flow**: Tool sleeps 2s, executor times out at 50ms, retries 2x, then returns error
- **Assertion**: Error contains `"timed out"`, elapsed > 150ms (3 attempts × 50ms + backoffs), elapsed < 2s
- **Verdict**: ✅ Timeout enforced per attempt, retries respected

---

## Category 2: SSE Reconnection & Disconnect

**Scenario**: An SSE stream drops mid-session and the client must reconnect transparently.

### `sse_disconnect_reconnects_and_succeeds`
- **Setup**: `SseDisconnectTool` with `disconnect_on_attempt: 1` (first call fails)
- **Flow**: Attempt 1 → `"SSE stream closed unexpectedly"` → retry → Attempt 2 succeeds with `{"attempt": 2, "ok": true}`
- **Assertion**: Result is `Ok`, value has `attempt == 2`
- **Verdict**: ✅ Transparent reconnection works

### `sse_permanent_failure_after_max_retries`
- **Setup**: `AlwaysFailTool` that always returns `"SSE stream closed unexpectedly"`, `max_retries: 2`
- **Flow**: 4 attempts total (1 initial + 3 retries), all fail
- **Assertion**: Error contains `"SSE stream closed"`, result is `Err`
- **Verdict**: ✅ Permanent failures surface after retry budget exhausted

---

## Category 3: Malformed Payloads

**Scenario**: A tool subprocess returns garbage, truncated, or structurally invalid responses.

### `malformed_truncated_json_returns_structured_error`
- **Failure mode**: `MalformedMode::TruncatedJson` — simulates broken pipe during JSON parse
- **Error**: `"unexpected EOF while parsing JSON response"`
- **Verdict**: ✅ Structured error, not a panic

### `malformed_invalid_type_returns_structured_error`
- **Failure mode**: `MalformedMode::InvalidType` — type mismatch in response
- **Error**: `"expected string, got integer"`
- **Verdict**: ✅ Type errors propagate cleanly

### `malformed_missing_field_returns_structured_error`
- **Failure mode**: `MalformedMode::MissingField` — required field absent
- **Error**: `"missing required field 'result'"`
- **Verdict**: ✅ Schema violations caught

### `malformed_non_utf8_returns_structured_error`
- **Failure mode**: `MalformedMode::NonUtf8` — binary garbage in response
- **Error**: `"invalid UTF-8 sequence in response"`
- **Verdict**: ✅ Encoding errors handled

---

## Category 4: Config Hot-Reload Failure Recovery

**Scenario**: A YAML config file is modified at runtime with invalid content.

### `config_hot_reload_malformed_yaml_retains_lkg`
- **Setup**: Register 1 tool from valid YAML → overwrite file with malformed YAML → attempt reload
- **Flow**: `McpConfigFile::from_path()` returns `Err` on malformed YAML
- **Assertion**: Original registry still has 1 tool (`"stable"`) — Last Known Good (LKG) config retained
- **Verdict**: ✅ Malformed config does not destroy existing tool registry

### `config_hot_reload_valid_update_works`
- **Setup**: Register 1 tool → overwrite with valid 2-tool config → reload
- **Flow**: Registry cleared + re-registered, now has 2 tools
- **Assertion**: Both `tool1` and `tool2` present
- **Verdict**: ✅ Valid hot-reload adds new tools

---

## Category 5: Retry Backoff & Timeout Verification

**Scenario**: Transient failures with exponential backoff and per-attempt timeout enforcement.

### `retry_backoff_250ms_exponential`
- **Setup**: `SlowFailingTool` fails on attempts 1–2, succeeds on 3. Base backoff: 250ms.
- **Flow**: Attempt 1 fail → sleep 250ms → Attempt 2 fail → sleep 500ms → Attempt 3 succeed
- **Assertion**: Elapsed ≥ 700ms (250+500), attempt count == 3
- **Verdict**: ✅ Exponential backoff confirmed

### `retry_backoff_max_retries_respected`
- **Setup**: `SlowFailingTool` with `max_attempts: 10` (never succeeds within budget), `max_retries: 3`
- **Flow**: 4 attempts total (1 initial + 3 retries), all fail
- **Assertion**: attempt count == 4, elapsed ≥ 550ms (100+200+300)
- **Verdict**: ✅ Retry budget enforced

### `timeout_enforced_per_attempt`
- **Setup**: `StdioTimeoutTool` with 200ms delay, 50ms timeout, 1 retry
- **Flow**: Attempt 1 timeout (50ms) → retry → Attempt 2 timeout (50ms) → fail
- **Assertion**: Elapsed ≥ 100ms, elapsed < 200ms
- **Verdict**: ✅ Timeout enforced per individual attempt, not cumulative

### `cancellation_token_propagates`
- **Setup**: `StdioTimeoutTool` with 10s delay, 50ms timeout, 0 retries
- **Flow**: Spawn task → abort after 100ms → verify task is cancelled
- **Assertion**: `handle.await` returns `Err` (JoinError from abort)
- **Verdict**: ✅ Tokio cancellation propagates correctly

---

## Category 6: Concurrent Stress

**Scenario**: Race conditions between tool execution and registry mutation.

### `concurrent_tool_execution_no_crosstalk`
- **Setup**: Register 100 tools, spawn 50 concurrent tasks each calling a random tool
- **Flow**: Each task calls `registry.get()` then `tool.call()`, asserts result matches tool ID
- **Verdict**: ✅ No cross-talk between concurrent tool executions

### `concurrent_registry_modification_during_execution`
- **Setup**: 20 initial tools, 5 writer tasks (register 10 + clear), 30 reader tasks (list + call)
- **Flow**: Writers and readers execute concurrently for ~50 batches
- **Verdict**: ✅ Registry remains consistent under concurrent read/write stress

---

## Infrastructure Notes

- **Mock tools**: `StdIoDropTool`, `StdioTimeoutTool`, `SseDisconnectTool`, `AlwaysFailTool`, `MalformedPayloadTool`, `SlowFailingTool` — all implement `Tool` trait with deterministic failure modes
- **Counters**: `AtomicUsize` for attempt tracking, `Arc<RwLock<bool>>` for drop flags
- **Config tests**: Use `tempfile::NamedTempFile` for isolated YAML files
- **No external processes**: All tests are in-process mocks — zero network, zero subprocess spawning
- **Runtime**: `tokio::test` with `#[tokio::test]` macro

---

## Failures Observed During Development

| Date | Test | Issue | Resolution |
|------|------|-------|------------|
| 2026-09-03 | `transport_drop_stdio_graceful_cleanup` | Initial impl used `unwrap()` on error, causing panic | Changed to `.map_err()` with structured error propagation |
| 2026-09-03 | `retry_backoff_250ms_exponential` | Backoff was linear, not exponential | Fixed multiplier in `execute_with_recovery` |
| 2026-09-04 | `config_hot_reload_malformed_yaml_retains_lkg` | LKG registry was cleared on failed reload | Added early-return on `Err` before clearing registry |
| 2026-09-05 | `concurrent_registry_modification_during_execution` | Flaky: registry clear during list caused index OOB | Added `if !tools.is_empty()` guard in reader tasks |

---

*This document reflects real test output as of 2026-09-07. Tests are run on every commit via `cargo test --workspace`.*
