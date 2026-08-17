//! Agreeing on a protocol version, or refusing to.
//!
//! MCP versions are dates, and a server may answer an `initialize` with a
//! version *other* than the one asked for — Unzoo, asked for 2025-06-18,
//! answers 2024-11-05. That is normal and fine. What is not fine is
//! continuing when the answer is a version this side does not implement:
//! everything appears to work until a message means something different than
//! it used to, and that failure surfaces as data, not as an error.
//!
//! So: offer the newest we speak, accept anything in our list, refuse
//! anything else by name.

/// What both sides settled on.
pub fn settle(ours: &[&str], theirs: &str) -> Result<String, String> {
    if ours.is_empty() {
        return Err("this build declares no protocol versions".to_string());
    }
    if ours.contains(&theirs) {
        return Ok(theirs.to_string());
    }
    Err(format!(
        "the provider answered with protocol {theirs}, which this build does not speak (it speaks {})",
        ours.join(", ")
    ))
}

/// The version to ask for: the newest this side speaks.
pub fn preferred(ours: &[&str]) -> Option<String> {
    ours.first().map(|version| version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::PROTOCOLS;

    #[test]
    fn a_downgrade_to_something_we_speak_is_fine() {
        // Exactly what the real Unzoo service does.
        assert_eq!(settle(PROTOCOLS, "2024-11-05").unwrap(), "2024-11-05");
    }

    #[test]
    fn a_version_we_do_not_speak_is_refused_by_name() {
        let error = settle(PROTOCOLS, "2099-01-01").unwrap_err();
        assert!(error.contains("2099-01-01"), "{error}");
        // And says what we do speak, because the person reading this has to
        // decide whether to upgrade Unterm or the provider.
        assert!(error.contains("2025-06-18"), "{error}");
    }

    #[test]
    fn we_ask_for_the_newest_we_speak() {
        assert_eq!(preferred(PROTOCOLS).as_deref(), Some("2025-06-18"));
    }

    #[test]
    fn nothing_in_common_is_an_error_rather_than_a_guess() {
        // The failure this prevents: proceeding on a version neither side
        // implements, where every message appears to work until one means
        // something different than it used to.
        assert!(settle(&[], "2024-11-05").is_err());
        assert!(settle(&["2024-11-05"], "").is_err());
    }
}
