use super::{
    osc133, pty_io, NextCoreActiveCommand, NextCoreCommandBlock, NextCoreRecording,
    NextCoreRecordingBlock, MAX_OUTPUT_BYTES, MAX_RECORDING_BLOCKS,
};
use base64::Engine as _;
use std::{fs::OpenOptions, io::Write};

pub(super) fn append(recording: &mut NextCoreRecording, text: &str, timestamp_micros: u128) {
    if text.is_empty() {
        return;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    let line = format!("{timestamp_micros}\tout\t{encoded}\n");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&recording.log_path)
    {
        let _ = file.write_all(line.as_bytes());
    }
    recording.bytes_raw = recording.bytes_raw.saturating_add(text.len() as u64);
    recording.block_count = recording.block_count.saturating_add(1);
    recording.blocks.push(NextCoreRecordingBlock {
        index: recording.block_count,
        timestamp_micros,
        text: text.to_string(),
    });
    record_osc133_command_blocks(recording, text, timestamp_micros);
    trim_recent_blocks(recording);
    pty_io::append_bounded_output(&mut recording.text_preview, text, MAX_OUTPUT_BYTES);
}

fn record_osc133_command_blocks(
    recording: &mut NextCoreRecording,
    text: &str,
    timestamp_micros: u128,
) {
    for item in osc133::split_stream(text) {
        match item {
            osc133::StreamItem::Text(text) => {
                if let Some(active) = recording.active_command.as_mut() {
                    active.text.push_str(text);
                }
            }
            osc133::StreamItem::Marker(marker) => {
                recording.osc133_seen = true;
                match marker.kind {
                    'C' => {
                        if let Some(active) = recording.active_command.take() {
                            recording.command_blocks.push(NextCoreCommandBlock {
                                index: active.index,
                                started_micros: active.started_micros,
                                ended_micros: None,
                                exit_code: None,
                                text: active.text,
                            });
                        }
                        let index = recording
                            .command_blocks
                            .last()
                            .map(|block| block.index.saturating_add(1))
                            .unwrap_or(1);
                        recording.active_command = Some(NextCoreActiveCommand {
                            index,
                            started_micros: timestamp_micros,
                            text: String::new(),
                        });
                    }
                    'D' => {
                        if let Some(active) = recording.active_command.take() {
                            recording.command_blocks.push(NextCoreCommandBlock {
                                index: active.index,
                                started_micros: active.started_micros,
                                ended_micros: Some(timestamp_micros),
                                exit_code: marker.exit_code,
                                text: active.text,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn trim_recent_blocks(recording: &mut NextCoreRecording) {
    if recording.blocks.len() > MAX_RECORDING_BLOCKS {
        recording
            .blocks
            .drain(..recording.blocks.len() - MAX_RECORDING_BLOCKS);
    }
    if recording.command_blocks.len() > MAX_RECORDING_BLOCKS {
        recording
            .command_blocks
            .drain(..recording.command_blocks.len() - MAX_RECORDING_BLOCKS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn append_tracks_bytes_blocks_and_preview() {
        let mut recording = sample_recording();

        append(&mut recording, "hello", 42);

        assert_eq!(recording.bytes_raw, 5);
        assert_eq!(recording.block_count, 1);
        assert_eq!(recording.blocks[0].index, 1);
        assert_eq!(recording.blocks[0].timestamp_micros, 42);
        assert_eq!(recording.blocks[0].text, "hello");
        assert_eq!(recording.text_preview, "hello");
    }

    #[test]
    fn append_extracts_osc133_command_blocks() {
        let mut recording = sample_recording();

        append(
            &mut recording,
            "\x1b]133;C\x07cargo test\x1b]133;D;0\x07",
            100,
        );

        assert!(recording.osc133_seen);
        assert!(recording.active_command.is_none());
        assert_eq!(recording.command_blocks.len(), 1);
        assert_eq!(recording.command_blocks[0].index, 1);
        assert_eq!(recording.command_blocks[0].started_micros, 100);
        assert_eq!(recording.command_blocks[0].ended_micros, Some(100));
        assert_eq!(recording.command_blocks[0].exit_code.as_deref(), Some("0"));
        assert_eq!(recording.command_blocks[0].text, "cargo test");
    }

    fn sample_recording() -> NextCoreRecording {
        NextCoreRecording {
            session_id: "session".to_string(),
            pane_id: 1,
            project_path: None,
            project_slug: "project".to_string(),
            started_at: "1".to_string(),
            log_path: PathBuf::from("target/next-core-recording-output-test.log"),
            md_path: PathBuf::from("target/next-core-recording-output-test.md"),
            bytes_raw: 0,
            block_count: 0,
            trace_ids: Vec::new(),
            text_preview: String::new(),
            blocks: Vec::new(),
            osc133_seen: false,
            command_blocks: Vec::new(),
            active_command: None,
        }
    }
}
