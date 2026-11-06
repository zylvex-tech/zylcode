//! Acceptance tests for the provider streaming layer.
//!
//! Every test here talks real HTTP to a loopback server (`providers::test_server`)
//! and drives real decoders. Nothing is stubbed except the provider itself, so a
//! passing test is evidence about the wire protocol, the chunk contract, and
//! cancellation — not about a mock.
//!
//! Deliberately covered:
//! * genuine incremental delivery (several writes, one stream),
//! * cancellation aborting the underlying request (server observes the disconnect),
//! * a request that is cancelled before send never egressing,
//! * non-success status classification,
//! * a provider error arriving after partial output,
//! * a malformed frame,
//! * a truncated stream (closed without a completion signal),
//! * a timeout,
//! * a refused connection.

use std::time::{Duration, Instant};

use serde_json::json;

use super::streaming::{CancelToken, ChatRequest, FinishReason, ProviderErrorKind, StreamChunk};
use super::test_server::{sse_data, sse_json, Scenario, TestServer};
use super::{anthropic, ollama, openai_compatible};

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("build test client")
}

fn anthropic_events() -> Vec<Vec<u8>> {
    vec![
        sse_json(&json!({
            "type": "message_start",
            "message": {"id": "msg_1", "usage": {"input_tokens": 12, "output_tokens": 0}}
        })),
        sse_json(&json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "Hel"}
        })),
        sse_json(&json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "lo"}
        })),
        sse_json(&json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn"},
            "usage": {"output_tokens": 2}
        })),
        sse_json(&json!({"type": "message_stop"})),
    ]
}

#[tokio::test]
async fn anthropic_delivers_genuine_incremental_chunks() {
    let server = TestServer::start(Scenario::Body(anthropic_events()));
    let req = ChatRequest::new("claude-sonnet-4-5", "sys", "say hello");
    let cancel = CancelToken::new();
    let mut chunks: Vec<StreamChunk> = Vec::new();

    let outcome = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_secs(5),
        &cancel,
        &mut |chunk| chunks.push(chunk),
    )
    .await
    .expect("stream succeeds");

    assert_eq!(outcome.text, "Hello");
    assert_eq!(outcome.finish, FinishReason::EndTurn);
    assert_eq!(outcome.provider_ref.as_deref(), Some("msg_1"));
    let usage = outcome.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, Some(12));
    assert_eq!(usage.output_tokens, Some(2));

    // More than one text chunk proves the deltas were forwarded incrementally
    // rather than accumulated and emitted once at the end.
    let text_chunks: Vec<_> = chunks
        .iter()
        .filter_map(|c| c.kind.text.clone())
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(text_chunks, vec!["Hel".to_string(), "lo".to_string()]);
    assert!(chunks.iter().all(|c| !c.synthetic));
    assert_eq!(chunks.iter().filter(|c| c.done).count(), 1);
    assert_eq!(chunks.last().unwrap().index, chunks.len() as u64 - 1);
}

#[tokio::test]
async fn openai_compatible_streams_until_done_sentinel() {
    let events = vec![
        sse_json(&json!({
            "id": "chatcmpl-1",
            "choices": [{"index": 0, "delta": {"content": "He"}, "finish_reason": null}]
        })),
        sse_json(&json!({
            "id": "chatcmpl-1",
            "choices": [{"index": 0, "delta": {"content": "llo"}, "finish_reason": "stop"}]
        })),
        sse_json(&json!({
            "id": "chatcmpl-1",
            "choices": [],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2}
        })),
        sse_data("[DONE]"),
    ];
    let server = TestServer::start(Scenario::Body(events));
    let req = ChatRequest::new("gpt-4o-mini", "", "say hello");
    let mut chunks = Vec::new();

    let target = openai_compatible::CompatTarget {
        base_url: &server.url,
        api_key: Some("sk-test"),
        provider: "openai-compatible",
        timeout: Duration::from_secs(5),
        extra_headers: &[],
    };
    let outcome = openai_compatible::stream(
        &client(),
        &target,
        &req,
        &CancelToken::new(),
        &mut |chunk| chunks.push(chunk),
    )
    .await
    .expect("stream succeeds");

    assert_eq!(outcome.text, "Hello");
    assert_eq!(outcome.finish, FinishReason::Stop);
    assert_eq!(outcome.usage.unwrap().input_tokens, Some(3));
    assert!(chunks.iter().all(|c| !c.synthetic));
}

#[tokio::test]
async fn ollama_streams_ndjson_until_done_flag() {
    let events = vec![
        b"{\"model\":\"llama3.1\",\"message\":{\"content\":\"He\"},\"done\":false}\n".to_vec(),
        b"{\"model\":\"llama3.1\",\"message\":{\"content\":\"llo\"},\"done\":false}\n".to_vec(),
        b"{\"model\":\"llama3.1\",\"message\":{\"content\":\"\"},\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":8,\"eval_count\":2}\n".to_vec(),
    ];
    let server = TestServer::start(Scenario::Body(events));
    let req = ChatRequest::new("llama3.1", "", "say hello");

    let outcome = ollama::stream(
        &client(),
        &server.url,
        &req,
        Duration::from_secs(5),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect("stream succeeds");

    assert_eq!(outcome.text, "Hello");
    assert_eq!(outcome.finish, FinishReason::Stop);
    let usage = outcome.usage.unwrap();
    assert_eq!(usage.input_tokens, Some(8));
    assert_eq!(usage.output_tokens, Some(2));
}

#[tokio::test]
async fn cancelling_mid_stream_aborts_the_underlying_request() {
    // The server sends one real event, then keeps the connection open. Cancelling
    // must close the connection — the server notices, which is the difference
    // between "stopped reading" and "aborted the request".
    let mut pieces = anthropic_events();
    pieces.truncate(2);
    let server = TestServer::start(Scenario::BodyThenHold(pieces));
    let req = ChatRequest::new("claude-sonnet-4-5", "", "say hello");

    let cancel = CancelToken::new();
    let cancel_from_callback = cancel.clone();
    let started = Instant::now();
    let outcome = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_secs(30),
        &cancel,
        &mut |chunk| {
            if chunk.kind.text.as_deref() == Some("Hel") {
                cancel_from_callback.cancel();
            }
        },
    )
    .await
    .expect("cancellation is not an error");

    assert_eq!(outcome.finish, FinishReason::Cancelled);
    assert_eq!(outcome.text, "Hel");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "cancellation must return promptly, took {:?}",
        started.elapsed()
    );

    let disconnected = server
        .disconnected
        .recv_timeout(Duration::from_secs(10))
        .expect("server must observe the client disconnecting");
    assert!(
        disconnected,
        "cancellation must tear down the connection, not just stop forwarding chunks"
    );
}

#[tokio::test]
async fn a_pre_cancelled_request_never_egresses() {
    // Port 1 is unusable, so any real attempt would fail loudly. A cancelled
    // request must return Cancelled instead, proving no connection was opened.
    let cancel = CancelToken::new();
    cancel.cancel();
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let outcome = anthropic::stream(
        &client(),
        "http://127.0.0.1:1",
        Some("sk-test"),
        &req,
        Duration::from_secs(2),
        &cancel,
        &mut |_| {},
    )
    .await
    .expect("cancelled request is not an error");

    assert_eq!(outcome.finish, FinishReason::Cancelled);
    assert!(outcome.text.is_empty());
}

#[tokio::test]
async fn non_success_status_is_classified_not_retried_blindly() {
    let server = TestServer::start_error("401 Unauthorized", "{\"error\":\"bad key\"}");
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let err = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-wrong"),
        &req,
        Duration::from_secs(5),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect_err("401 must fail");

    assert_eq!(err.kind, ProviderErrorKind::Authentication);
    assert_eq!(err.code.as_deref(), Some("401"));
}

#[tokio::test]
async fn a_provider_error_after_partial_output_preserves_the_partial_text() {
    let events = vec![
        sse_json(&json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "partial"}
        })),
        sse_json(&json!({
            "type": "error",
            "error": {"type": "overloaded_error", "message": "Overloaded"}
        })),
    ];
    let server = TestServer::start(Scenario::Body(events));
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");
    let mut chunks = Vec::new();

    let err = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_secs(5),
        &CancelToken::new(),
        &mut |chunk| chunks.push(chunk),
    )
    .await
    .expect_err("error event must fail the stream");

    assert_eq!(err.kind, ProviderErrorKind::ProviderRejected);
    assert!(err.message.contains("Overloaded"));
    // The partial output was forwarded before the error, and the terminal chunk
    // carries the error instead of pretending the stream completed.
    assert!(chunks
        .iter()
        .any(|c| c.kind.text.as_deref() == Some("partial")));
    let terminal = chunks.last().unwrap();
    assert!(terminal.done);
    assert!(terminal.error.is_some());
    assert!(!terminal.synthetic);
}

#[tokio::test]
async fn a_malformed_frame_is_reported_as_malformed() {
    let server = TestServer::start(Scenario::Body(vec![sse_data("{not json")]));
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let err = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_secs(5),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect_err("malformed frame must fail");

    assert_eq!(err.kind, ProviderErrorKind::Malformed);
}

#[tokio::test]
async fn a_stream_closed_without_completion_signal_is_truncated() {
    // One text delta, then the socket closes with no message_stop.
    let mut events = anthropic_events();
    events.truncate(2);
    let server = TestServer::start(Scenario::Body(events));
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let err = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_secs(5),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect_err("truncated stream must fail");

    assert_eq!(err.kind, ProviderErrorKind::Truncated);
}

#[tokio::test]
async fn a_stalled_stream_hits_the_request_timeout() {
    // Headers arrive, then only keep-alive comments: the body never completes.
    let server = TestServer::start(Scenario::BodyThenHold(Vec::new()));
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let err = anthropic::stream(
        &client(),
        &server.url,
        Some("sk-test"),
        &req,
        Duration::from_millis(300),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect_err("timeout must fail");

    assert_eq!(err.kind, ProviderErrorKind::Transport);
}

#[tokio::test]
async fn a_refused_connection_is_a_transport_error() {
    // Bind and drop to obtain a port nothing is listening on.
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").port()
    };
    let req = ChatRequest::new("claude-sonnet-4-5", "", "hi");

    let err = anthropic::stream(
        &client(),
        &format!("http://127.0.0.1:{port}"),
        Some("sk-test"),
        &req,
        Duration::from_secs(2),
        &CancelToken::new(),
        &mut |_| {},
    )
    .await
    .expect_err("refused connection must fail");

    assert_eq!(err.kind, ProviderErrorKind::Transport);
}

#[tokio::test]
async fn synthetic_streams_are_labelled_and_never_egress() {
    let mut chunks = Vec::new();
    let outcome = super::synthetic::stream("deploy", "sys", &mut |chunk| chunks.push(chunk));
    assert_eq!(outcome.finish, FinishReason::Stop);
    assert!(chunks.iter().all(|c| c.synthetic));
    assert!(outcome.usage.is_none());
    assert!(outcome.text.contains("Synthetic offline response"));
}
