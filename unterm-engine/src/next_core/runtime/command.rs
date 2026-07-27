#![allow(dead_code)]

use crate::{CreateSessionRequest, ScrollbackTextRequest, SplitSessionRequest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::next_core) enum RuntimeCommandClass {
    SessionLifecycle,
    Input,
    ScreenRead,
    ScreenMutation,
    Recording,
    Status,
}

#[derive(Debug)]
pub(in crate::next_core) enum RuntimeCommand {
    CreateSession(Box<CreateSessionRequest>),
    SplitSession(SplitSessionRequest),
    FocusSession {
        pane_id: usize,
    },
    ResizeSession {
        pane_id: usize,
        cols: usize,
        rows: usize,
    },
    DestroySession {
        pane_id: usize,
    },
    WriteInput {
        pane_id: usize,
        text: String,
    },
    PasteInput {
        pane_id: usize,
        text: String,
    },
    ScrollViewport {
        pane_id: usize,
        target: isize,
    },
    ReadScreen {
        pane_id: usize,
    },
    ReadRenderFrame {
        pane_id: usize,
        since_revision: Option<u64>,
    },
    ReadScrollbackText {
        pane_id: usize,
        request: ScrollbackTextRequest,
    },
    SearchScreen {
        pane_id: usize,
        pattern: String,
        max_results: usize,
    },
    StartRecording {
        pane_id: usize,
    },
    StopRecording {
        pane_id: usize,
    },
    RecordingStatus {
        pane_id: usize,
    },
    ShellSnapshot {
        pane_id: usize,
    },
    SessionActivity {
        pane_id: usize,
    },
    HealthSnapshot,
}

impl RuntimeCommand {
    pub(in crate::next_core) fn class(&self) -> RuntimeCommandClass {
        match self {
            Self::CreateSession(_)
            | Self::SplitSession(_)
            | Self::FocusSession { .. }
            | Self::ResizeSession { .. }
            | Self::DestroySession { .. } => RuntimeCommandClass::SessionLifecycle,
            Self::WriteInput { .. } | Self::PasteInput { .. } => RuntimeCommandClass::Input,
            Self::ScrollViewport { .. } => RuntimeCommandClass::ScreenMutation,
            Self::ReadScreen { .. }
            | Self::ReadRenderFrame { .. }
            | Self::ReadScrollbackText { .. }
            | Self::SearchScreen { .. } => RuntimeCommandClass::ScreenRead,
            Self::StartRecording { .. }
            | Self::StopRecording { .. }
            | Self::RecordingStatus { .. } => RuntimeCommandClass::Recording,
            Self::ShellSnapshot { .. } | Self::SessionActivity { .. } | Self::HealthSnapshot => {
                RuntimeCommandClass::Status
            }
        }
    }

    pub(in crate::next_core) fn pane_id(&self) -> Option<usize> {
        match self {
            Self::CreateSession(_) | Self::HealthSnapshot => None,
            Self::SplitSession(request) => Some(request.source_pane_id),
            Self::FocusSession { pane_id }
            | Self::ResizeSession { pane_id, .. }
            | Self::DestroySession { pane_id }
            | Self::WriteInput { pane_id, .. }
            | Self::PasteInput { pane_id, .. }
            | Self::ScrollViewport { pane_id, .. }
            | Self::ReadScreen { pane_id }
            | Self::ReadRenderFrame { pane_id, .. }
            | Self::ReadScrollbackText { pane_id, .. }
            | Self::SearchScreen { pane_id, .. }
            | Self::StartRecording { pane_id }
            | Self::StopRecording { pane_id }
            | Self::RecordingStatus { pane_id }
            | Self::ShellSnapshot { pane_id }
            | Self::SessionActivity { pane_id } => Some(*pane_id),
        }
    }

    pub(in crate::next_core) fn is_write_path(&self) -> bool {
        matches!(
            self,
            Self::CreateSession(_)
                | Self::SplitSession(_)
                | Self::FocusSession { .. }
                | Self::ResizeSession { .. }
                | Self::DestroySession { .. }
                | Self::WriteInput { .. }
                | Self::PasteInput { .. }
                | Self::ScrollViewport { .. }
                | Self::StartRecording { .. }
                | Self::StopRecording { .. }
        )
    }

    pub(in crate::next_core) fn latency_sensitive(&self) -> bool {
        matches!(
            self,
            Self::WriteInput { .. }
                | Self::PasteInput { .. }
                | Self::FocusSession { .. }
                | Self::ReadRenderFrame { .. }
        )
    }

    pub(in crate::next_core) fn input_bytes(&self) -> usize {
        match self {
            Self::WriteInput { text, .. } | Self::PasteInput { text, .. } => text.len(),
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::next_core) struct RuntimeQueuePolicy {
    pub max_pending_commands: usize,
    pub max_pending_input_bytes: usize,
    pub max_render_wakeups_per_second: u16,
}

impl Default for RuntimeQueuePolicy {
    fn default() -> Self {
        Self {
            max_pending_commands: 2048,
            max_pending_input_bytes: 1024 * 1024,
            max_render_wakeups_per_second: 120,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SplitDirection;

    #[test]
    fn classifies_input_as_latency_sensitive_write_path() {
        let command = RuntimeCommand::WriteInput {
            pane_id: 7,
            text: "\u{1b}[C".to_string(),
        };

        assert_eq!(command.class(), RuntimeCommandClass::Input);
        assert_eq!(command.pane_id(), Some(7));
        assert!(command.is_write_path());
        assert!(command.latency_sensitive());
    }

    #[test]
    fn classifies_render_reads_as_latency_sensitive_read_path() {
        let command = RuntimeCommand::ReadRenderFrame {
            pane_id: 3,
            since_revision: Some(42),
        };

        assert_eq!(command.class(), RuntimeCommandClass::ScreenRead);
        assert_eq!(command.pane_id(), Some(3));
        assert!(!command.is_write_path());
        assert!(command.latency_sensitive());
    }

    #[test]
    fn classifies_session_lifecycle_source_pane() {
        let command = RuntimeCommand::SplitSession(SplitSessionRequest {
            source_pane_id: 9,
            direction: SplitDirection::Right,
            size_percent: 50,
            command_dir: None,
        });

        assert_eq!(command.class(), RuntimeCommandClass::SessionLifecycle);
        assert_eq!(command.pane_id(), Some(9));
        assert!(command.is_write_path());
    }

    #[test]
    fn default_queue_policy_sets_bounded_backpressure_budget() {
        let policy = RuntimeQueuePolicy::default();

        assert_eq!(policy.max_pending_commands, 2048);
        assert_eq!(policy.max_pending_input_bytes, 1024 * 1024);
        assert_eq!(policy.max_render_wakeups_per_second, 120);
    }

    #[test]
    fn reports_input_byte_cost_only_for_input_payloads() {
        let paste = RuntimeCommand::PasteInput {
            pane_id: 1,
            text: "abc".to_string(),
        };
        let read = RuntimeCommand::ReadScreen { pane_id: 1 };

        assert_eq!(paste.input_bytes(), 3);
        assert_eq!(read.input_bytes(), 0);
    }
}
