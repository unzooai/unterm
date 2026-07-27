use super::{recording_archive, recording_markdown, session_handles, state, NextCoreRecording};
use crate::{
    RecordingExportResult, RecordingStartResult, RecordingStatusSnapshot, RecordingStopResult,
};
use anyhow::{bail, Result};
use std::{
    fs::File,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn start_current(pane_id: usize) -> Result<RecordingStartResult> {
    start(pane_id, timestamp_string())
}

pub(super) fn stop_current(pane_id: usize) -> Result<RecordingStopResult> {
    stop(pane_id, timestamp_string())
}

fn timestamp_string() -> String {
    unix_micros().to_string()
}

fn unix_micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros())
        .unwrap_or_default()
}

pub(super) fn start(pane_id: usize, started_at: String) -> Result<RecordingStartResult> {
    let handles = {
        let state = state().read();
        session_handles::recording(&state, pane_id)?
    };

    let mut slot = handles.recording.lock();
    if slot.is_some() {
        bail!("Recording already active for pane {pane_id}");
    }

    let project_slug = recording_archive::project_slug(handles.project_path.as_deref());
    let (log_path, md_path) = recording_archive::paths(
        pane_id,
        handles.project_path.as_deref(),
        &project_slug,
        &started_at,
    );
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    File::create(&log_path)?;

    let session_id = format!("next-core-{pane_id}-{started_at}");
    let recording = NextCoreRecording {
        session_id: session_id.clone(),
        pane_id,
        project_path: handles.project_path,
        project_slug,
        started_at,
        log_path: log_path.clone(),
        md_path: md_path.clone(),
        bytes_raw: 0,
        block_count: 0,
        trace_ids: Vec::new(),
        text_preview: String::new(),
        blocks: Vec::new(),
        osc133_seen: false,
        command_blocks: Vec::new(),
        active_command: None,
    };
    recording_archive::upsert_index(&recording, None)?;
    *slot = Some(recording);

    Ok(RecordingStartResult {
        session_id,
        log_path: log_path.display().to_string(),
        md_path: md_path.display().to_string(),
    })
}

pub(super) fn stop(pane_id: usize, ended_at: String) -> Result<RecordingStopResult> {
    let recording_handle = {
        let state = state().read();
        session_handles::recording(&state, pane_id)?.recording
    };
    let mut slot = recording_handle.lock();
    let Some(recording) = slot.take() else {
        bail!("No active recording for pane {pane_id}");
    };
    drop(slot);

    recording_markdown::write(&recording, Some(&ended_at), "recording_stopped")?;
    recording_archive::upsert_index(&recording, Some(ended_at.clone()))?;

    Ok(RecordingStopResult {
        session_id: recording.session_id,
        ended_at,
        block_count: recording.block_count,
        exit_reason: "recording_stopped".to_string(),
        md_path: recording.md_path.display().to_string(),
    })
}

pub(super) fn status(pane_id: usize) -> Result<RecordingStatusSnapshot> {
    let Some(recording_handle) = ({
        let state = state().read();
        session_handles::recording_optional(&state, pane_id)
    }) else {
        return Ok(inactive_status());
    };
    let slot = recording_handle.lock();
    Ok(status_snapshot(slot.as_ref()))
}

pub(super) fn attach_trace(pane_id: usize, trace_id: String) -> Result<Vec<String>> {
    let recording_handle = {
        let state = state().read();
        session_handles::recording(&state, pane_id)?.recording
    };
    let mut slot = recording_handle.lock();
    let Some(recording) = slot.as_mut() else {
        bail!("No active recording for pane {pane_id}");
    };
    if !recording
        .trace_ids
        .iter()
        .any(|existing| existing == &trace_id)
    {
        recording.trace_ids.push(trace_id);
    }
    recording_archive::upsert_index(recording, None)?;
    Ok(recording.trace_ids.clone())
}

pub(super) fn export_markdown(
    pane_id: usize,
    target_path: Option<String>,
) -> Result<RecordingExportResult> {
    let recording_handle = {
        let state = state().read();
        session_handles::recording(&state, pane_id)?.recording
    };
    let slot = recording_handle.lock();
    let Some(recording) = slot.as_ref() else {
        bail!("No active recording for pane {pane_id}");
    };
    let mut export = recording.clone();
    drop(slot);

    if let Some(target_path) = target_path {
        export.md_path = PathBuf::from(target_path);
    }
    let bytes = recording_markdown::write(&export, None, "recording_exported")?;

    Ok(RecordingExportResult {
        session_id: export.session_id,
        path: export.md_path.display().to_string(),
        bytes,
        block_count: export.block_count,
    })
}

fn status_snapshot(recording: Option<&NextCoreRecording>) -> RecordingStatusSnapshot {
    if let Some(recording) = recording {
        RecordingStatusSnapshot {
            enabled: true,
            session_id: Some(recording.session_id.clone()),
            started_at: Some(recording.started_at.clone()),
            block_count: Some(recording.block_count),
            bytes: Some(recording.bytes_raw),
        }
    } else {
        inactive_status()
    }
}

fn inactive_status() -> RecordingStatusSnapshot {
    RecordingStatusSnapshot {
        enabled: false,
        session_id: None,
        started_at: None,
        block_count: None,
        bytes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_snapshot_reports_inactive_without_recording() {
        let status = status_snapshot(None);

        assert!(!status.enabled);
        assert_eq!(status.session_id, None);
        assert_eq!(status.bytes, None);
    }

    #[test]
    fn status_snapshot_reports_active_recording_counters() {
        let recording = NextCoreRecording {
            session_id: "session-1".to_string(),
            pane_id: 7,
            project_path: Some("D:/code/unterm".to_string()),
            project_slug: "unterm".to_string(),
            started_at: "12345".to_string(),
            log_path: PathBuf::from("trace.log"),
            md_path: PathBuf::from("trace.md"),
            bytes_raw: 128,
            block_count: 3,
            trace_ids: Vec::new(),
            text_preview: String::new(),
            blocks: Vec::new(),
            osc133_seen: false,
            command_blocks: Vec::new(),
            active_command: None,
        };

        let status = status_snapshot(Some(&recording));

        assert!(status.enabled);
        assert_eq!(status.session_id.as_deref(), Some("session-1"));
        assert_eq!(status.started_at.as_deref(), Some("12345"));
        assert_eq!(status.block_count, Some(3));
        assert_eq!(status.bytes, Some(128));
    }
}
