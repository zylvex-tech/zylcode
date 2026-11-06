//! The synthetic / offline provider.
//!
//! This is not a model. It produces a deterministic, clearly-labelled placeholder so
//! offline and CI runs stay functional, and it has two hard rules:
//!
//! 1. It never touches the network. There is no client, no endpoint, and no header
//!    construction here, so nothing can egress even if a credential is present.
//! 2. It never claims to have done anything. The payload says a synthetic response
//!    was used and lists that as a remaining limitation; it does not report a
//!    successful deployment, a verified change, or a completed external task.
//!
//! Every chunk it emits carries `synthetic: true` so a consumer can label it without
//! inferring provenance from payload shape.

use super::streaming::{FinishReason, StreamChunk, StreamOutcome};

/// The deterministic offline payload, in the shape the agent protocol expects.
pub fn response(prompt: &str, _system: &str) -> String {
    serde_json::json!({
        "action": "Complete",
        "payload": {
            "summary": format!("Task completed: {}", prompt),
            "evidence": ["Synthetic offline response"],
            "remaining_limitations": [
                "Using synthetic offline mode - configure API keys for live LLM generation"
            ]
        }
    })
    .to_string()
}

/// Stream the synthetic payload as labelled chunks.
///
/// The chunks exist so a UI has progress to render, not because a provider streamed
/// anything. `finish` is [`FinishReason::Stop`] and `chunks` counts what was
/// forwarded; no usage is reported because no provider measured any.
pub fn stream(
    prompt: &str,
    system: &str,
    on_chunk: &mut (dyn FnMut(StreamChunk) + Send),
) -> StreamOutcome {
    let full = response(prompt, system);
    let mut index = 0u64;
    for line in full.lines() {
        let chunk = StreamChunk::text(index, line, true);
        on_chunk(chunk);
        index += 1;
    }
    let mut terminal = StreamChunk::terminal(index, true);
    terminal.finish_reason = Some(FinishReason::Stop);
    on_chunk(terminal);
    StreamOutcome {
        text: full,
        finish: FinishReason::Stop,
        usage: None,
        chunks: index + 1,
        provider_ref: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_is_deterministic_and_labelled() {
        let a = response("build a counter", "sys");
        let b = response("build a counter", "sys");
        assert_eq!(a, b);
        assert!(a.contains("Synthetic offline response"));
        assert!(a.contains("configure API keys"));
    }

    #[test]
    fn payload_never_claims_a_real_side_effect() {
        let payload = response("deploy the app", "sys");
        let lower = payload.to_lowercase();
        for forbidden in ["deployed to", "successfully deployed", "pushed to origin"] {
            assert!(
                !lower.contains(forbidden),
                "synthetic payload must not claim {forbidden:?}: {payload}"
            );
        }
    }

    #[test]
    fn every_chunk_is_marked_synthetic_and_the_stream_terminates_once() {
        let mut chunks = Vec::new();
        let outcome = stream("hi", "", &mut |c| chunks.push(c));
        assert_eq!(outcome.finish, FinishReason::Stop);
        assert!(outcome.usage.is_none());
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| c.synthetic));
        let terminals: Vec<_> = chunks.iter().filter(|c| c.done).collect();
        assert_eq!(terminals.len(), 1);
        assert_eq!(chunks.last().unwrap().index, chunks.len() as u64 - 1);
    }

    #[test]
    fn streaming_reassembles_the_payload() {
        let mut text = String::new();
        let outcome = stream("build a counter", "", &mut |c| {
            if let Some(delta) = c.kind.text {
                text.push_str(&delta);
                text.push('\n');
            }
        });
        assert_eq!(text.trim_end(), outcome.text);
    }
}
