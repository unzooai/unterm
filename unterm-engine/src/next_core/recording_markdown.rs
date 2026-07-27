use super::{recording_text, NextCoreCommandBlock, NextCoreRecording};
use anyhow::Result;
use std::fmt::Write as FmtWrite;

pub(super) fn write(
    recording: &NextCoreRecording,
    ended_at: Option<&str>,
    exit_reason: &str,
) -> Result<usize> {
    if let Some(parent) = recording.md_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let markdown = render(recording, ended_at, exit_reason);
    std::fs::write(&recording.md_path, markdown.as_bytes())?;
    Ok(markdown.len())
}

pub(super) fn render(
    recording: &NextCoreRecording,
    ended_at: Option<&str>,
    exit_reason: &str,
) -> String {
    let stripped = recording_text::strip_ansi(&recording.text_preview);
    let (redacted, redaction_count) = recording_text::redact_text(&stripped);
    let total_lines = redacted.lines().count() as u64;
    let command_blocks = command_blocks(recording);
    let mut md = String::new();

    writeln!(&mut md, "---").ok();
    writeln!(&mut md, "unterm_session_id: {}", recording.session_id).ok();
    writeln!(&mut md, "tab_id: {}", recording.pane_id).ok();
    match &recording.project_path {
        Some(path) => writeln!(&mut md, "project_path: {}", path).ok(),
        None => writeln!(&mut md, "project_path: null").ok(),
    };
    writeln!(&mut md, "project_slug: {}", recording.project_slug).ok();
    writeln!(&mut md, "shell: {}", env_var_or("SHELL", "next-core")).ok();
    writeln!(&mut md, "hostname: {}", hostname()).ok();
    writeln!(&mut md, "unterm_version: next-core").ok();
    writeln!(&mut md, "started_at: {}", recording.started_at).ok();
    match ended_at {
        Some(value) => writeln!(&mut md, "ended_at: {}", value).ok(),
        None => writeln!(&mut md, "ended_at: null").ok(),
    };
    writeln!(&mut md, "exit_reason: {}", exit_reason).ok();
    writeln!(&mut md, "osc133_active: {}", recording.osc133_seen).ok();
    writeln!(
        &mut md,
        "block_render: {}",
        if recording.osc133_seen {
            "osc133_command_blocks"
        } else {
            "chunked_output"
        }
    )
    .ok();
    writeln!(&mut md, "block_count: {}", recording.block_count).ok();
    writeln!(&mut md, "command_block_count: {}", command_blocks.len()).ok();
    writeln!(&mut md, "total_lines: {}", total_lines).ok();
    writeln!(&mut md, "bytes_raw: {}", recording.bytes_raw).ok();
    writeln!(
        &mut md,
        "trace_ids: {}",
        recording_text::yaml_string_array(&recording.trace_ids)
    )
    .ok();
    writeln!(&mut md, "redaction_active: true").ok();
    writeln!(&mut md, "redaction_count: {}", redaction_count).ok();
    writeln!(&mut md, "parent_session_id: null").ok();
    writeln!(&mut md, "---\n").ok();

    let title_ts = recording
        .started_at
        .split('+')
        .next()
        .unwrap_or(&recording.started_at)
        .replace('T', " ");
    writeln!(&mut md, "# Unterm session - {}\n", title_ts).ok();
    if recording.osc133_seen {
        writeln!(
            &mut md,
            "> next-core recording with OSC133 shell command markers.\n"
        )
        .ok();
    } else {
        writeln!(
            &mut md,
            "> next-core fallback recording; shell command markers were not captured.\n"
        )
        .ok();
    }
    if !command_blocks.is_empty() {
        writeln!(&mut md, "## Command Blocks\n").ok();
        for block in &command_blocks {
            let stripped = recording_text::strip_ansi(&block.text);
            let (redacted_block, _) = recording_text::redact_text(&stripped);
            writeln!(
                &mut md,
                "### Command {} `{}`\n\n- started: `{}`\n- ended: `{}`\n- exit_code: `{}`\n\n```\n{}\n```\n",
                block.index,
                block.started_micros,
                block.started_micros,
                block
                    .ended_micros
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "null".to_string()),
                block.exit_code.as_deref().unwrap_or("null"),
                redacted_block.trim_end()
            )
            .ok();
        }
    }
    if !recording.blocks.is_empty() {
        writeln!(
            &mut md,
            "## Output Blocks\n\nThese blocks are raw output chunks captured by next-core.\n"
        )
        .ok();
        for block in &recording.blocks {
            let stripped = recording_text::strip_ansi(&block.text);
            let (redacted_block, _) = recording_text::redact_text(&stripped);
            writeln!(
                &mut md,
                "### Block {} `{}`\n\n```\n{}\n```\n",
                block.index,
                block.timestamp_micros,
                redacted_block.trim_end()
            )
            .ok();
        }
        writeln!(&mut md, "## Aggregated Preview\n").ok();
    }
    writeln!(&mut md, "```\n{}\n```", redacted.trim_end()).ok();

    md
}

fn command_blocks(recording: &NextCoreRecording) -> Vec<NextCoreCommandBlock> {
    let mut blocks = recording.command_blocks.clone();
    if let Some(active) = recording.active_command.as_ref() {
        blocks.push(NextCoreCommandBlock {
            index: active.index,
            started_micros: active.started_micros,
            ended_micros: None,
            exit_code: None,
            text: active.text.clone(),
        });
    }
    blocks
}

fn env_var_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_recording() -> NextCoreRecording {
        NextCoreRecording {
            session_id: "session-1".to_string(),
            pane_id: 7,
            project_path: Some("C:\\work\\demo".to_string()),
            project_slug: "demo".to_string(),
            started_at: "100".to_string(),
            log_path: PathBuf::from("session.log"),
            md_path: PathBuf::from("session.md"),
            bytes_raw: 64,
            block_count: 1,
            trace_ids: vec!["trace-1".to_string()],
            text_preview: "\x1b[31mhello token=super-secret-value\x1b[0m\n".to_string(),
            blocks: vec![super::super::NextCoreRecordingBlock {
                index: 1,
                timestamp_micros: 101,
                text: "block token=block-secret\n".to_string(),
            }],
            osc133_seen: true,
            command_blocks: vec![NextCoreCommandBlock {
                index: 1,
                started_micros: 102,
                ended_micros: Some(103),
                exit_code: Some("0".to_string()),
                text: "command token=command-secret\n".to_string(),
            }],
            active_command: None,
        }
    }

    #[test]
    fn renders_redacted_command_blocks_and_output_blocks() {
        let markdown = render(&sample_recording(), Some("200"), "recording_stopped");
        assert!(markdown.contains("osc133_active: true"));
        assert!(markdown.contains("command_block_count: 1"));
        assert!(markdown.contains("trace_ids: [\"trace-1\"]"));
        assert!(markdown.contains("## Command Blocks"));
        assert!(markdown.contains("exit_code: `0`"));
        assert!(markdown.contains("command [REDACTED]"));
        assert!(markdown.contains("block [REDACTED]"));
        assert!(markdown.contains("hello [REDACTED]"));
        assert!(!markdown.contains("\x1b[31m"));
        assert!(!markdown.contains("super-secret-value"));
    }

    #[test]
    fn includes_active_command_as_open_block() {
        let mut recording = sample_recording();
        recording.command_blocks.clear();
        recording.active_command = Some(super::super::NextCoreActiveCommand {
            index: 2,
            started_micros: 222,
            text: "still running".to_string(),
        });
        let markdown = render(&recording, None, "recording_exported");
        assert!(markdown.contains("command_block_count: 1"));
        assert!(markdown.contains("### Command 2 `222`"));
        assert!(markdown.contains("- ended: `null`"));
        assert!(markdown.contains("still running"));
    }
}
