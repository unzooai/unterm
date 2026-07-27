use super::super::recording_lifecycle;
use super::command::RuntimeCommand;
use crate::{
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
};
use anyhow::{bail, Result};

pub(in crate::next_core) fn execute_start(command: RuntimeCommand) -> Result<RecordingStartResult> {
    match command {
        RuntimeCommand::StartRecording { pane_id } => {
            recording_lifecycle::start(pane_id, recording_lifecycle::timestamp_string())
        }
        _ => bail!("runtime recording executor expected start-recording command"),
    }
}

pub(in crate::next_core) fn execute_stop(command: RuntimeCommand) -> Result<RecordingStopResult> {
    match command {
        RuntimeCommand::StopRecording { pane_id } => {
            recording_lifecycle::stop(pane_id, recording_lifecycle::timestamp_string())
        }
        _ => bail!("runtime recording executor expected stop-recording command"),
    }
}

pub(in crate::next_core) fn execute_status(
    command: RuntimeCommand,
) -> Result<RecordingStatusSnapshot> {
    match command {
        RuntimeCommand::RecordingStatus { pane_id } => recording_lifecycle::status(pane_id),
        _ => bail!("runtime recording executor expected recording-status command"),
    }
}

pub(in crate::next_core) fn execute_attach_trace(command: RuntimeCommand) -> Result<Vec<String>> {
    match command {
        RuntimeCommand::AttachRecordingTrace { pane_id, trace_id } => {
            recording_lifecycle::attach_trace(pane_id, trace_id)
        }
        _ => bail!("runtime recording executor expected attach-recording-trace command"),
    }
}

pub(in crate::next_core) fn execute_export_markdown(
    command: RuntimeCommand,
) -> Result<RecordingExportResult> {
    match command {
        RuntimeCommand::ExportRecordingMarkdown {
            pane_id,
            target_path,
        } => recording_lifecycle::export_markdown(pane_id, target_path),
        _ => bail!("runtime recording executor expected export-recording-markdown command"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_executor_rejects_wrong_command_shape() {
        let err = execute_status(RuntimeCommand::HealthSnapshot)
            .expect_err("wrong recording command should fail");

        assert!(err
            .to_string()
            .contains("expected recording-status command"));
    }

    #[test]
    fn status_reports_inactive_for_missing_session() {
        let status =
            execute_status(RuntimeCommand::RecordingStatus { pane_id: 404 }).expect("status");

        assert!(!status.enabled);
    }
}
