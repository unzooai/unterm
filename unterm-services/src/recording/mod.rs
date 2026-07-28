//! The session archive: what is on disk under `~/.unterm/sessions/`.
//!
//! Recording a live pane needs a pane, so that stays with whichever front end
//! has one -- next-core has its own recorder, and the GUI keeps the one
//! attached to a mux pane. What is here is everything that only needs the
//! files: the index, redaction, and rendering a log to markdown.
//!
//! Splitting it this way is what lets a second front end list and read the
//! same recordings without owning the recorder.

pub mod index;
pub mod redact;
pub mod render;

/// Apply the recording subsystem's secret patterns to short-lived product
/// metadata such as MCP audit entries.
///
/// Audit logs must never be a less-safe copy of the exported transcript.
pub fn redact_sensitive_text(text: &str, custom_patterns: &[String]) -> String {
    redact::redact(text, custom_patterns).0
}
