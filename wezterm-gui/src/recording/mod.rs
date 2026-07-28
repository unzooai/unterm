//! Recording a live pane through the mux.
//!
//! The session archive -- the index, redaction and markdown rendering -- moved
//! to `unterm-services`, because none of it needs a pane. What is left is the
//! recorder itself, which attaches to a mux pane and is therefore this front
//! end's business.

pub mod recorder;

pub use recorder::{
    attach_trace, export_active_recording_markdown, export_scrollback_markdown_for_session,
    list_sessions, read_session_markdown, recording_status, recording_status_snapshot,
    start_recording, stop_recording,
};

/// Apply the recording subsystem's secret patterns to product metadata.
pub(crate) fn redact_sensitive_text(text: &str) -> String {
    let config = recorder::load_config();
    unterm_services::recording::redact_sensitive_text(text, &config.redaction.custom_patterns)
}
