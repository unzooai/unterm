//! The banner that asks before an agent writes to a shell.
//!
//! Bytes written to a pty are indistinguishable from bytes the user typed, so
//! the MCP surface parks the writing thread until someone says yes. Someone
//! has to be asked, and only a front end can ask: a surface with no window
//! would leave that thread parked until it timed out, and the agent would see
//! a refusal it could not explain.
//!
//! Laid out here rather than in the window so the wording can be tested
//! without a GPU.

/// What the banner says, as the one status-bar row it takes over --
/// 0.57.4 painted this question in the status row, and so does this.
///
/// The preview is truncated rather than wrapped: a banner that grows with the
/// input can cover the screen it is asking about. The key hints give way last,
/// because they are the part that answers the question.
pub fn status_line(agent: &str, method: &str, preview: &str, cols: usize) -> String {
    let width = cols.clamp(24, 240);
    const KEYS: &str = "[Enter] allow   [A] always allow   [Esc] refuse";
    let ask = format!("{agent} wants to run {method}:  {}", preview.trim());
    let room = width.saturating_sub(KEYS.chars().count() + 3);
    let ask = if ask.chars().count() > room {
        let mut cut: String = ask.chars().take(room.saturating_sub(1)).collect();
        if !cut.is_empty() {
            cut.push('…');
        }
        cut
    } else {
        ask
    };
    let line = if ask.is_empty() {
        KEYS.to_string()
    } else {
        format!("{ask}   {KEYS}")
    };
    if line.chars().count() > width {
        let mut cut: String = line.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        cut
    } else {
        line
    }
}

/// Which decision a typed character means, if any.
///
/// Enter and Escape arrive as named keys and are handled beside this; the
/// letters exist so the 0.57.4 muscle memory and the labelled keys both work.
/// While the banner is up no key reaches the shell, so none of these can leak.
pub fn decision_for(text: &str) -> Option<unterm_mcp::handler::ConfirmationDecision> {
    use unterm_mcp::handler::ConfirmationDecision;
    match text {
        t if t.eq_ignore_ascii_case("y") => Some(ConfirmationDecision::Allow),
        t if t.eq_ignore_ascii_case("a") => Some(ConfirmationDecision::AlwaysAllow),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_says_who_is_asking_and_for_what() {
        let line = status_line("claude", "session.input", "rm -rf /", 200);
        assert!(line.contains("claude"));
        assert!(line.contains("session.input"));
        assert!(line.contains("rm -rf /"));
    }

    #[test]
    fn every_option_is_offered() {
        let line = status_line("claude", "session.input", "ls", 200);
        assert!(line.contains("[Enter]"));
        assert!(line.contains("[A]"));
        assert!(line.contains("[Esc]"));
    }

    #[test]
    fn a_long_command_is_cut_rather_than_allowed_to_cover_the_screen() {
        let long = "echo ".to_string() + &"x".repeat(500);
        let line = status_line("claude", "session.input", &long, 100);
        assert!(
            line.chars().count() <= 100,
            "banner line ran past the window: {line}"
        );
        assert!(line.contains("[Esc]"), "the keys gave way: {line}");
    }

    #[test]
    fn a_narrow_window_still_gets_a_readable_banner() {
        // Below the floor the text would be cut to nothing, which is worse
        // than a banner that runs a little wide.
        let line = status_line("claude", "session.input", "ls", 4);
        assert!(line.chars().count() <= 24);
        assert!(!line.is_empty());
    }

    #[test]
    fn only_the_offered_keys_decide() {
        use unterm_mcp::handler::ConfirmationDecision;
        assert!(matches!(
            decision_for("y"),
            Some(ConfirmationDecision::Allow)
        ));
        assert!(matches!(
            decision_for("Y"),
            Some(ConfirmationDecision::Allow)
        ));
        assert!(matches!(
            decision_for("a"),
            Some(ConfirmationDecision::AlwaysAllow)
        ));
        assert!(decision_for("n").is_none(), "refusing is Esc, not a letter");
        assert!(decision_for("").is_none());
    }
}
