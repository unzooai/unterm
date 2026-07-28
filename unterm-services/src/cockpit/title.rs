//! Parse the OSC 0/2 window titles that CLI agents set into state signals.
//!
//! Verified against real agents (2026-07, see the design doc's research
//! notes):
//! * Claude Code — `✳ <task summary>` when idle, a braille spinner char
//!   (U+2800–U+28FF) prefix + summary while working.
//! * Codex — `⠼ <cwd name>` while working, bare cwd name when idle.
//! * Gemini CLI — `◇ Ready (<dir>)`, `✋ Action Required (<dir>)`,
//!   `⏲ Working… (<dir>)`, `✦ <thought>` (dynamicWindowTitle default-on).
//! * Aider — does not touch the title.

use super::status::AgentState;

/// What a title told us. `agent` is only set when the syntax is
/// unambiguous about which agent produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleSignal {
    pub state: AgentState,
    pub agent: Option<&'static str>,
    pub task_hint: Option<String>,
}

fn is_braille(c: char) -> bool {
    ('\u{2800}'..='\u{28FF}').contains(&c)
}

/// Strip a Gemini-style ` (<dir>)` suffix and surrounding padding.
fn gemini_hint(rest: &str) -> Option<String> {
    let rest = rest.trim();
    let rest = rest.strip_prefix('(').unwrap_or(rest);
    let rest = rest.strip_suffix(')').unwrap_or(rest);
    let rest = rest.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

pub fn parse_title(title: &str) -> Option<TitleSignal> {
    // Gemini pads titles to a fixed width; trim first.
    let t = title.trim();
    if t.is_empty() {
        return None;
    }
    let mut chars = t.chars();
    let first = chars.next()?;
    let rest = chars.as_str().trim();

    // Braille spinner prefix — Claude Code and Codex both use this while
    // a turn is running. The rest of the title is the task summary
    // (Claude) or the cwd name (Codex); either way it is the best hint
    // we have.
    if is_braille(first) {
        return Some(TitleSignal {
            state: AgentState::Working,
            agent: None,
            task_hint: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
        });
    }

    match first {
        // Claude Code idle: `✳ <summary>` (U+2733).
        '\u{2733}' => Some(TitleSignal {
            state: AgentState::Idle,
            agent: Some("claude"),
            task_hint: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
        }),
        // Gemini: `✋ Action Required (<dir>)`.
        '\u{270B}' => Some(TitleSignal {
            state: AgentState::WaitingForUser,
            agent: Some("gemini"),
            task_hint: None,
        }),
        // Gemini idle: `◇ Ready (<dir>)`.
        '\u{25C7}' => Some(TitleSignal {
            state: AgentState::Idle,
            agent: Some("gemini"),
            task_hint: gemini_hint(rest.strip_prefix("Ready").unwrap_or(rest)),
        }),
        // Gemini working: `⏲ Working… (<dir>)` or `✦ <thought>`.
        '\u{23F2}' => Some(TitleSignal {
            state: AgentState::Working,
            agent: Some("gemini"),
            task_hint: None,
        }),
        '\u{2726}' => Some(TitleSignal {
            state: AgentState::Working,
            agent: Some("gemini"),
            task_hint: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
        }),
        _ => None,
    }
}

/// Classify an OSC 9 / OSC 777 toast notification emitted by an agent.
/// Codex sends `approval-requested` / `agent-turn-complete` style texts,
/// Gemini sends attention / session-complete notifications. Keyword
/// matching is deliberately loose — a notification from an agent pane
/// that we cannot classify still means "wants attention".
pub fn classify_notification(title: Option<&str>, body: &str) -> AgentState {
    let hay = format!(
        "{} {}",
        title.unwrap_or_default().to_ascii_lowercase(),
        body.to_ascii_lowercase()
    );
    const DONE: &[&str] = &["complete", "completed", "finished", "done", "turn-complete"];
    const WAITING: &[&str] = &[
        "approval",
        "approve",
        "permission",
        "action required",
        "needs your",
        "waiting for",
        "confirm",
        "attention",
        "input requested",
    ];
    if WAITING.iter().any(|k| hay.contains(k)) {
        AgentState::WaitingForUser
    } else if DONE.iter().any(|k| hay.contains(k)) {
        AgentState::Done
    } else {
        AgentState::WaitingForUser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_idle_title() {
        let s = parse_title("✳ Fix the login bug").unwrap();
        assert_eq!(s.state, AgentState::Idle);
        assert_eq!(s.agent, Some("claude"));
        assert_eq!(s.task_hint.as_deref(), Some("Fix the login bug"));
    }

    #[test]
    fn braille_spinner_is_working() {
        for t in ["⠐ Respond with greeting", "⠼ unterm", "⠂x"] {
            let s = parse_title(t).unwrap();
            assert_eq!(s.state, AgentState::Working, "title {t:?}");
        }
    }

    #[test]
    fn gemini_titles() {
        let s = parse_title("✋ Action Required (unterm)").unwrap();
        assert_eq!(s.state, AgentState::WaitingForUser);
        assert_eq!(s.agent, Some("gemini"));

        let s = parse_title("◇ Ready (unterm)   ").unwrap();
        assert_eq!(s.state, AgentState::Idle);
        assert_eq!(s.task_hint.as_deref(), Some("unterm"));

        let s = parse_title("⏲ Working… (unterm)").unwrap();
        assert_eq!(s.state, AgentState::Working);

        let s = parse_title("✦ Thinking about tests").unwrap();
        assert_eq!(s.state, AgentState::Working);
        assert_eq!(s.task_hint.as_deref(), Some("Thinking about tests"));
    }

    #[test]
    fn plain_titles_are_ignored() {
        assert!(parse_title("zsh").is_none());
        assert!(parse_title("unterm").is_none());
        assert!(parse_title("").is_none());
        assert!(parse_title("vim README.md").is_none());
    }

    #[test]
    fn notification_classes() {
        assert_eq!(
            classify_notification(Some("Codex"), "approval-requested: run rm?"),
            AgentState::WaitingForUser
        );
        assert_eq!(
            classify_notification(None, "agent-turn-complete"),
            AgentState::Done
        );
        assert_eq!(
            classify_notification(Some("Gemini"), "Action Required"),
            AgentState::WaitingForUser
        );
        // Unclassifiable agent notifications still mean "look at me".
        assert_eq!(
            classify_notification(None, "hello"),
            AgentState::WaitingForUser
        );
    }
}
