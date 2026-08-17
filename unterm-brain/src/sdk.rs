//! The same brain, spoken over an SDK instead of a CLI.
//!
//! A CLI hands over one JSON object per finished thing. An SDK streams the
//! same turn as fragments: a block opens, deltas arrive, the block closes,
//! and usage is reported in pieces at both ends of the message. Nothing
//! downstream should have to care which of those it is talking to, so this
//! adapter buffers the fragments and emits the same events the CLI adapter
//! would.
//!
//! Two things follow from the shape of the SDK stream, and both are
//! decisions rather than details:
//!
//! **A block becomes one event, when it closes.** Emitting a `Text` per
//! delta would be honest about the wire and useless downstream: every reader
//! would have to re-assemble the sentence, and the equivalence test would
//! compare a paragraph against forty fragments of one.
//!
//! **`stop_reason: tool_use` is a pause, not an end.** The model stopped to
//! have a tool run and the turn continues in the next message. Reporting a
//! turn ending there would make one turn look like two, and a task counting
//! turns would double everything a tool-using agent did.

use crate::{BrainAdapter, BrainEvent, StopReason, Usage};
use serde_json::Value;

/// A content block being assembled.
#[derive(Debug)]
enum Block {
    Text(String),
    Reasoning(String),
    Tool {
        call_id: String,
        name: String,
        /// The SDK sends tool arguments as fragments of JSON text, which are
        /// only parseable once the block closes.
        json: String,
    },
    Other,
}

/// Reads a streaming SDK response.
#[derive(Default)]
pub struct SdkAdapter {
    turn_open: bool,
    block: Option<Block>,
    /// Summed over every message in the turn, and emitted once at the end.
    /// The SDK reports input at the start and output at the finish; a `Usage`
    /// event per fragment would leave every reader doing the addition.
    usage: Usage,
    session: Option<String>,
    stop: Option<StopReason>,
}

impl SdkAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    fn take_usage(&mut self) -> Usage {
        std::mem::take(&mut self.usage)
    }

    fn add_usage(&mut self, value: &Value) {
        self.usage.input_tokens += value
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        self.usage.output_tokens += value
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        self.usage.cached_input_tokens += value
            .get("cache_read_input_tokens")
            .or_else(|| value.get("cached_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
    }

    fn close_block(&mut self) -> Option<BrainEvent> {
        match self.block.take()? {
            Block::Text(text) if !text.is_empty() => Some(BrainEvent::Text { text }),
            Block::Reasoning(text) if !text.is_empty() => Some(BrainEvent::Reasoning { text }),
            Block::Tool {
                call_id,
                name,
                json,
            } => Some(BrainEvent::ToolRequested {
                call_id,
                name,
                // An empty argument stream means no arguments, which is not
                // the same as arguments that failed to parse — the second
                // becomes an error the operator can see.
                arguments: if json.trim().is_empty() {
                    Value::Object(Default::default())
                } else {
                    serde_json::from_str(&json).unwrap_or(Value::Null)
                },
            }),
            _ => None,
        }
    }
}

impl BrainAdapter for SdkAdapter {
    fn id(&self) -> &'static str {
        "sdk"
    }

    fn external_id(&self) -> Option<&str> {
        self.session.as_deref()
    }

    fn on_line(&mut self, line: &str) -> Vec<BrainEvent> {
        // SDK transports usually wrap the JSON in SSE framing; accept either,
        // since the framing is the transport's business and not the model's.
        let line = line.trim();
        let line = line.strip_prefix("data:").unwrap_or(line).trim();
        if line.is_empty() || line == "[DONE]" {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return vec![BrainEvent::Error {
                message: format!("sdk: unparsable line: {}", crate::adapters::truncate(line)),
            }];
        };
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mut events = Vec::new();

        match kind.as_str() {
            "message_start" => {
                let message = value.get("message").unwrap_or(&value);
                if let Some(id) = message
                    .get("session_id")
                    .or_else(|| value.get("session_id"))
                    .and_then(Value::as_str)
                {
                    self.session = Some(id.to_string());
                }
                if let Some(usage) = message.get("usage") {
                    self.add_usage(usage);
                }
                if !self.turn_open {
                    // Only the first message of a turn starts it. The second
                    // is the model resuming after a tool ran.
                    self.turn_open = true;
                    events.push(BrainEvent::TurnStarted {
                        model: message
                            .get("model")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    });
                }
            }
            "content_block_start" => {
                let block = value.get("content_block").unwrap_or(&Value::Null);
                self.block = Some(
                    match block.get("type").and_then(Value::as_str).unwrap_or("") {
                        "text" => Block::Text(String::new()),
                        "thinking" => Block::Reasoning(String::new()),
                        "tool_use" => Block::Tool {
                            call_id: block
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_string(),
                            name: block
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_string(),
                            json: String::new(),
                        },
                        _ => Block::Other,
                    },
                );
            }
            "content_block_delta" => {
                let delta = value.get("delta").unwrap_or(&Value::Null);
                let fragment = delta
                    .get("text")
                    .or_else(|| delta.get("thinking"))
                    .or_else(|| delta.get("partial_json"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match self.block.as_mut() {
                    Some(Block::Text(buffer)) | Some(Block::Reasoning(buffer)) => {
                        buffer.push_str(fragment)
                    }
                    Some(Block::Tool { json, .. }) => json.push_str(fragment),
                    _ => {}
                }
            }
            "content_block_stop" => {
                if let Some(event) = self.close_block() {
                    events.push(event);
                }
            }
            "tool_result" => {
                // Not part of the model's stream: an SDK harness runs the
                // tool itself and reports the outcome here. Without it the
                // SDK path would be the one place a tool result is invisible,
                // and the two paths would stop being comparable.
                events.push(BrainEvent::ToolResult {
                    call_id: value
                        .get("tool_use_id")
                        .or_else(|| value.get("call_id"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    ok: !value
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    output: value.get("content").map(|content| match content.as_str() {
                        Some(text) => text.to_string(),
                        None => content.to_string(),
                    }),
                });
            }
            "message_delta" => {
                if let Some(usage) = value.get("usage") {
                    self.add_usage(usage);
                }
                let reason = value
                    .get("delta")
                    .and_then(|delta| delta.get("stop_reason"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                self.stop = match reason {
                    "" => self.stop,
                    "tool_use" => {
                        // A pause. The turn continues once the tool has run.
                        None
                    }
                    "max_tokens" => Some(StopReason::Limit),
                    "refusal" => Some(StopReason::Completed),
                    _ => Some(StopReason::Completed),
                };
            }
            "message_stop" => {
                if let Some(reason) = self.stop.take() {
                    let usage = self.take_usage();
                    if usage != Usage::default() {
                        events.push(BrainEvent::Usage(usage));
                    }
                    self.turn_open = false;
                    events.push(BrainEvent::TurnEnded { reason });
                }
            }
            "error" => {
                self.turn_open = false;
                events.push(BrainEvent::Error {
                    message: value
                        .get("error")
                        .and_then(|error| error.get("message"))
                        .or_else(|| value.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("the sdk reported an error")
                        .to_string(),
                });
                events.push(BrainEvent::TurnEnded {
                    reason: StopReason::Error,
                });
            }
            _ => {}
        }
        events
    }

    fn on_eof(&mut self) -> Vec<BrainEvent> {
        let mut events = Vec::new();
        // A block still open when the stream died is half a sentence; losing
        // it silently would make a truncated answer look complete.
        if let Some(event) = self.close_block() {
            events.push(event);
        }
        if self.turn_open {
            self.turn_open = false;
            let usage = self.take_usage();
            if usage != Usage::default() {
                events.push(BrainEvent::Usage(usage));
            }
            events.push(BrainEvent::TurnEnded {
                reason: StopReason::Interrupted,
            });
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::ClaudeAdapter;
    use crate::replay;

    /// One turn over the SDK: think, call a tool, get its result, answer.
    /// Two messages, because a tool call splits a turn in the SDK's model.
    const SDK_STREAM: &str = r#"
data: {"type":"message_start","message":{"model":"gpt-5","session_id":"sess_9","usage":{"input_tokens":100,"cache_read_input_tokens":100}}}
data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking"}}
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I should list "}}
data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"the directory"}}
data: {"type":"content_block_stop","index":0}
data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_1","name":"shell"}}
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"comm"}}
data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"and\": \"ls\"}"}}
data: {"type":"content_block_stop","index":1}
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":10}}
data: {"type":"message_stop"}
data: {"type":"tool_result","tool_use_id":"call_1","is_error":false,"content":"a\nb"}
data: {"type":"message_start","message":{"model":"gpt-5","usage":{"input_tokens":20}}}
data: {"type":"content_block_start","index":0,"content_block":{"type":"text"}}
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"There are "}}
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"two files."}}
data: {"type":"content_block_stop","index":0}
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20}}
data: {"type":"message_stop"}
data: [DONE]
"#;

    /// The same turn as the Claude CLI prints it.
    const CLI_STREAM: &str = r#"
{"type":"system","subtype":"init","model":"gpt-5","session_id":"sess_9"}
{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"I should list the directory"},{"type":"tool_use","id":"call_1","name":"shell","input":{"command":"ls"}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"call_1","is_error":false,"content":"a\nb"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"There are two files."}]}}
{"type":"result","subtype":"success","usage":{"input_tokens":120,"output_tokens":30,"cache_read_input_tokens":100}}
"#;

    fn sorted(events: &[BrainEvent]) -> Vec<String> {
        let mut described: Vec<String> = events
            .iter()
            .map(|event| serde_json::to_string(event).unwrap())
            .collect();
        described.sort();
        described
    }

    #[test]
    fn the_sdk_and_the_cli_describe_the_same_turn_the_same_way() {
        // M4-05's gate. The SDK streams fragments and splits the turn in two;
        // the CLI hands over finished objects. Downstream must not be able to
        // tell, or every reader ends up with two code paths.
        let sdk = replay(&mut SdkAdapter::new(), SDK_STREAM);
        let cli = replay(&mut ClaudeAdapter::new(), CLI_STREAM);
        assert_eq!(sorted(&sdk), sorted(&cli), "\nsdk: {sdk:#?}\ncli: {cli:#?}");
    }

    #[test]
    fn a_tool_pause_is_not_the_end_of_a_turn() {
        // Otherwise a tool-using agent looks like it did twice the work, and
        // anything counting turns — budgets, limits, the Cockpit — doubles.
        let events = replay(&mut SdkAdapter::new(), SDK_STREAM);
        let starts = events
            .iter()
            .filter(|event| matches!(event, BrainEvent::TurnStarted { .. }))
            .count();
        let ends = events
            .iter()
            .filter(|event| matches!(event, BrainEvent::TurnEnded { .. }))
            .count();
        assert_eq!((starts, ends), (1, 1), "{events:#?}");
    }

    #[test]
    fn fragments_are_assembled_before_anyone_downstream_sees_them() {
        let events = replay(&mut SdkAdapter::new(), SDK_STREAM);
        assert!(events.contains(&BrainEvent::Text {
            text: "There are two files.".into()
        }));
        assert!(events.contains(&BrainEvent::Reasoning {
            text: "I should list the directory".into()
        }));
        // Including the tool arguments, which arrive as fragments of JSON text
        // and are not parseable until the block closes.
        assert!(events.contains(&BrainEvent::ToolRequested {
            call_id: "call_1".into(),
            name: "shell".into(),
            arguments: serde_json::json!({"command": "ls"}),
        }));
    }

    #[test]
    fn cost_is_reported_once_for_the_whole_turn() {
        let events = replay(&mut SdkAdapter::new(), SDK_STREAM);
        let usages: Vec<Usage> = events
            .iter()
            .filter_map(|event| match event {
                BrainEvent::Usage(usage) => Some(*usage),
                _ => None,
            })
            .collect();
        assert_eq!(usages.len(), 1, "cost was reported in pieces: {usages:?}");
        assert_eq!(usages[0].input_tokens, 120);
        assert_eq!(usages[0].output_tokens, 30);
        assert_eq!(usages[0].cached_input_tokens, 100);
    }

    #[test]
    fn a_stream_that_dies_mid_sentence_keeps_what_was_said() {
        // A truncated answer that vanished entirely would look like the model
        // said nothing, which is a worse account than a partial one.
        let events = replay(
            &mut SdkAdapter::new(),
            r#"{"type":"message_start","message":{"model":"m"}}
{"type":"content_block_start","index":0,"content_block":{"type":"text"}}
{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"half a sen"}}"#,
        );
        assert!(events.contains(&BrainEvent::Text {
            text: "half a sen".into()
        }));
        assert_eq!(
            events.last(),
            Some(&BrainEvent::TurnEnded {
                reason: StopReason::Interrupted
            })
        );
    }

    #[test]
    fn hitting_the_output_cap_is_a_limit_not_a_failure() {
        let events = replay(
            &mut SdkAdapter::new(),
            r#"{"type":"message_start","message":{"model":"m"}}
{"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":4000}}
{"type":"message_stop"}"#,
        );
        assert_eq!(
            events.last(),
            Some(&BrainEvent::TurnEnded {
                reason: StopReason::Limit
            })
        );
    }

    #[test]
    fn an_sdk_error_ends_the_turn() {
        let events = replay(
            &mut SdkAdapter::new(),
            r#"{"type":"message_start","message":{"model":"m"}}
{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#,
        );
        assert!(events.contains(&BrainEvent::Error {
            message: "overloaded".into()
        }));
        assert_eq!(
            events.last(),
            Some(&BrainEvent::TurnEnded {
                reason: StopReason::Error
            })
        );
    }

    #[test]
    fn the_sdk_learns_the_session_id_too() {
        let mut adapter = SdkAdapter::new();
        replay(&mut adapter, SDK_STREAM);
        assert_eq!(adapter.external_id(), Some("sess_9"));
    }

    #[test]
    fn sse_framing_is_the_transports_business() {
        // The same events whether or not the transport wrapped them.
        let framed = replay(&mut SdkAdapter::new(), SDK_STREAM);
        let bare: String = SDK_STREAM
            .lines()
            .map(|line| line.strip_prefix("data: ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(framed, replay(&mut SdkAdapter::new(), &bare));
    }

    #[test]
    fn the_sdk_adapter_performs_nothing_either() {
        let events = replay(
            &mut SdkAdapter::new(),
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"c","name":"shell"}}
{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"command\":\"rm -rf /\"}"}}
{"type":"content_block_stop","index":0}"#,
        );
        assert_eq!(events.len(), 1);
        assert!(events[0].is_tool_request());
    }
}
