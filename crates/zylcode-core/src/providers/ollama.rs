//! Ollama chat protocol: request construction, NDJSON decoding, streaming.
//!
//! Wire facts this module depends on (`POST /api/chat`):
//!
//! * Ollama streams newline-delimited JSON, not SSE: one complete JSON object per
//!   line, no `data:` prefix and no event framing.
//! * `message.content` carries incremental text on each object.
//! * `done: true` marks the final object, which also carries `done_reason` and the
//!   counters `prompt_eval_count` / `eval_count`.
//! * An error is returned as `{"error": "..."}` with a non-2xx status, or inline.
//!
//! Native tool calling **is** sent: `tools` uses the OpenAI-shaped
//! `{type, function}` objects Ollama documents, and `message.tool_calls` is
//! translated into the neutral tool-call contract. Execution remains ZylCode's
//! gated path.

use std::time::Duration;

use super::streaming::{
    CancelToken, ChatMessage, ChatRequest, Completion, DecodedEvent, FinishReason, LineDecoder,
    ProviderError, ProviderErrorKind, StreamChunk, StreamEndpoint, StreamOutcome, ToolCall,
    ToolCallDelta, ToolCallDeltaKind, Usage,
};

/// Streaming URL for an Ollama base URL such as `http://127.0.0.1:11434`.
pub fn chat_url(base: &str) -> String {
    format!("{}/api/chat", base.trim_end_matches('/'))
}

/// Build the streaming request body for `/api/chat`.
pub fn chat_body(req: &ChatRequest, stream: bool) -> serde_json::Value {
    let mut messages = Vec::new();
    if !req.system.is_empty() {
        messages.push(serde_json::json!({ "role": "system", "content": req.system }));
    }
    for turn in req.turns() {
        messages.push(match turn {
            ChatMessage::User { content } => serde_json::json!({
                "role": "user",
                "content": content,
            }),
            // Ollama takes the arguments as an object and addresses results by
            // position, not by id, so an unparsable argument string cannot be
            // carried faithfully — it is sent as an empty object and the caller
            // has already been told the call was malformed.
            ChatMessage::Assistant {
                content,
                tool_calls,
            } => serde_json::json!({
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls
                    .iter()
                    .map(|call| serde_json::json!({
                        "function": {
                            "name": crate::providers::streaming::wire_name(&call.name),
                            "arguments": serde_json::from_str::<serde_json::Value>(&call.arguments)
                                .unwrap_or_else(|_| serde_json::json!({})),
                        },
                    }))
                    .collect::<Vec<_>>(),
            }),
            ChatMessage::ToolResult { content, .. } => serde_json::json!({
                "role": "tool",
                "content": content,
            }),
        });
    }

    let mut body = serde_json::json!({
        "model": req.model,
        "messages": messages,
        "stream": stream,
    });
    body["options"] = serde_json::json!({
        "temperature": req.temperature,
        "num_predict": req.max_tokens,
    });
    if !req.tools.is_empty() {
        body["tools"] = serde_json::Value::Array(
            req.tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": tool.wire_name(),
                            "description": tool.description,
                            "parameters": tool.parameters,
                        }
                    })
                })
                .collect(),
        );
    }
    body
}

/// Extract the answer and usage from a non-streamed `/api/chat` response.
///
/// Ollama reports token counts as `prompt_eval_count` / `eval_count` on the message
/// object, not as a `usage` block.
pub fn parse_completion(json: &serde_json::Value) -> Completion {
    let text = json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    // Ollama returns arguments as a JSON *object*, unlike OpenAI's string. The
    // neutral contract carries text, so it is serialised here and the boundary
    // keeps one shape for every provider.
    let tool_calls = json
        .get("message")
        .and_then(|m| m.get("tool_calls"))
        .and_then(|calls| calls.as_array())
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    let name = function.get("name")?.as_str()?.to_string();
                    Some(ToolCall {
                        call_id: call.get("id").and_then(|v| v.as_str()).map(str::to_string),
                        block_id: None,
                        name,
                        arguments: function
                            .get("arguments")
                            .map(|a| a.to_string())
                            .unwrap_or_else(|| "{}".to_string()),
                        done: true,
                        metadata: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Completion {
        text,
        tool_calls,
        usage: Usage {
            input_tokens: json.get("prompt_eval_count").and_then(|v| v.as_u64()),
            output_tokens: json.get("eval_count").and_then(|v| v.as_u64()),
            extra: None,
        },
    }
}

/// Ollama has no credentials; a locally bound daemon needs no auth header.
pub fn headers(_api_key: Option<&str>) -> Vec<(String, String)> {
    Vec::new()
}

/// Decoder for Ollama's NDJSON chat stream.
#[derive(Debug, Default)]
pub struct OllamaDecoder {
    done: bool,
    finish: Option<FinishReason>,
    usage: Usage,
    model_ref: Option<String>,
}

impl OllamaDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn decode_line(&mut self, line: &str) -> Result<Vec<DecodedEvent>, ProviderError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let json: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Malformed,
                format!("ollama stream sent a line that is not JSON: {e}"),
            )
        })?;

        if let Some(error) = json.get("error").and_then(|e| e.as_str()) {
            let mut provider_error =
                ProviderError::new(ProviderErrorKind::ProviderRejected, error.to_string());
            provider_error.raw = Some(json.clone());
            return Ok(vec![DecodedEvent::error(provider_error)]);
        }

        if let Some(model) = json.get("model").and_then(|m| m.as_str()) {
            self.model_ref = Some(model.to_string());
        }

        let mut events = Vec::new();

        if let Some(content) = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            if !content.is_empty() {
                events.push(DecodedEvent::text(content));
            }
        }

        // Ollama emits a tool call whole rather than in deltas, so each call
        // becomes a start, its full argument text, and a completion: the same
        // three facts an SSE provider spreads over several frames.
        if let Some(tool_calls) = json
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .and_then(|t| t.as_array())
        {
            for (index, call) in tool_calls.iter().enumerate() {
                let Some(function) = call.get("function") else {
                    continue;
                };
                let Some(name) = function.get("name").and_then(|n| n.as_str()) else {
                    continue;
                };
                let key = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("call-{index}"));
                events.push(DecodedEvent {
                    tool_call: Some(ToolCallDelta {
                        kind: ToolCallDeltaKind::Start,
                        call_id: Some(key.clone()),
                        block_id: None,
                        argument_delta: None,
                        done: false,
                        metadata: Some(serde_json::json!({ "name": name })),
                    }),
                    ..DecodedEvent::default()
                });
                if let Some(arguments) = function.get("arguments") {
                    events.push(DecodedEvent {
                        tool_call: Some(ToolCallDelta {
                            kind: ToolCallDeltaKind::ArgumentDelta,
                            call_id: Some(key.clone()),
                            block_id: None,
                            argument_delta: Some(arguments.to_string()),
                            done: false,
                            metadata: None,
                        }),
                        ..DecodedEvent::default()
                    });
                }
                events.push(DecodedEvent {
                    tool_call: Some(ToolCallDelta {
                        kind: ToolCallDeltaKind::Done,
                        call_id: Some(key),
                        block_id: None,
                        argument_delta: None,
                        done: true,
                        metadata: None,
                    }),
                    ..DecodedEvent::default()
                });
            }
        }

        let is_done = json.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
        if is_done {
            self.usage = Usage::new(
                json.get("prompt_eval_count").and_then(|v| v.as_u64()),
                json.get("eval_count").and_then(|v| v.as_u64()),
            );
            self.finish = Some(map_done_reason(
                json.get("done_reason").and_then(|r| r.as_str()),
            ));
            self.done = true;
            let mut event = DecodedEvent::done();
            event.finish = self.finish;
            event.usage = Some(self.usage.clone());
            event.provider_ref = self.model_ref.clone();
            events.push(event);
        }

        Ok(events)
    }
}

impl LineDecoder for OllamaDecoder {
    fn feed(&mut self, line: &str) -> Result<Vec<DecodedEvent>, ProviderError> {
        self.decode_line(line)
    }

    fn finish(&mut self) -> Vec<DecodedEvent> {
        Vec::new()
    }
}

fn map_done_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        Some("stop") | None => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("content_filter") => FinishReason::ContentFiltered,
        Some(_) => FinishReason::Other,
    }
}

/// Stream one `/api/chat` call, forwarding neutral chunks and honouring `cancel`.
pub async fn stream(
    client: &reqwest::Client,
    base_url: &str,
    req: &ChatRequest,
    timeout: Duration,
    cancel: &CancelToken,
    on_chunk: &mut (dyn FnMut(StreamChunk) + Send),
) -> Result<StreamOutcome, ProviderError> {
    let endpoint = StreamEndpoint::new("ollama", chat_url(base_url), timeout);
    let body = chat_body(req, true);
    let mut decoder = OllamaDecoder::new();
    super::streaming::run_stream(
        client,
        &endpoint,
        &body,
        cancel,
        &mut decoder,
        false,
        on_chunk,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(lines: &[&str]) -> Vec<DecodedEvent> {
        let mut decoder = OllamaDecoder::new();
        let mut out = Vec::new();
        for line in lines {
            out.extend(decoder.feed(line).expect("valid line"));
        }
        out.extend(decoder.finish());
        out
    }

    #[test]
    fn parse_completion_reads_message_content_and_eval_counters() {
        let json = serde_json::json!({
            "message": { "role": "assistant", "content": "pong" },
            "prompt_eval_count": 3,
            "eval_count": 1,
            "done": true
        });

        let completion = parse_completion(&json);
        assert_eq!(completion.text, "pong");
        assert_eq!(completion.usage.input_tokens, Some(3));
        assert_eq!(completion.usage.output_tokens, Some(1));
    }

    #[test]
    fn request_body_is_ndjson_chat_shaped() {
        let req = ChatRequest::new("llama3.1", "be terse", "hi");
        let body = chat_body(&req, true);
        assert_eq!(body["model"], "llama3.1");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "hi");
        assert_eq!(body["options"]["num_predict"], 4096);
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn url_joins_without_double_slashes() {
        assert_eq!(
            chat_url("http://127.0.0.1:11434/"),
            "http://127.0.0.1:11434/api/chat"
        );
    }

    #[test]
    fn text_deltas_accumulate_until_done() {
        let events = decode(&[
            "{\"model\":\"llama3.1\",\"message\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"done\":false}",
            "{\"model\":\"llama3.1\",\"message\":{\"role\":\"assistant\",\"content\":\"lo\"},\"done\":false}",
            "{\"model\":\"llama3.1\",\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":12,\"eval_count\":5}",
        ]);
        let text: String = events.iter().filter_map(|e| e.text.clone()).collect();
        assert_eq!(text, "Hello");
        let last = events.last().unwrap();
        assert!(last.done);
        assert_eq!(last.finish, Some(FinishReason::Stop));
        let usage = last.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, Some(12));
        assert_eq!(usage.output_tokens, Some(5));
    }

    #[test]
    fn blank_lines_are_ignored() {
        assert!(decode(&["", "   ", ""]).is_empty());
    }

    #[test]
    fn inline_error_line_becomes_a_provider_error() {
        let events = decode(&["{\"error\":\"model 'nope' not found\"}"]);
        let err = events.last().unwrap().error.as_ref().expect("error");
        assert_eq!(err.kind, ProviderErrorKind::ProviderRejected);
        assert!(err.message.contains("not found"));
    }

    #[test]
    fn malformed_line_is_reported() {
        let mut decoder = OllamaDecoder::new();
        let err = decoder.feed("{oops").expect_err("must not silently drop");
        assert_eq!(err.kind, ProviderErrorKind::Malformed);
    }

    #[test]
    fn stream_without_done_is_not_marked_complete() {
        let events = decode(&["{\"message\":{\"content\":\"partial\"},\"done\":false}"]);
        assert!(events.iter().all(|e| !e.done));
    }

    #[test]
    fn tool_calls_are_translated_into_the_neutral_contract() {
        let events = decode(&[
            "{\"message\":{\"content\":\"\",\"tool_calls\":[{\"function\":{\"name\":\"x\"}}]},\"done\":false}",
        ]);
        assert!(events.iter().all(|e| e.text.is_none()));
        let kinds: Vec<ToolCallDeltaKind> = events
            .iter()
            .filter_map(|e| e.tool_call.as_ref().map(|d| d.kind))
            .collect();
        assert_eq!(
            kinds,
            vec![ToolCallDeltaKind::Start, ToolCallDeltaKind::Done],
            "a call with no arguments still starts and completes"
        );
    }
}
