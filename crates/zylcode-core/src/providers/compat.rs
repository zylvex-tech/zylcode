//! Application-boundary compatibility projection for streamed output.
//!
//! [`StreamChunk`](super::streaming::StreamChunk) is the streaming contract for
//! this crate. The desktop IPC layer, however, already ships three fields to the
//! frontend (`{ index, delta, done }`) and the UI renders exactly that shape, so
//! changing the payload would break a consumer that is not part of this contract
//! change.
//!
//! Rather than keep a second streaming producer alive, this module holds one
//! explicitly lossy projection used only where an existing consumer needs the old
//! payload. It is not an alternative streaming path:
//!
//! - nothing is produced from a provider stream directly into [`StreamEvent`];
//! - the only constructor is `From<&StreamChunk>`, so the chunk is always the
//!   source of truth and the projection cannot drift from it;
//! - information the projection drops (usage, finish reason, tool-call deltas,
//!   structured errors, synthetic provenance) stays available on the chunk itself,
//!   which is what new consumers should read.
//!
//! Callers that can read [`StreamChunk`](super::streaming::StreamChunk) should do
//! so instead of using this type.

use serde::{Deserialize, Serialize};

use super::streaming::StreamChunk;

/// The pre-existing three-field stream payload, kept for consumers that already
/// render it.
///
/// Fields are deliberately limited to what those consumers use; this type is a
/// projection, not a description of what the stream carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Index of the chunk within one stream.
    pub index: u64,
    /// Text delta, or an empty string for a chunk that carried no text.
    pub delta: String,
    /// Whether this chunk ended the stream.
    pub done: bool,
}

impl From<&StreamChunk> for StreamEvent {
    fn from(chunk: &StreamChunk) -> Self {
        Self {
            index: chunk.index,
            // A tool-call or usage-only chunk has no text; an empty delta is the
            // honest projection rather than a placeholder string.
            delta: chunk.kind.text.clone().unwrap_or_default(),
            done: chunk.done,
        }
    }
}

impl From<StreamChunk> for StreamEvent {
    fn from(chunk: StreamChunk) -> Self {
        Self::from(&chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::streaming::{ChunkKind, FinishReason, Usage};

    fn tool_only_chunk() -> StreamChunk {
        let mut chunk = StreamChunk::text(3, "", false);
        chunk.kind = ChunkKind {
            text: None,
            tool_call: Some(crate::providers::streaming::ToolCallDelta {
                kind: crate::providers::streaming::ToolCallDeltaKind::Start,
                call_id: Some("call_1".into()),
                block_id: None,
                argument_delta: None,
                done: false,
                metadata: None,
            }),
        };
        chunk
    }

    #[test]
    fn text_chunk_projects_its_delta_and_index() {
        let event = StreamEvent::from(&StreamChunk::text(2, "hello", false));
        assert_eq!(event.index, 2);
        assert_eq!(event.delta, "hello");
        assert!(!event.done);
    }

    #[test]
    fn tool_only_chunk_projects_an_empty_delta_rather_than_inventing_text() {
        let event = StreamEvent::from(&tool_only_chunk());
        assert_eq!(event.delta, "");
        assert!(!event.done);
    }

    #[test]
    fn terminal_chunk_projects_done() {
        let event = StreamEvent::from(&StreamChunk::terminal(7, true));
        assert_eq!(event.index, 7);
        assert!(event.done);
        assert_eq!(event.delta, "");
    }

    #[test]
    fn projection_drops_fields_that_remain_on_the_chunk() {
        let mut chunk = StreamChunk::text(0, "x", true);
        chunk.finish_reason = Some(FinishReason::Length);
        chunk.usage = Some(Usage {
            input_tokens: Some(5),
            output_tokens: Some(1),
            extra: None,
        });

        let event = StreamEvent::from(&chunk);

        // The projection is lossy by design, so these must still be true of the
        // chunk: a consumer that needs them reads the chunk, not this type.
        assert!(chunk.synthetic);
        assert_eq!(chunk.finish_reason, Some(FinishReason::Length));
        assert_eq!(chunk.usage.as_ref().and_then(|u| u.input_tokens), Some(5));
        assert_eq!(event.delta, "x");
    }

    #[test]
    fn projection_serialises_to_the_payload_existing_consumers_read() {
        let json = serde_json::to_value(StreamEvent::from(&StreamChunk::text(1, "ab", false)))
            .expect("projection must serialise");
        assert_eq!(
            json,
            serde_json::json!({ "index": 1, "delta": "ab", "done": false })
        );
    }
}
