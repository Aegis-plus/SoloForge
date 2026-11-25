//! Parse Claude Code's `--output-format stream-json` lines.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum GmEvent {
    Init { session_id: String },
    /// A new assistant message started streaming.
    MessageStart,
    TextDelta(String),
    ToolStart(String),
    /// A complete assistant message: its text (may be empty) and tool names.
    Assistant { text: String, tools: Vec<String> },
    Result { is_error: bool, text: String, session_id: Option<String> },
    Other,
}

pub fn parse_line(line: &str) -> Option<GmEvent> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;
    let ty = v.get("type")?.as_str()?;
    Some(match ty {
        "system" if v.get("subtype").and_then(|s| s.as_str()) == Some("init") => {
            GmEvent::Init { session_id: v.get("session_id").and_then(|s| s.as_str()).unwrap_or_default().to_string() }
        }
        "stream_event" => {
            let ev = v.get("event")?;
            match ev.get("type").and_then(|t| t.as_str()) {
                Some("message_start") => GmEvent::MessageStart,
                Some("content_block_delta") => match ev.pointer("/delta/type").and_then(|t| t.as_str()) {
                    Some("text_delta") => GmEvent::TextDelta(ev.pointer("/delta/text").and_then(|t| t.as_str()).unwrap_or_default().to_string()),
                    _ => GmEvent::Other,
                },
                Some("content_block_start") if ev.pointer("/content_block/type").and_then(|t| t.as_str()) == Some("tool_use") => {
                    GmEvent::ToolStart(ev.pointer("/content_block/name").and_then(|t| t.as_str()).unwrap_or_default().to_string())
                }
                _ => GmEvent::Other,
            }
        }
        "assistant" => {
            let content = v.pointer("/message/content").and_then(|c| c.as_array()).cloned().unwrap_or_default();
            let text = content.iter().filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text")).filter_map(|b| b.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join("");
            let tools = content.iter().filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use")).filter_map(|b| b.get("name").and_then(|t| t.as_str()).map(String::from)).collect();
            GmEvent::Assistant { text, tools }
        }
        "result" => GmEvent::Result {
            is_error: v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false) || v.get("subtype").and_then(|s| s.as_str()).map(|s| s != "success").unwrap_or(false),
            text: v.get("result").and_then(|r| r.as_str()).unwrap_or_default().to_string(),
            session_id: v.get("session_id").and_then(|s| s.as_str()).map(String::from),
        },
        _ => GmEvent::Other,
    })
}

/// "mcp__vtt__move_token" → "move_token"
pub fn short_tool(name: &str) -> &str {
    name.rsplit("__").next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/stream.jsonl");

    #[test]
    fn parses_a_recorded_turn() {
        let evs: Vec<GmEvent> = FIXTURE.lines().filter_map(parse_line).collect();
        assert!(matches!(&evs[0], GmEvent::Init { session_id } if !session_id.is_empty()));
        let text: String = evs.iter().filter_map(|e| if let GmEvent::TextDelta(t) = e { Some(t.as_str()) } else { None }).collect();
        assert!(text.contains("Rain hisses"), "{text}");
        assert!(evs.iter().any(|e| matches!(e, GmEvent::ToolStart(n) if short_tool(n) == "get_state")));
        assert!(evs.iter().any(|e| matches!(e, GmEvent::Result { is_error: false, .. })));
    }

    #[test]
    fn ignores_noise() {
        assert_eq!(parse_line("not json"), None);
        assert_eq!(parse_line(r#"{"type":"rate_limit_event"}"#), Some(GmEvent::Other));
        assert_eq!(short_tool("mcp__vtt__roll"), "roll");
    }
}
