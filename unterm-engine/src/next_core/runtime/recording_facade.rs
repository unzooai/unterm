use super::super::recording_lifecycle;
use crate::{
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
};
use anyhow::Result;

pub(in crate::next_core) fn start_recording(pane_id: usize) -> Result<RecordingStartResult> {
    recording_lifecycle::start(pane_id, recording_lifecycle::timestamp_string())
}

pub(in crate::next_core) fn stop_recording(pane_id: usize) -> Result<RecordingStopResult> {
    recording_lifecycle::stop(pane_id, recording_lifecycle::timestamp_string())
}

pub(in crate::next_core) fn recording_status(pane_id: usize) -> Result<RecordingStatusSnapshot> {
    recording_lifecycle::status(pane_id)
}

pub(in crate::next_core) fn attach_recording_trace(
    pane_id: usize,
    trace_id: String,
) -> Result<Vec<String>> {
    recording_lifecycle::attach_trace(pane_id, trace_id)
}

pub(in crate::next_core) fn export_recording_markdown(
    pane_id: usize,
    target_path: Option<String>,
) -> Result<RecordingExportResult> {
    recording_lifecycle::export_markdown(pane_id, target_path)
}
