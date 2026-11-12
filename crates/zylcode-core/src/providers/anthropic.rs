//! Anthropic Messages API protocol: request construction, SSE decoding, streaming.
//!
//! Wire facts this module depends on (Messages API, `anthropic-version: 2023-06-01`):
//!
//! * `POST /v1/messages` with `stream: true` answers with SSE.
//! * `content_block_delta` carries `delta.type = "text_delta"` and `delta.text`.
//! * `message_start` carries `message.id` and `message.usage.input_tokens`.
//! * `message_delta` carries `delta.stop_reason` and `usage.output_tokens`.
//! * `message_stop` ends the stream; `ping` events are keep-alives.
//! * An `error` event carries `error.type` and `error.message`.
//!
//! Native tool calling **is** sent: `tools` carries `{name, description,
//! input_schema}` and `tool_use` blocks are translated into the neutral tool-call
//! contract. Execution remains ZylCode's gated path; this module only translates
//! the protocol.

use std::time::Duration;

use super::sse::SseDecoder;
use super::streaming::{
    CancelToken, ChatMessage, ChatRequest, Completion, DecodedEvent, FinishReason, LineDecoder,
    ProviderError, ProviderErrorKind, StreamEndpoint, StreamOutcome, ToolCall, ToolCallDelta,
    ToolCallDeltaKind, Usage,
};
use std::collections::HashMap;

/// Anthropic API version this module is written against.
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Build the streaming request body for the Messages API.
pub fn messages_body(req: &ChatRequest, stream: bool) -> serde_json::Value {
    // Anthropic keys tool results by block id and requires them to arrive in a
    // *user* turn, so an assistant turn's `tool_use` blocks and the following
    // `tool_result` blocks are two deliberate shapes rather than one message list.
    let mut messages: Vec<serde_json::Value> = Vec::new();
    for turn in req.turns() {
        match turn {
            ChatMessage::User { content } => messages.push(serde_json::json!({
                "role": "user",
                "content": [{ "type": "text", "text": content }],
            })),
            ChatMessage::Assistant {
                content,
                tool_calls,
            } => {
                let mut blocks: Vec<serde_json::Value> = Vec::new();
                if !content.is_empty() {
                    blocks.push(serde_json::json!({ "type": "text", "text": content }));
                }
                for call in &tool_calls {
                    // The block id is what a `tool_result` addresses; without it
                    // the result cannot be attached to the call that asked for it.
                    let Some(id) = call.call_id.clone() else {
                        continue;
                    };
                    let input: serde_json::Value =
                        serde_json::from_str(&call.arguments).unwrap_or(serde_json::json!({}));
                    blocks.push(serde_json::json!({
                        "type": "tool_use",
                        "id": id,
                        "name": crate::providers::streaming::wire_name(&call.name),
                        "input": input,
                    }));
                }
                messages.push(serde_json::json!({ "role": "assistant", "content": blocks }));
            }
            ChatMessage::ToolResult {
                call_id,
                content,
                is_error,
                ..
            } => messages.push(serde_json::json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": call_id,
                    "content": content,
                    "is_error": is_error,
                }],
            })),
        }
    }
    if messages.is_empty() {
        messages.push(serde_json::json!({
            "role": "user",
            "content": [{ "type": "text", "text": req.prompt }],
        }));
    }

    let mut body = serde_json::json!({
        "model": req.model,
        "max_tokens": req.max_tokens,
        "messages": messages,
        "stream": stream,
    });
    if !req.system.is_empty() {
        body["system"] = serde_json::Value::String(req.system.clone());
    }
    // `temperature` is only meaningful to send when the caller tuned it.
    if (req.temperature - 0.2).abs() > f32::EPSILON {
        body["temperature"] = serde_json::json!(req.temperature);
    }
    if !req.tools.is_empty() {
        body["tools"] = serde_json::Value::Array(
            req.tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "name": tool.wire_name(),
                        "description": tool.description,
                        "input_schema": tool.parameters,
                    })
                })
                .collect(),
        );
        // Anthropic names the must-call case `any`, not `required`.
        body["tool_choice"] = serde_json::json!({
            "type": match req.tool_choice {
                super::streaming::ToolChoice::Auto => "auto",
                super::streaming::ToolChoice::None => "none",
                super::streaming::ToolChoice::Required => "any",
            }
        });
    }
    body
}

/// Extract the answer and usage from a non-streamed Messages API response.
///
/// `content` is an array of blocks. Only blocks the provider types as `text` (or
/// leaves untyped) contribute assistant text; a `tool_use` block is not text and
/// must never be concatenated into the answer.
pub fn parse_completion(json: &serde_json::Value) -> Completion {
    let text = json
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| {
                    block
                        .get("type")
                        .and_then(|t| t.as_str())
                        .is_none_or(|t| t == "text")
                })
                .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                .collect::<String>()
        })
        .unwrap_or_default();

    // `tool_use` blocks are requests to call a tool, not text. Their `input`
    // is already a JSON object in this API, so it is serialised to keep the
    // neutral contract's single string shape.
    let tool_calls = json
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                .filter_map(|block| {
                    let name = block.get("name")?.as_str()?.to_string();
                    Some(ToolCall {
                        call_id: None,
                        block_id: block.get("id").and_then(|v| v.as_str()).map(str::to_string),
                        name,
                        arguments: block
                            .get("input")
                            .map(|input| input.to_string())
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
            input_tokens: json
                .get("usage")
                .and_then(|u| u.get("input_tokens"))
                .and_then(|v| v.as_u64()),
            output_tokens: json
                .get("usage")
                .and_then(|u| u.get("output_tokens"))
                .and_then(|v| v.as_u64()),
            extra: None,
        },
    }
}

/// Headers the Messages API requires beyond `content-type`.
pub fn headers(api_key: Option<&str>) -> Vec<(String, String)> {
    let mut headers = vec![(
        "anthropic-version".to_string(),
        ANTHROPIC_VERSION.to_string(),
    )];
    if let Some(key) = api_key {
        headers.push(("x-api-key".to_string(), key.to_string()));
    }
    headers
}

/// Streaming URL for a base URL such as `https://api.anthropic.com`.
pub fn messages_url(base: &str) -> String {
    format!("{}/v1/messages", base.trim_end_matches('/'))
}

/// Decoder for the Messages API event stream.
#[derive(Debug, Default)]
pub struct AnthropicDecoder {
    sse: SseDecoder,
    provider_ref: Option<String>,
    finish: Option<FinishReason>,
    usage: Usage,
    /// Argument deltas and `content_block_stop` carry only the block `index`,
    /// so the block's id and tool name are remembered when it starts.
    tool_blocks: HashMap<u64, (String, String)>,
}

impl AnthropicDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn decode_event(
        &mut self,
        name: Option<&str>,
        data: &str,
    ) -> Result<Vec<DecodedEvent>, ProviderError> {
        if data.trim().is_empty() {
            return Ok(Vec::new());
        }
        let json: serde_json::Value = serde_json::from_str(data).map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Malformed,
                format!("anthropic stream sent a frame that is not JSON: {e}"),
            )
        })?;

        // The payload's own `type` is authoritative; the SSE `event:` name is a
        // duplicate of it, and older gateways omit the latter.
        let kind = json
            .get("type")
            .and_then(|t| t.as_str())
            .or(name)
            .unwrap_or_default();

        match kind {
            "message_start" => {
                if let Some(id) = json
                    .get("message")
                    .and_then(|m| m.get("id"))
                    .and_then(|v| v.as_str())
                {
                    self.provider_ref = Some(id.to_string());
                }
                if let Some(usage) = json.get("message").and_then(|m| m.get("usage")) {
                    self.usage.input_tokens = usage.get("input_tokens").and_then(|v| v.as_u64());
                    self.usage.output_tokens = usage.get("output_tokens").and_then(|v| v.as_u64());
                }
                Ok(vec![DecodedEvent {
                    provider_ref: self.provider_ref.clone(),
                    usage: Some(self.usage.clone()),
                    ..DecodedEvent::default()
                }])
            }
            "content_block_delta" => {
                let delta = json.get("delta").cloned().unwrap_or_default();
                let delta_type = delta
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                if delta_type == "text_delta" {
                    let text = delta
                        .get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default();
                    return Ok(vec![DecodedEvent::text(text)]);
                }
                if delta_type == "input_json_delta" {
                    let index = json
                        .get("index")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let block_id = self.tool_blocks.get(&index).map(|(id, _)| id.clone());
                    let partial = delta
                        .get("partial_json")
                        .and_then(|p| p.as_str())
                        .unwrap_or_default();
                    return Ok(vec![DecodedEvent {
                        tool_call: Some(ToolCallDelta {
                            kind: ToolCallDeltaKind::ArgumentDelta,
                            call_id: None,
                            block_id,
                            argument_delta: Some(partial.to_string()),
                            done: false,
                            metadata: None,
                        }),
                        provider_ref: self.provider_ref.clone(),
                        ..DecodedEvent::default()
                    }]);
                }
                // A delta type this module does not implement is preserved as
                // metadata rather than silently dropped.
                Ok(vec![DecodedEvent {
                    metadata: Some(serde_json::json!({
                        "unhandled_delta_type": delta_type,
                        "delta": delta,
                    })),
                    ..DecodedEvent::default()
                }])
            }
            "content_block_start" => {
                let index = json
                    .get("index")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let block = json.get("content_block").cloned().unwrap_or_default();
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    let id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    self.tool_blocks.insert(index, (id.clone(), name.clone()));
                    return Ok(vec![DecodedEvent {
                        tool_call: Some(ToolCallDelta {
                            kind: ToolCallDeltaKind::Start,
                            call_id: None,
                            block_id: Some(id),
                            argument_delta: None,
                            done: false,
                            metadata: Some(serde_json::json!({ "name": name })),
                        }),
                        provider_ref: self.provider_ref.clone(),
                        ..DecodedEvent::default()
                    }]);
                }
                // A text block needs no event of its own: its deltas follow.
                Ok(Vec::new())
            }
            "content_block_stop" => {
                let index = json
                    .get("index")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                match self.tool_blocks.remove(&index) {
                    Some((id, _name)) => Ok(vec![DecodedEvent {
                        tool_call: Some(ToolCallDelta {
                            kind: ToolCallDeltaKind::Done,
                            call_id: None,
                            block_id: Some(id),
                            argument_delta: None,
                            done: true,
                            metadata: None,
                        }),
                        provider_ref: self.provider_ref.clone(),
                        ..DecodedEvent::default()
                    }]),
                    None => Ok(Vec::new()),
                }
            }
            "message_delta" => {
                if let Some(reason) = json
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(|r| r.as_str())
                {
                    self.finish = Some(map_stop_reason(reason));
                }
                if let Some(usage) = json.get("usage") {
                    if let Some(out) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                        self.usage.output_tokens = Some(out);
                    }
                    if let Some(inp) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                        self.usage.input_tokens = Some(inp);
                    }
                }
                Ok(vec![DecodedEvent {
                    finish: self.finish,
                    usage: Some(self.usage.clone()),
                    ..DecodedEvent::default()
                }])
            }
            "message_stop" => {
                let mut event = DecodedEvent::done();
                event.finish = self.finish.or(Some(FinishReason::Stop));
                event.usage = Some(self.usage.clone());
                Ok(vec![event])
            }
            "ping" => Ok(Vec::new()),
            "error" => {
                let err = json.get("error").cloned().unwrap_or_default();
                let message = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("anthropic returned an error event");
                let mut provider_error = ProviderError::new(
                    classify_error_type(err.get("type").and_then(|t| t.as_str())),
                    message.to_string(),
                );
                provider_error.code = err
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t.to_string());
                provider_error.raw = Some(err);
                Ok(vec![DecodedEvent::error(provider_error)])
            }
            other => Ok(vec![DecodedEvent {
                metadata: Some(serde_json::json!({ "unhandled_event_type": other })),
                ..DecodedEvent::default()
            }]),
        }
    }
}

impl LineDecoder for AnthropicDecoder {
    fn feed(&mut self, line: &str) -> Result<Vec<DecodedEvent>, ProviderError> {
        let Some(event) = self.sse.push_line(line) else {
            return Ok(Vec::new());
        };
        self.decode_event(event.event.as_deref(), &event.data)
    }

    fn finish(&mut self) -> Vec<DecodedEvent> {
        match self.sse.finish() {
            Some(event) => self
                .decode_event(event.event.as_deref(), &event.data)
                .unwrap_or_else(|err| vec![DecodedEvent::error(err)]),
            None => Vec::new(),
        }
    }
}

fn map_stop_reason(reason: &str) -> FinishReason {
    match reason {
        "end_turn" | "stop_sequence" => FinishReason::EndTurn,
        "max_tokens" => FinishReason::Length,
        "refusal" => FinishReason::ContentFiltered,
        "tool_use" => FinishReason::Stop,
        _ => FinishReason::Other,
    }
}

fn classify_error_type(error_type: Option<&str>) -> ProviderErrorKind {
    match error_type {
        Some("authentication_error") | Some("permission_error") => {
            ProviderErrorKind::Authentication
        }
        Some("not_found_error") => ProviderErrorKind::ModelUnavailable,
        Some("invalid_request_error") => ProviderErrorKind::ProviderRejected,
        Some("rate_limit_error") | Some("overloaded_error") | Some("api_error") => {
            ProviderErrorKind::ProviderRejected
        }
        _ => ProviderErrorKind::Unknown,
    }
}

/// Stream one Messages API call, forwarding neutral chunks and honouring `cancel`.
pub async fn stream(
    client: &reqwest::Client,
    base_url: &str,
    api_key: Option<&str>,
    req: &ChatRequest,
    timeout: Duration,
    cancel: &CancelToken,
    on_chunk: &mut (dyn FnMut(super::streaming::StreamChunk) + Send),
) -> Result<StreamOutcome, ProviderError> {
    let mut endpoint = StreamEndpoint::new("anthropic", messages_url(base_url), timeout);
    for (name, value) in headers(api_key) {
        endpoint = endpoint.with_header(name, value);
    }
    let body = messages_body(req, true);
    let mut decoder = AnthropicDecoder::new();
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
        let mut decoder = AnthropicDecoder::new();
        let mut out = Vec::new();
        for line in lines {
            out.extend(decoder.feed(line).expect("valid frame"));
        }
        out.extend(decoder.finish());
        out
    }

    #[test]
    fn parse_completion_joins_text_blocks_and_ignores_tool_use() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "Hello" },
                { "type": "tool_use", "id": "t1", "name": "read", "input": {} },
                { "type": "text", "text": " world" }
            ],
            "usage": { "input_tokens": 11, "output_tokens": 4 },
            "stop_reason": "end_turn"
        });

        let completion = parse_completion(&json);
        assert_eq!(completion.text, "Hello world");
        assert_eq!(completion.usage.input_tokens, Some(11));
        assert_eq!(completion.usage.output_tokens, Some(4));
    }

    #[test]
    fn parse_completion_reports_missing_usage_as_missing_not_zero() {
        let json = serde_json::json!({ "content": [{ "type": "text", "text": "hi" }] });
        let completion = parse_completion(&json);
        assert_eq!(completion.text, "hi");
        assert_eq!(completion.usage.input_tokens, None);
        assert_eq!(completion.usage.output_tokens, None);
    }

    #[test]
    fn request_body_uses_messages_shape() {
        let req = ChatRequest::new("claude-sonnet-4-5", "be terse", "hi");
        let body = messages_body(&req, true);
        assert_eq!(body["model"], "claude-sonnet-4-5");
        assert_eq!(body["stream"], true);
        assert_eq!(body["system"], "be terse");
        assert_eq!(body["messages"][0]["role"], "user");
        // Content is a block array: a plain string cannot also carry a
        // `tool_result` block in the same turn, and tool results must.
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][0]["content"][0]["text"], "hi");
        assert_eq!(body["max_tokens"], 4096);
        // Native tool calling is gated: no tools field may be sent.
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn empty_system_is_omitted() {
        let req = ChatRequest::new("m", "", "hi");
        assert!(messages_body(&req, false).get("system").is_none());
    }

    #[test]
    fn auth_header_and_version_are_present() {
        let with_key = headers(Some("sk-test"));
        assert!(with_key
            .iter()
            .any(|(k, v)| k == "x-api-key" && v == "sk-test"));
        assert!(with_key
            .iter()
            .any(|(k, v)| k == "anthropic-version" && v == ANTHROPIC_VERSION));
        let without_key = headers(None);
        assert!(without_key.iter().all(|(k, _)| k != "x-api-key"));
    }

    #[test]
    fn url_joins_without_double_slashes() {
        assert_eq!(
            messages_url("https://api.anthropic.com/"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn text_deltas_accumulate_in_order() {
        let events = decode(&[
            "event: content_block_delta",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel\"}}",
            "",
            "event: content_block_delta",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}",
            "",
        ]);
        let text: String = events.iter().filter_map(|e| e.text.clone()).collect();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn usage_from_start_and_delta_are_both_reported() {
        let events = decode(&[
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"usage\":{\"input_tokens\":11,\"output_tokens\":0}}}",
            "",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}",
            "",
            "data: {\"type\":\"message_stop\"}",
            "",
        ]);
        assert_eq!(events[0].provider_ref.as_deref(), Some("msg_1"));
        assert_eq!(events[0].usage.as_ref().unwrap().input_tokens, Some(11));
        let last = events.last().unwrap();
        assert!(last.done);
        assert_eq!(last.finish, Some(FinishReason::EndTurn));
        let usage = last.usage.as_ref().unwrap();
        assert_eq!(usage.input_tokens, Some(11));
        assert_eq!(usage.output_tokens, Some(7));
    }

    #[test]
    fn ping_events_produce_nothing() {
        assert!(decode(&["data: {\"type\":\"ping\"}", ""]).is_empty());
    }

    #[test]
    fn error_event_maps_to_authentication() {
        let events = decode(&[
            "data: {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid x-api-key\"}}",
            "",
        ]);
        let event = events.last().unwrap();
        let err = event.error.as_ref().expect("error event");
        assert_eq!(err.kind, ProviderErrorKind::Authentication);
        assert!(err.message.contains("invalid x-api-key"));
        assert_eq!(err.code.as_deref(), Some("authentication_error"));
        assert_eq!(event.finish, Some(FinishReason::Error));
        assert!(event.done);
    }

    #[test]
    fn malformed_json_is_reported_as_malformed_not_ignored() {
        let mut decoder = AnthropicDecoder::new();
        assert!(decoder.feed("data: {not json").unwrap().is_empty());
        // SSE only dispatches an event when its block ends.
        let err = decoder.feed("").expect_err("must not silently drop");
        assert_eq!(err.kind, ProviderErrorKind::Malformed);
    }

    #[test]
    fn stop_reason_mapping_is_explicit() {
        assert_eq!(map_stop_reason("end_turn"), FinishReason::EndTurn);
        assert_eq!(map_stop_reason("max_tokens"), FinishReason::Length);
        assert_eq!(map_stop_reason("refusal"), FinishReason::ContentFiltered);
        assert_eq!(map_stop_reason("something_new"), FinishReason::Other);
    }

    #[test]
    fn tool_use_delta_is_translated_into_a_neutral_tool_call() {
        // A tool block must start before its arguments arrive; the decoder
        // remembers the block id so the fragments can be attributed.
        let start = serde_json::json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": { "type": "tool_use", "id": "toolu_9", "name": "x" }
        })
        .to_string();
        let frame = serde_json::json!({
            "type": "content_block_delta",
            "index": 1,
            "delta": { "type": "input_json_delta", "partial_json": "{\"a\":1" }
        })
        .to_string();
        let mut decoder = AnthropicDecoder::new();
        let mut events = decoder
            .feed(&format!("data: {start}"))
            .expect("valid frame");
        events.extend(decoder.feed("").expect("valid frame"));
        let mut delta_events = decoder
            .feed(&format!("data: {frame}"))
            .expect("valid frame");
        delta_events.extend(decoder.feed("").expect("valid frame"));

        let start_delta = events[0].tool_call.as_ref().expect("a start delta");
        assert_eq!(start_delta.kind, ToolCallDeltaKind::Start);
        let argument = delta_events[0]
            .tool_call
            .as_ref()
            .expect("an argument delta");
        assert_eq!(argument.kind, ToolCallDeltaKind::ArgumentDelta);
        assert_eq!(argument.block_id.as_deref(), Some("toolu_9"));
        assert_eq!(argument.argument_delta.as_deref(), Some("{\"a\":1"));
    }
}
