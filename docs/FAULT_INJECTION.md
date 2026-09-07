# Fault Injection Test Design

> **Test crate**: `crates/zylcode-mcp/tests/fault_injection.rs`
> **Suite**: 16 tests across 5 categories
> **Purpose**: Validate resilience against real-world MCP failure modes

## Summary

The MCP integration layer undergo systematic fault injection to validate resilience against real-world failure modes. Every test uses mock tools with deterministic failure triggers — no external processes, no flaky network calls.

These are **design-target specifications**, not records of historical test runs. Each section describes what the test exercises and what it validates; results are reported by `cargo test --workspace` and should be re-run to confirm current status.

| Category | Tests | Status |
|----------|-------|--------|
| Transport Drop | 2 | Target: all pass |
| SSE Reconnection | 2 | Target: all pass |
| Malformed Payloads | 4 | Target: all pass |
| Config Hot-Reload | 2 | Target: all pass |
| Retry/Backoff/Timeout | 4 | Target: all pass |
| Concurrent Stress | 2 | Target: all pass |

---

## Category 1: Transport Drop (SIGKILL / Process Termination)

**Scenario**: A stdio subprocess is killed mid-operation (e.g., OOM killer, user kill, crash).

### `transport_drop_stdio_graceful_cleanup`
- **Setup**: `StdIoDropTool` with `should_drop: Arc<RwLock<bool>>` flag
- **Flow**: Call 1 succeeds → enable drop flag → Call 2 returns `Err("SIGKILL: process terminated unexpectedly")`
- **Assertion**: Error contains `"SIGKILL"` or `"process terminated"` — no panic, no unwrap-crash
- **Design target**: Graceful error propagation confirmed

### `transport_stdio_timeout_retries_then_fails`
- **Setup**: `StdioTimeoutTool` with 2s internal delay, 50ms timeout, 2 retries
- **Flow**: Tool sleeps 2s, executor times out at 50ms, retries 2x, then returns error
- **Assertion**: Error contains `"timed out"`, elapsed > 150ms (3 attempts × 50ms + backoffs), elapsed < 2s
- **Design target**: Timeout enforced per attempt, retries respected

---

## Category 2: SSE Reconnection & Disconnect

**Scenario**: An SSE stream drops mid-session and the client must reconnect transparently.

### `sse_disconnect_reconnects_and_succeeds`
- **Setup**: `SseDisconnectTool` with `disconnect_on_attempt: 1` (first call fails)
- **Flow**: Attempt 1 → `"SSE stream closed unexpectedly"` → retry → Attempt 2 succeeds with `{"attempt": 2, "ok": true}`
- **Assertion**: Result is `Ok`, value has `attempt == 2`
- **Design target**: Transparent reconnection works

### `sse_permanent_failure_after_max_retries`
- **Setup**: `AlwaysFailTool` that always returns `"SSE stream closed unexpectedly"`, `max_retries: 2`
- **Flow**: 4 attempts total (1 initial + 3 retries), all fail
- **Assertion**: Error contains `"SSE stream closed"`, result is `Err`
- **Design target**: Permanent failures surface after retry budget exhausted

---

## Category 3: Malformed Payloads

**Scenario**: A tool subprocess returns garbage, truncated, or structurally invalid responses.

### `malformed_truncated_json_returns_structured_error`
- **Failure mode**: `MalformedMode::TruncatedJson` — simulates broken pipe during JSON parse
- **Error**: `"unexpected EOF while parsing JSON response"`
- **Design target**: Structured error, not a panic

### `malformed_invalid_type_returns_structured_error`
- **Failure mode**: `MalformedMode::InvalidType` — type mismatch in response
- **Error**: `"expected string, got integer"`
- **Design target**: Type errors propagate cleanly

### `malformed_missing_field_returns_structured_error`
- **Failure mode**: `MalformedMode::MissingField` — required field absent
- **Error**: `"missing required field 'result'"`
- **Design target**: Schema violations caught

### `malformed_non_utf8_returns_structured_error`
- **Failure mode**: `MalformedMode::NonUtf8` — binary garbage in response
- **Error**: `"invalid UTF-8 sequence in response"`
- **Design target**: Encoding errors handled

---

## Category 4: Config Hot-Reload Failure Recovery

**Scenario**: A YAML config file is modified at runtime with invalid content.

### `config_hot_reload_malformed_yaml_retains_lkg`
- **Setup**: Register 1 tool from valid YAML → overwrite file with malformed YAML → attempt reload
- **Flow**: `McpConfigFile::from_path()` returns `Err` on malformed YAML
- **Assertion**: Original registry still has 1 tool (`"stable"`) — Last Known Good (LKG) config retained
- **Design target**: Malformed config does not destroy existing tool registry

### `config_hot_reload_valid_update_works`
- **Setup**: Register 1 tool → overwrite with valid 2-tool config → reload
- **Flow**: Registry cleared + re-registered, now has 2 tools
- **Assertion**: Both `tool1` and `tool2` present
- **Design target**: Valid hot-reload adds new tools

---

## Category 5: Retry Backoff & Timeout Verification

**Scenario**: Transient failures with exponential backoff and per-attempt timeout enforcement.

### `retry_backoff_250ms_exponential`
- **Setup**: `SlowFailingTool` fails on attempts 1–2, succeeds on 3. Base backoff: 250ms.
- **Flow**: Attempt 1 fail → sleep 250ms → Attempt 2 fail → sleep 500ms → Attempt 3 succeed
- **Assertion**: Elapsed ≥ 700ms (250+500), attempt count == 3
- **Design target**: Exponential backoff confirmed

### `retry_backoff_max_retries_respected`
- **Setup**: `SlowFailingTool` with `max_attempts: 10` (never succeeds within budget), `max_retries: 3`
- **Flow**: 4 attempts total (1 initial + 3 retries), all fail
- **Assertion**: attempt count == 4, elapsed ≥ 550ms (100+200+300)
- **Design target**: Retry budget enforced

### `timeout_enforced_per_attempt`
- **Setup**: `StdioTimeoutTool` with 200ms delay, 50ms timeout, 1 retry
- **Flow**: Attempt 1 timeout (50ms) → retry → Attempt 2 timeout (50ms) → fail
- **Assertion**: Elapsed ≥ 100ms, elapsed < 200ms
- **Design target**: Timeout enforced per individual attempt, not cumulative

### `cancellation_token_propagates`
- **Setup**: `StdioTimeoutTool` with 10s delay, 50ms timeout, 0 retries
- **Flow**: Spawn task → abort after 100ms → verify task is cancelled
- **Assertion**: `handle.await` returns `Err` (JoinError from abort)
- **Design target**: Tokio cancellation propagates correctly

---

## Category 6: Concurrent Stress

**Scenario**: Race conditions between tool execution and registry mutation.

### `concurrent_tool_execution_no_crosstalk`
- **Setup**: Register 100 tools, spawn 50 concurrent tasks each calling a random tool
- **Flow**: Each task calls `registry.get()` then `tool.call()`, asserts result matches tool ID
- **Design target**: No cross-talk between concurrent tool executions

### `concurrent_registry_modification_during_execution`
- **Setup**: 20 initial tools, 5 writer tasks (register 10 + clear), 30 reader tasks (list + call)
- **Flow**: Writers and readers execute concurrently for ~50 batches
- **Design target**: Registry remains consistent under concurrent read/write stress

---

## Infrastructure Notes

- **Mock tools**: `StdIoDropTool`, `StdioTimeoutTool`, `SseDisconnectTool`, `AlwaysFailTool`, `MalformedPayloadTool`, `SlowFailingTool` — all implement `Tool` trait with deterministic failure modes
- **Counters**: `AtomicUsize` for attempt tracking, `Arc<RwLock<bool>>` for drop flags
- **Config tests**: Use `tempfile::NamedTempFile` for isolated YAML files
- **No external processes**: All tests are in-process mocks — zero network, zero subprocess spawning
- **Runtime**: `tokio::test` with `#[tokio::test]` macro
