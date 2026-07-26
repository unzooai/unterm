//! Session recording subsystem.
//!
//! Stores the raw byte stream of a pane (the source of truth) plus
//! lightweight in-memory counters, and renders to redacted markdown
//! on demand.
//!
//! Storage layout (under `~/.unterm/sessions/`):
//!
//! ```text
//! ~/.unterm/sessions/
//! ├── index.json
//! ├── <project-slug>/<yyyy-mm-dd>/<tab-N>-<HHmmss>.md
//! ├── <project-slug>/<yyyy-mm-dd>/<tab-N>-<HHmmss>.log
//! └── _orphan/<yyyy-mm-dd>/...
//! ```

mod index;
pub mod recorder;
mod redact;
mod render;

pub use recorder::{
    attach_trace, export_active_recording_markdown, export_scrollback_markdown, list_sessions,
    read_session_markdown, recording_status, recording_status_snapshot, start_recording,
    stop_recording,
};

/// Apply the recording subsystem's built-in and user-configured secret
/// patterns to short-lived product metadata such as MCP audit entries.
/// Audit logs must never be a less-safe copy of the exported transcript.
pub(crate) fn redact_sensitive_text(text: &str) -> String {
    let config = recorder::load_config();
    redact::redact(text, &config.redaction.custom_patterns).0
}

#[cfg(test)]
mod tests {
    #[test]
    fn audit_redaction_removes_key_value_secrets() {
        let raw = "Write-Output 'api_key=sk-test-secret-should-redact'";
        let redacted = super::redact_sensitive_text(raw);
        assert!(!redacted.contains("sk-test-secret-should-redact"));
        assert!(redacted.contains("<redacted>"));
    }
}
