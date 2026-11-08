//! Provider-native tool calling, translated and verified per protocol.
//!
//! These tests are about the **translation boundary**, not about execution. Each
//! provider family sends a different shape and answers with a different one; the
//! claim in `capabilities.rs` is only honest if the body builder and the response
//! parser both exist for that provider. So every family asserted `tools: true`
//! has a test here that builds a request containing a tool and a test that reads
//! a tool call back out of a response — including the streaming path, where the
//! arguments arrive in pieces.

use serde_json::json;

use super::capabilities::provider_capabilities;
use super::streaming::{ChatRequest, LineDecoder, ToolCallDeltaKind, ToolChoice, ToolSpec};
use super::{anthropic, ollama, openai_compatible};
use crate::router::ModelProvider;

fn tool() -> ToolSpec {
    ToolSpec {
        name: "fs.read".to_string(),
        description: "Read a file from the workspace.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }),
    }
}

fn request() -> ChatRequest {
    ChatRequest::new("test-model", "be useful", "read a.rs").with_tools(vec![tool()])
}

// ---------------------------------------------------------------------------
// OpenAI-compatible family: OpenAI, OpenRouter, DeepSeek, Respan, custom
// ---------------------------------------------------------------------------

#[test]
fn openai_body_declares_the_tool_and_the_choice() {
    let body = openai_compatible::chat_body(&request(), true);
    let tools = body["tools"].as_array().expect("tools must be sent");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
    // Dots are not legal in an OpenAI function name, so the wire name is sanitised.
    assert_eq!(tools[0]["function"]["name"], "fs_read");
    assert_eq!(tool().wire_name(), "fs_read");
    assert_eq!(tool().name, "fs.read", "the ZylCode id is unchanged");
    assert_eq!(tools[0]["function"]["parameters"]["required"][0], "path");
    assert_eq!(body["tool_choice"], "auto");
}

#[test]
fn openai_body_omits_tools_entirely_when_none_are_declared() {
    let body = openai_compatible::chat_body(&ChatRequest::new("m", "", "hi"), false);
    assert!(
        body.get("tools").is_none(),
        "a request without tools must not send an empty declaration"
    );
    assert!(body.get("tool_choice").is_none());
}

#[test]
fn openai_choice_maps_required_and_none() {
    let mut req = request();
    req.tool_choice = ToolChoice::Required;
    assert_eq!(
        openai_compatible::chat_body(&req, false)["tool_choice"],
        "required"
    );
    req.tool_choice = ToolChoice::None;
    assert_eq!(
        openai_compatible::chat_body(&req, false)["tool_choice"],
        "none"
    );
}

#[test]
fn openai_completion_reads_tool_calls_as_text_arguments() {
    let response = json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": { "name": "fs.read", "arguments": "{\"path\":\"a.rs\"}" }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": { "prompt_tokens": 4, "completion_tokens": 2 }
    });
    let completion = openai_compatible::parse_completion(&response);
    assert_eq!(completion.tool_calls.len(), 1);
    let call = &completion.tool_calls[0];
    assert_eq!(call.name, "fs.read");
    assert_eq!(call.call_id.as_deref(), Some("call_1"));
    assert_eq!(call.arguments, "{\"path\":\"a.rs\"}");
    assert!(call.done);
    assert!(
        completion.text.is_empty(),
        "a tool call is not assistant text and must not be concatenated into it"
    );
}

#[test]
fn openai_stream_reassembles_a_call_split_across_frames() {
    let mut decoder = openai_compatible::OpenAiCompatDecoder::new();
    let mut first = decoder
        .feed(r#"data: {"id":"gen-1","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"fs.read","arguments":"{\"path\":"}}]}}]}"#)
        .expect("first frame");
    first.extend(decoder.feed("").expect("blank line ends the first event"));
    let mut second = decoder
        .feed(r#"data: {"id":"gen-1","choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"a.rs\"}"}}]}}]}"#)
        .expect("second frame");
    second.extend(decoder.feed("").expect("blank line ends the second event"));

    let deltas: Vec<_> = first
        .iter()
        .chain(second.iter())
        .filter_map(|e| e.tool_call.clone())
        .collect();
    assert_eq!(deltas.len(), 3, "start plus two argument fragments");
    assert_eq!(deltas[0].kind, ToolCallDeltaKind::Start);
    assert_eq!(
        deltas[0].metadata.as_ref().and_then(|m| m.get("name")),
        Some(&json!("fs.read"))
    );
    assert_eq!(deltas[1].kind, ToolCallDeltaKind::ArgumentDelta);
    assert_eq!(
        deltas[2].call_id.as_deref(),
        Some("call_1"),
        "the second frame omits the id, so it must be correlated by index"
    );
}

// ---------------------------------------------------------------------------
// Anthropic Messages API
// ---------------------------------------------------------------------------

#[test]
fn anthropic_body_uses_input_schema_and_any_for_required() {
    let mut req = request();
    req.tool_choice = ToolChoice::Required;
    let body = anthropic::messages_body(&req, true);
    assert_eq!(body["tools"][0]["name"], "fs_read");
    assert_eq!(body["tools"][0]["input_schema"]["required"][0], "path");
    assert_eq!(
        body["tool_choice"]["type"], "any",
        "Anthropic spells must-call `any`, not `required`"
    );
}

#[test]
fn anthropic_completion_reads_tool_use_blocks_and_ignores_them_as_text() {
    let response = json!({
        "content": [
            { "type": "text", "text": "reading" },
            { "type": "tool_use", "id": "toolu_1", "name": "fs.read",
              "input": { "path": "a.rs" } }
        ],
        "usage": { "input_tokens": 5, "output_tokens": 3 }
    });
    let completion = anthropic::parse_completion(&response);
    assert_eq!(completion.text, "reading");
    assert_eq!(completion.tool_calls.len(), 1);
    let call = &completion.tool_calls[0];
    assert_eq!(call.block_id.as_deref(), Some("toolu_1"));
    assert_eq!(call.name, "fs.read");
    assert_eq!(call.arguments, "{\"path\":\"a.rs\"}");
}

#[test]
fn anthropic_stream_translates_start_arguments_and_stop() {
    let mut decoder = anthropic::AnthropicDecoder::new();
    let mut start = decoder
        .feed(r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"fs.read","input":{}}}"#)
        .expect("start");
    start.extend(decoder.feed("").expect("frame end"));
    let mut delta = decoder
        .feed(r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"a.rs\"}"}}"#)
        .expect("delta");
    delta.extend(decoder.feed("").expect("frame end"));
    let mut stop = decoder
        .feed(r#"data: {"type":"content_block_stop","index":0}"#)
        .expect("stop");
    stop.extend(decoder.feed("").expect("frame end"));

    let kinds: Vec<ToolCallDeltaKind> = start
        .iter()
        .chain(delta.iter())
        .chain(stop.iter())
        .filter_map(|e| e.tool_call.as_ref().map(|d| d.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ToolCallDeltaKind::Start,
            ToolCallDeltaKind::ArgumentDelta,
            ToolCallDeltaKind::Done
        ]
    );
    let block_ids: Vec<_> = start
        .iter()
        .chain(delta.iter())
        .chain(stop.iter())
        .filter_map(|e| e.tool_call.as_ref())
        .filter_map(|d| d.block_id.clone())
        .collect();
    assert_eq!(block_ids, vec!["toolu_1", "toolu_1", "toolu_1"]);
}

// ---------------------------------------------------------------------------
// Ollama
// ---------------------------------------------------------------------------

#[test]
fn ollama_body_declares_tools_in_the_openai_shape() {
    let body = ollama::chat_body(&request(), true);
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["function"]["name"], "fs_read");
}

#[test]
fn ollama_body_sends_no_tools_when_none_are_declared() {
    let body = ollama::chat_body(&ChatRequest::new("m", "", "hi"), false);
    assert!(body.get("tools").is_none());
}

#[test]
fn ollama_completion_serialises_object_arguments() {
    let response = json!({
        "message": {
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "function": { "name": "fs.read", "arguments": { "path": "a.rs" } }
            }]
        },
        "prompt_eval_count": 3,
        "eval_count": 1
    });
    let completion = ollama::parse_completion(&response);
    assert_eq!(completion.tool_calls.len(), 1);
    let call = &completion.tool_calls[0];
    assert_eq!(call.name, "fs.read");
    let parsed: serde_json::Value =
        serde_json::from_str(&call.arguments).expect("arguments must stay valid JSON");
    assert_eq!(parsed["path"], "a.rs");
}

#[test]
fn ollama_stream_emits_start_arguments_and_done() {
    let mut decoder = ollama::OllamaDecoder::new();
    let events = decoder
        .feed(r#"{"model":"llama3.1","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"fs.read","arguments":{"path":"a.rs"}}}]},"done":false}"#)
        .expect("frame");
    let kinds: Vec<ToolCallDeltaKind> = events
        .iter()
        .filter_map(|e| e.tool_call.as_ref().map(|d| d.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            ToolCallDeltaKind::Start,
            ToolCallDeltaKind::ArgumentDelta,
            ToolCallDeltaKind::Done
        ]
    );
}

// ---------------------------------------------------------------------------
// The table matches the parsers above
// ---------------------------------------------------------------------------

#[test]
fn every_provider_claiming_tools_has_a_translation_tested_here() {
    for (provider, family) in [
        (ModelProvider::OpenAi, "openai-compatible"),
        (ModelProvider::OpenRouter, "openai-compatible"),
        (ModelProvider::DeepSeek, "openai-compatible"),
        (ModelProvider::Respan, "openai-compatible"),
        (ModelProvider::OpenAiCompatible, "openai-compatible"),
        (ModelProvider::Anthropic, "anthropic"),
        (ModelProvider::LocalOllama, "ollama"),
    ] {
        assert!(
            provider_capabilities(&provider).tools,
            "{provider} claims tools, so {family} must be tested above"
        );
    }
    assert!(
        !provider_capabilities(&ModelProvider::SyntheticOffline).tools,
        "the offline path has no wire; it must not claim tool calling"
    );
}
