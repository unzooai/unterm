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
    attach_trace, export_pane_markdown, list_sessions, read_session_markdown,
    recording_status, start_recording, stop_recording,
};
