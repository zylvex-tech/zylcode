//! Server-Sent Events framing.
//!
//! Anthropic and OpenAI-compatible chat completions both stream SSE. The framing
//! rules that matter here are:
//!
//! * an event is a run of `field: value` lines terminated by a blank line,
//! * `data:` lines for one event concatenate with `\n` between them,
//! * a line starting with `:` is a comment (keep-alive) and carries no payload,
//! * `event:` names the event type, and is absent on OpenAI's data-only streams.
//!
//! Framing lives here so provider modules only translate their own JSON payloads.

/// One dispatched SSE event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Value of the last `event:` field, if the provider sent one.
    pub event: Option<String>,
    /// Joined `data:` payload.
    pub data: String,
}

/// Incremental SSE event decoder driven one line at a time.
#[derive(Debug, Default)]
pub struct SseDecoder {
    event: Option<String>,
    data: String,
    has_data: bool,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one complete line. Returns the event when the line terminated a block.
    pub fn push_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            if !self.has_data {
                // A blank line with no data ends nothing; reset the event name so a
                // stray `event:` cannot leak into the next block.
                self.event = None;
                return None;
            }
            let event = SseEvent {
                event: self.event.take(),
                data: std::mem::take(&mut self.data),
            };
            self.has_data = false;
            return Some(event);
        }

        if line.starts_with(':') {
            return None;
        }

        let (field, value) = match line.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            // A field with no colon has an empty value.
            None => (line, ""),
        };

        match field {
            "event" => self.event = Some(value.to_string()),
            "data" => {
                if self.has_data {
                    self.data.push('\n');
                }
                self.data.push_str(value);
                self.has_data = true;
            }
            // `id`, `retry`, and unknown fields are irrelevant to payload parsing.
            _ => {}
        }
        None
    }

    /// Flush a block that ended because the connection closed rather than with a
    /// blank line. Providers do this often enough that treating it as truncation
    /// would be wrong.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.has_data {
            self.event = None;
            return None;
        }
        self.has_data = false;
        Some(SseEvent {
            event: self.event.take(),
            data: std::mem::take(&mut self.data),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(lines: &[&str]) -> Vec<SseEvent> {
        let mut decoder = SseDecoder::new();
        let mut out = Vec::new();
        for line in lines {
            if let Some(event) = decoder.push_line(line) {
                out.push(event);
            }
        }
        if let Some(event) = decoder.finish() {
            out.push(event);
        }
        out
    }

    #[test]
    fn one_data_line_per_event() {
        let events = decode(&["data: {\"a\":1}", "", "data: {\"a\":2}", ""]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "{\"a\":1}");
        assert_eq!(events[0].event, None);
        assert_eq!(events[1].data, "{\"a\":2}");
    }

    #[test]
    fn event_name_is_captured_and_reset() {
        let events = decode(&[
            "event: message_start",
            "data: {}",
            "",
            "data: {\"b\":2}",
            "",
        ]);
        assert_eq!(events[0].event.as_deref(), Some("message_start"));
        assert_eq!(events[1].event, None);
    }

    #[test]
    fn multiple_data_lines_join_with_newlines() {
        let events = decode(&["data: line1", "data: line2", ""]);
        assert_eq!(events[0].data, "line1\nline2");
    }

    #[test]
    fn comments_and_unknown_fields_are_ignored() {
        let events = decode(&[": keep-alive", "id: 7", "retry: 100", "data: x", ""]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "x");
    }

    #[test]
    fn blank_lines_without_data_emit_nothing() {
        assert!(decode(&["", "", ""]).is_empty());
    }

    #[test]
    fn unterminated_final_block_is_still_delivered() {
        let events = decode(&["data: tail"]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "tail");
    }

    #[test]
    fn field_without_colon_has_empty_value() {
        let events = decode(&["data", ""]);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "");
    }
}
