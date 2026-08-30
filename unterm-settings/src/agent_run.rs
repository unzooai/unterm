//! Running a brain from the console.
//!
//! `agent_session.*` over MCP takes an argv, which is the right shape for a
//! trusted caller and the wrong one to put behind HTTP: a page that can pass
//! argv can run anything. So these routes take an **agent id** instead. The
//! command comes from that agent's manifest entry and the resolved binary
//! path that `installer::detect` hands back; the caller's own input only ever
//! reaches the prompt, which is written to the agent's stdin.
//!
//! What the caller can decide: which agent, what prompt, and which
//! task/run/step the events belong to. What it cannot decide: what runs.

use super::server::Response;
use serde_json::{json, Value};
use unterm_agents::{fetch_manifests, installer};
use unterm_mcp::handler::McpHandler;

/// Flags that put an agent into headless JSON mode.
///
/// A manifest's `launch.args` is written for a human at a terminal. The
/// adapters in `unterm-brain` read a JSON event stream instead, so the flags
/// that produce one live here, next to the code that depends on them.
fn headless_args(agent_id: &str) -> Option<&'static [&'static str]> {
    match agent_id {
        // ClaudeAdapter reads the `type: system|assistant|result` lines that
        // `--output-format stream-json` emits. `--verbose` is what makes it
        // stream rather than buffer to a single final message.
        "claude-code" => Some(&["-p", "--output-format", "stream-json", "--verbose"]),
        // CodexAdapter's JSONL is the reference format.
        "codex-cli" => Some(&["exec", "--json"]),
        _ => None,
    }
}

fn parse_body(body: &[u8]) -> Value {
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

fn string_field<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// POST /api/agent/start — run one agent against one prompt.
pub fn api_start(handler: &McpHandler, body: &[u8]) -> Response {
    let body = parse_body(body);

    let Some(agent_id) = string_field(&body, "agent") else {
        return Response::err(400, "Bad Request", "agent is required");
    };
    let Some(prompt) = string_field(&body, "prompt") else {
        return Response::err(400, "Bad Request", "prompt is required");
    };
    let Some(args) = headless_args(agent_id) else {
        return Response::err(
            400,
            "Bad Request",
            &format!("{agent_id} has no headless mode the console knows how to drive"),
        );
    };

    let set = match fetch_manifests() {
        Ok(set) => set,
        Err(e) => return Response::err(503, "Service Unavailable", &format!("manifest fetch: {e}")),
    };
    let Some(manifest) = set
        .for_current_platform()
        .into_iter()
        .find(|m| m.id == agent_id)
    else {
        return Response::err(404, "Not Found", &format!("no agent named {agent_id}"));
    };

    // Resolve once, here: on Windows the binary is a .cmd shim that a bare
    // name does not spawn, and the detected path is already absolute.
    let detected = installer::detect(&manifest.detect);
    if !detected.ok {
        return Response::err(
            503,
            "Service Unavailable",
            &format!("{} is not installed on this machine", manifest.name),
        );
    }
    let Some(binary) = detected.binary_path else {
        return Response::err(503, "Service Unavailable", "the agent has no resolved path");
    };

    let mut command = vec![json!(binary)];
    command.extend(args.iter().map(|arg| json!(arg)));

    let mut params = serde_json::Map::new();
    params.insert("command".into(), Value::Array(command));
    params.insert("prompt".into(), json!(prompt));
    // Carried onto every event and log line, so the console can thread them
    // back onto the task they belong to.
    for key in ["task_id", "run_id", "step_id", "idempotency_key"] {
        if let Some(value) = string_field(&body, key) {
            params.insert(key.into(), json!(value));
        }
    }

    let ctx = unterm_mcp::handler::ConnectionContext::internal("web_settings");
    match handler.handle(&ctx, "agent_session.start", &Value::Object(params)) {
        Ok(value) => Response::ok_json(json!({
            "agent": agent_id,
            "adapter": if agent_id == "claude-code" { "claude" } else { "codex" },
            "session": value,
        })),
        Err(e) => Response::err(400, "Bad Request", &e.to_string()),
    }
}

/// The rest of a session's life: events, status, input, interrupt, close.
///
/// These only ever address an existing session by id, so they pass through
/// with the fields each method takes and nothing else.
pub fn api_session_act(handler: &McpHandler, method: &str, body: &[u8]) -> Response {
    let body = parse_body(body);
    let Some(session_id) = string_field(&body, "session_id") else {
        return Response::err(400, "Bad Request", "session_id is required");
    };

    let mut params = serde_json::Map::new();
    params.insert("session_id".into(), json!(session_id));
    if let Some(cursor) = body.get("cursor").and_then(Value::as_u64) {
        params.insert("cursor".into(), json!(cursor));
    }
    if let Some(text) = string_field(&body, "text") {
        params.insert("text".into(), json!(text));
    }
    if let Some(grace) = body.get("grace_ms").and_then(Value::as_u64) {
        params.insert("grace_ms".into(), json!(grace));
    }

    let ctx = unterm_mcp::handler::ConnectionContext::internal("web_settings");
    match handler.handle(&ctx, method, &Value::Object(params)) {
        Ok(value) => Response::ok_json(value),
        Err(e) => Response::err(400, "Bad Request", &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_agents_with_a_known_headless_mode_are_runnable() {
        assert!(headless_args("claude-code").is_some());
        assert!(headless_args("codex-cli").is_some());
        // An agent we have not taught the console to drive is refused rather
        // than launched with whatever its interactive flags happen to be.
        assert!(headless_args("gemini-cli").is_none());
        assert!(headless_args("").is_none());
        assert!(headless_args("../../evil").is_none());
    }

    #[test]
    fn claude_gets_the_flags_its_adapter_parses() {
        let args = headless_args("claude-code").expect("claude");
        assert!(args.contains(&"-p"), "headless, not interactive");
        assert!(args.contains(&"stream-json"), "the adapter reads JSON lines");
        assert!(args.contains(&"--verbose"), "streamed, not buffered to one message");
    }

    #[test]
    fn a_start_without_an_agent_or_prompt_is_refused() {
        // No handler needed: both checks run before anything is dispatched.
        let missing_agent = parse_body(br#"{"prompt":"hi"}"#);
        assert!(string_field(&missing_agent, "agent").is_none());
        let missing_prompt = parse_body(br#"{"agent":"claude-code"}"#);
        assert!(string_field(&missing_prompt, "prompt").is_none());
        // An empty string is not a value.
        let empty = parse_body(br#"{"agent":"","prompt":""}"#);
        assert!(string_field(&empty, "agent").is_none());
        assert!(string_field(&empty, "prompt").is_none());
    }

    #[test]
    fn a_malformed_body_does_not_panic() {
        assert_eq!(parse_body(b"not json"), Value::Null);
        assert!(string_field(&parse_body(b"not json"), "agent").is_none());
    }
}
