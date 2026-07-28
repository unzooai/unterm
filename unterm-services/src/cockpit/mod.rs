//! Agent Cockpit — the state engine that watches which AI agent runs in
//! which pane and what it is doing right now.
//!
//! Design: docs/design-agent-cockpit.md. The engine ingests four signal
//! layers (official hook events, OSC title/progress/notification parsing,
//! foreground-process detection, screen-text heuristics) and folds them
//! into one `AgentState` per pane. Everything else in the cockpit — the
//! tab badges, the top-bar chip, the Inbox palette, `agent.status` over
//! MCP — reads from this registry and never probes panes itself.

pub mod fleet;
pub mod observability;
pub mod review;
pub mod status;
pub mod title;
pub mod verification;

pub use status::{
    on_bell, on_hook_signal, on_notification, on_progress, on_title_change, on_user_input, poll,
    snapshot, status_for_pane, summary, tab_state, AgentState, PaneAgentStatus,
};
