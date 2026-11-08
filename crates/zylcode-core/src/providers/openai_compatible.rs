//! OpenAI-compatible chat completions: request construction, SSE decoding, streaming.
//!
//! Wire facts this module depends on (`POST /chat/completions`, `stream: true`):
//!
//! * SSE frames whose `data:` payload is a `chat.completion.chunk`.
//! * `choices[0].delta.content` carries incremental text; `delta.role` is present
//!   on the first chunk of some implementations and carries no text.
//! * `choices[0].finish_reason` is `null` until the last choice chunk.
//! * `usage` is only present when `stream_options: {"include_usage": true}` was
//!   requested, and arrives in a final chunk with an empty `choices` array.
//! * The literal `data: [DONE]` sentinel ends the stream.
//!
//! Native tool calling **is** sent: `tools` carries OpenAI's
//! `{type: function, function}` shape and `delta.tool_calls` (or the
//! non-streamed `message.tool_calls`) is translated into the neutral tool-call
//! contract. Execution stays on ZylCode's gated path.

use std::time::Duration;

use super::sse::SseDecoder;
use super::streaming::{
    CancelToken, ChatRequest, Completion, DecodedEvent, FinishReason, LineDecoder, ProviderError,
    ProviderErrorKind, StreamChunk, StreamEndpoint, StreamOutcome, ToolCall, ToolCallDelta,
    ToolCallDeltaKind, Usage,
};

/// Values of `finish_reason` this layer maps concretely.
fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFiltered,
        "tool_calls" | "function_call" => FinishReason::Stop,
        _ => FinishReason::Other,
    }
}

/// Build the streaming request body for a chat completions call.
pub fn chat_body(req: &ChatRequest, stream: bool) -> serde_json::Value {
    let mut messages = Vec::new();
    if !req.system.is_empty() {
        messages.push(serde_json::json!({ "role": "system", "content": req.system }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": req.prompt }));

    let mut body = serde_json::json!({
        "model": req.model,
        "messages": messages,
        "temperature": req.temperature,
        "max_tokens": req.max_tokens,
        "stream": stream,
    });
    if stream {
        // Without this, OpenAI omits usage entirely from streamed responses.
        body["stream_options"] = serde_json::json!({ "include_usage": true });
    }
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
        body["tool_choice"] = serde_json::json!(match req.tool_choice {
            crate::providers::streaming::ToolChoice::Auto => "auto",
            crate::providers::streaming::ToolChoice::None => "none",
            crate::providers::streaming::ToolChoice::Required => "required",
        });
    }
    body
}

/// Headers for an OpenAI-compatible endpoint.
pub fn headers(api_key: Option<&str>) -> Vec<(String, String)> {
    match api_key {
        Some(key) => vec![("Authorization".to_string(), format!("Bearer {key}"))],
        None => Vec::new(),
    }
}

/// Extract the answer and usage from a non-streamed chat completions response.
///
/// A completion with no `choices` (some gateways return that on a filtered prompt)
/// yields empty text rather than an error: the absence of an answer is not a
/// protocol violation, and the caller already decides how to treat empty text.
pub fn parse_completion(json: &serde_json::Value) -> Completion {
    let text = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .unwrap_or("")
        .to_string();

    // `tool_calls` is where this family asks for tools. `arguments` is a JSON
    // *string* in the protocol and stays a string here: parsing it at the
    // translation boundary would turn a model's malformed arguments into a
    // provider bug, when the caller needs to see exactly what was generated.
    let tool_calls = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("tool_calls"))
        .and_then(|calls| calls.as_array())
        .map(|calls| {
            calls
                .iter()
                .filter_map(|call| {
                    let function = call.get("function")?;
                    let name = function.get("name")?.as_str()?.to_string();
                    Some(ToolCall {
                        call_id: call
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string),
                        block_id: None,
                        name,
                        arguments: function
                            .get("arguments")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
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
                .and_then(|u| u.get("prompt_tokens"))
                .and_then(|v| v.as_u64()),
            output_tokens: json
                .get("usage")
                .and_then(|u| u.get("completion_tokens"))
                .and_then(|v| v.as_u64()),
            extra: None,
        },
    }
}

/// Extra headers OpenRouter documents for attribution.
///
/// Other OpenAI-compatible endpoints reject or ignore them, so they are applied
/// only for that provider by the caller.
pub fn openrouter_headers() -> Vec<(String, String)> {
    vec![
        (
            "HTTP-Referer".to_string(),
            "https://zylcode.dev".to_string(),
        ),
        ("X-Title".to_string(), "ZylCode".to_string()),
    ]
}

/// Streaming URL for a base URL such as `https://api.openai.com/v1`.
pub fn chat_url(base: &str) -> String {
    format!("{}/chat/completions", base.trim_end_matches('/'))
}

/// Decoder for an OpenAI-compatible chat completions event stream.
#[derive(Debug, Default)]
pub struct OpenAiCompatDecoder {
    sse: SseDecoder,
    finish: Option<FinishReason>,
    usage: Usage,
    provider_ref: Option<String>,
    /// OpenAI streams a tool call's id once and then refers to it by `index`, so
    /// the decoder has to remember the mapping to keep the call correlated.
    tool_call_ids: std::collections::HashMap<u64, String>,
}

impl OpenAiCompatDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn decode_event(&mut self, data: &str) -> Result<Vec<DecodedEvent>, ProviderError> {
        let trimmed = data.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        if trimmed == "[DONE]" {
            let mut event = DecodedEvent::done();
            event.finish = self.finish.or(Some(FinishReason::Stop));
            if self.usage.input_tokens.is_some() || self.usage.output_tokens.is_some() {
                event.usage = Some(self.usage.clone());
            }
            return Ok(vec![event]);
        }

        let json: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Malformed,
                format!("openai-compatible stream sent a frame that is not JSON: {e}"),
            )
        })?;

        // Some gateways report failures inline rather than as an HTTP status.
        if let Some(error) = json.get("error") {
            if !error.is_null() {
                let message = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("provider returned an error object");
                let mut provider_error =
                    ProviderError::new(ProviderErrorKind::ProviderRejected, message.to_string());
                provider_error.code = error
                    .get("code")
                    .and_then(|c| c.as_str())
                    .map(|c| c.to_string())
                    .or_else(|| {
                        error
                            .get("type")
                            .and_then(|t| t.as_str())
                            .map(|t| t.to_string())
                    });
                provider_error.raw = Some(error.clone());
                return Ok(vec![DecodedEvent::error(provider_error)]);
            }
        }

        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
            self.provider_ref = Some(id.to_string());
        }

        let mut events: Vec<DecodedEvent> = Vec::new();

        if let Some(usage) = json.get("usage").filter(|u| !u.is_null()) {
            let mapped = Usage::new(
                usage.get("prompt_tokens").and_then(|v| v.as_u64()),
                usage.get("completion_tokens").and_then(|v| v.as_u64()),
            );
            self.usage.merge(mapped.clone());
            events.push(DecodedEvent {
                usage: Some(mapped),
                provider_ref: self.provider_ref.clone(),
                ..DecodedEvent::default()
            });
        }

        let choice = json
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first());

        if let Some(choice) = choice {
            if let Some(delta) = choice.get("delta") {
                if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                    if !text.is_empty() {
                        let mut event = DecodedEvent::text(text);
                        event.provider_ref = self.provider_ref.clone();
                        events.push(event);
                    }
                }
                if let Some(tool_calls) = delta
                    .get("tool_calls")
                    .and_then(serde_json::Value::as_array)
                {
                    for call in tool_calls {
                        let index = call
                            .get("index")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0);
                        // The first frame carries the id; later frames for the same
                        // call carry only the index, so the id is remembered.
                        let key = call
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_string)
                            .or_else(|| self.tool_call_ids.get(&index).cloned())
                            .unwrap_or_else(|| format!("call-{index}"));
                        self.tool_call_ids.insert(index, key.clone());

                        let function = call.get("function");
                        if let Some(name) = function
                            .and_then(|f| f.get("name"))
                            .and_then(serde_json::Value::as_str)
                        {
                            events.push(DecodedEvent {
                                tool_call: Some(ToolCallDelta {
                                    kind: ToolCallDeltaKind::Start,
                                    call_id: Some(key.clone()),
                                    block_id: None,
                                    argument_delta: None,
                                    done: false,
                                    metadata: Some(serde_json::json!({ "name": name })),
                                }),
                                provider_ref: self.provider_ref.clone(),
                                ..DecodedEvent::default()
                            });
                        }
                        if let Some(arguments) = function
                            .and_then(|f| f.get("arguments"))
                            .and_then(serde_json::Value::as_str)
                            .filter(|a| !a.is_empty())
                        {
                            events.push(DecodedEvent {
                                tool_call: Some(ToolCallDelta {
                                    kind: ToolCallDeltaKind::ArgumentDelta,
                                    call_id: Some(key),
                                    block_id: None,
                                    argument_delta: Some(arguments.to_string()),
                                    done: false,
                                    metadata: None,
                                }),
                                provider_ref: self.provider_ref.clone(),
                                ..DecodedEvent::default()
                            });
                        }
                    }
                }
            }

            if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                self.finish = Some(map_finish_reason(reason));
                events.push(DecodedEvent {
                    finish: self.finish,
                    provider_ref: self.provider_ref.clone(),
                    ..DecodedEvent::default()
                });
            }
        }

        Ok(events)
    }
}

impl LineDecoder for OpenAiCompatDecoder {
    fn feed(&mut self, line: &str) -> Result<Vec<DecodedEvent>, ProviderError> {
        let Some(event) = self.sse.push_line(line) else {
            return Ok(Vec::new());
        };
        self.decode_event(&event.data)
    }

    fn finish(&mut self) -> Vec<DecodedEvent> {
        match self.sse.finish() {
            Some(event) => self
                .decode_event(&event.data)
                .unwrap_or_else(|err| vec![DecodedEvent::error(err)]),
            None => Vec::new(),
        }
    }
}

/// Where one OpenAI-compatible stream should go.
///
/// Grouped into a struct because the same five knobs (base URL, key, provider label,
/// timeout, vendor headers) are needed together and a nine-argument call is not
/// reviewable.
pub struct CompatTarget<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    /// Provider label used in diagnostics, e.g. `openrouter`.
    pub provider: &'a str,
    pub timeout: Duration,
    /// Vendor headers such as OpenRouter's attribution pair.
    pub extra_headers: &'a [(String, String)],
}

/// Stream one chat completions call, forwarding neutral chunks and honouring `cancel`.
pub async fn stream(
    client: &reqwest::Client,
    target: &CompatTarget<'_>,
    req: &ChatRequest,
    cancel: &CancelToken,
    on_chunk: &mut (dyn FnMut(StreamChunk) + Send),
) -> Result<StreamOutcome, ProviderError> {
    let mut endpoint =
        StreamEndpoint::new(target.provider, chat_url(target.base_url), target.timeout);
    for (name, value) in headers(target.api_key) {
        endpoint = endpoint.with_header(name, value);
    }
    for (name, value) in target.extra_headers {
        endpoint = endpoint.with_header(name.clone(), value.clone());
    }
    let body = chat_body(req, true);
    let mut decoder = OpenAiCompatDecoder::new();
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
        let mut decoder = OpenAiCompatDecoder::new();
        let mut out = Vec::new();
        for line in lines {
            out.extend(decoder.feed(line).expect("valid frame"));
        }
        out.extend(decoder.finish());
        out
    }

    #[test]
    fn parse_completion_reads_choices_message_and_usage() {
        let json = serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "pong" }, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 7, "completion_tokens": 2 }
        });

        let completion = parse_completion(&json);
        assert_eq!(completion.text, "pong");
        assert_eq!(completion.usage.input_tokens, Some(7));
        assert_eq!(completion.usage.output_tokens, Some(2));
    }

    #[test]
    fn parse_completion_tolerates_a_response_with_no_choices() {
        let completion = parse_completion(&serde_json::json!({ "usage": { "prompt_tokens": 3 } }));
        assert_eq!(completion.text, "");
        assert_eq!(completion.usage.input_tokens, Some(3));
        assert_eq!(completion.usage.output_tokens, None);
    }

    #[test]
    fn openrouter_headers_are_the_documented_pair() {
        let headers = openrouter_headers();
        assert_eq!(headers.len(), 2);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "HTTP-Referer" && v.contains("zylcode")));
        assert!(headers.iter().any(|(k, _)| k == "X-Title"));
    }

    fn chunk(delta: serde_json::Value, finish: Option<&str>) -> String {
        serde_json::json!({
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "choices": [{ "index": 0, "delta": delta, "finish_reason": finish }]
        })
        .to_string()
    }

    #[test]
    fn request_body_includes_system_message_and_stream_flags() {
        let req = ChatRequest::new("gpt-4o-mini", "be terse", "hi");
        let body = chat_body(&req, true);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn non_streaming_body_omits_stream_options() {
        let req = ChatRequest::new("m", "", "hi");
        let body = chat_body(&req, false);
        assert!(body.get("stream_options").is_none());
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn bearer_header_only_when_a_key_exists() {
        assert_eq!(headers(Some("sk")).len(), 1);
        assert_eq!(headers(Some("sk"))[0].0, "Authorization");
        assert!(headers(None).is_empty());
    }

    #[test]
    fn url_joins_without_double_slashes() {
        assert_eq!(
            chat_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn content_deltas_accumulate_and_done_terminates() {
        let events = decode(&[
            &format!(
                "data: {}",
                chunk(serde_json::json!({"role":"assistant"}), None)
            ),
            "",
            &format!(
                "data: {}",
                chunk(serde_json::json!({"content":"Hel"}), None)
            ),
            "",
            &format!("data: {}", chunk(serde_json::json!({"content":"lo"}), None)),
            "",
            &format!("data: {}", chunk(serde_json::json!({}), Some("stop"))),
            "",
            "data: [DONE]",
            "",
        ]);
        let text: String = events.iter().filter_map(|e| e.text.clone()).collect();
        assert_eq!(text, "Hello");
        let last = events.last().unwrap();
        assert!(last.done);
        assert_eq!(last.finish, Some(FinishReason::Stop));
    }

    #[test]
    fn usage_from_the_final_chunk_is_preserved() {
        let final_chunk = serde_json::json!({
            "id": "chatcmpl-1",
            "choices": [],
            "usage": { "prompt_tokens": 9, "completion_tokens": 4 }
        })
        .to_string();
        let events = decode(&[&format!("data: {final_chunk}"), "", "data: [DONE]", ""]);
        let usage = events
            .iter()
            .find_map(|e| e.usage.clone())
            .expect("usage chunk");
        assert_eq!(usage.input_tokens, Some(9));
        assert_eq!(usage.output_tokens, Some(4));
        let done = events.last().unwrap();
        assert!(done.done);
        assert_eq!(done.usage.as_ref().unwrap().input_tokens, Some(9));
    }

    #[test]
    fn inline_error_object_becomes_a_provider_error() {
        let events = decode(&[
            "data: {\"error\":{\"message\":\"rate limited\",\"code\":\"rate_limit_exceeded\"}}",
            "",
        ]);
        let err = events.last().unwrap().error.as_ref().expect("error");
        assert_eq!(err.kind, ProviderErrorKind::ProviderRejected);
        assert!(err.message.contains("rate limited"));
        assert_eq!(err.code.as_deref(), Some("rate_limit_exceeded"));
    }

    #[test]
    fn malformed_frame_is_reported() {
        let mut decoder = OpenAiCompatDecoder::new();
        assert!(decoder.feed("data: {oops").unwrap().is_empty());
        // SSE only dispatches an event when its block ends.
        let err = decoder.feed("").expect_err("must not silently drop");
        assert_eq!(err.kind, ProviderErrorKind::Malformed);
    }

    #[test]
    fn tool_call_deltas_are_translated_into_the_neutral_contract() {
        let frame = chunk(
            serde_json::json!({"tool_calls":[{"index":0,"id":"call_1","function":{"name":"x","arguments":"{"}}]}),
            None,
        );
        let events = decode(&[&format!("data: {frame}"), ""]);
        assert!(events.iter().all(|e| e.text.is_none()));
        let deltas: Vec<_> = events.iter().filter_map(|e| e.tool_call.clone()).collect();
        assert_eq!(deltas[0].kind, ToolCallDeltaKind::Start);
        assert_eq!(deltas[1].kind, ToolCallDeltaKind::ArgumentDelta);
        assert_eq!(deltas[1].argument_delta.as_deref(), Some("{"));
    }

    #[test]
    fn finish_reason_mapping_is_explicit() {
        assert_eq!(map_finish_reason("stop"), FinishReason::Stop);
        assert_eq!(map_finish_reason("length"), FinishReason::Length);
        assert_eq!(
            map_finish_reason("content_filter"),
            FinishReason::ContentFiltered
        );
        assert_eq!(map_finish_reason("weird"), FinishReason::Other);
    }
}
