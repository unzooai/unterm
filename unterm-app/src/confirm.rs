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

/// What the banner says, one line per row.
///
/// Kept narrow enough to read at a glance. The preview is truncated rather
/// than wrapped: a banner that grows with the input can cover the screen it is
/// asking about.
pub fn lines(agent: &str, method: &str, preview: &str, cols: usize) -> Vec<String> {
    let width = cols.clamp(24, 100);
    let fit = |text: String| -> String {
        if text.chars().count() <= width {
            text
        } else {
            let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
            out.push('…');
            out
        }
    };

    vec![
        fit(format!("{agent} wants to run {method}")),
        fit(format!("  {}", preview.trim())),
        fit("[Y] allow   [A] always allow   [Esc] refuse".to_string()),
    ]
}

/// Which decision a key press means, if any.
///
/// Deliberately not the same keys as anything the terminal uses: while the
/// banner is up these three do not reach the shell, and taking a key a program
/// needs would make the banner a hazard of its own.
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
        let lines = lines("claude", "session.input", "rm -rf /", 80);
        assert!(lines[0].contains("claude"));
        assert!(lines[0].contains("session.input"));
        assert!(lines[1].contains("rm -rf /"));
    }

    #[test]
    fn every_option_is_offered() {
        let lines = lines("claude", "session.input", "ls", 80);
        let keys = lines.last().expect("a key line");
        assert!(keys.contains("[Y]"));
        assert!(keys.contains("[A]"));
        assert!(keys.contains("[Esc]"));
    }

    #[test]
    fn a_long_command_is_cut_rather_than_allowed_to_cover_the_screen() {
        let long = "echo ".to_string() + &"x".repeat(500);
        for line in lines("claude", "session.input", &long, 40) {
            assert!(
                line.chars().count() <= 40,
                "banner line ran past the window: {line}"
            );
        }
    }

    #[test]
    fn a_narrow_window_still_gets_a_readable_banner() {
        // Below the floor the text would be cut to nothing, which is worse
        // than a banner that runs a little wide.
        for line in lines("claude", "session.input", "ls", 4) {
            assert!(line.chars().count() <= 24);
            assert!(!line.is_empty());
        }
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
