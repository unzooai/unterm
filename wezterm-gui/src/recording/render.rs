//! Markdown renderer. Reads a session's `.log` file, decodes the
//! base64 payloads, strips ANSI escapes, partitions into OSC 133 blocks
//! when present, runs redaction, and emits the YAML-fronted markdown.

use super::index::IndexEntry;
use super::redact;
use anyhow::{Context, Result};
use base64::Engine as _;
use chrono::DateTime;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Debug)]
pub struct RenderConfig {
    pub redaction_enabled: bool,
    pub custom_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct RenderOutput {
    pub markdown: String,
    pub block_count: u64,
    pub redaction_count: u64,
    pub osc133_active: bool,
    pub total_lines: u64,
    pub bytes_raw: u64,
}

#[derive(Debug, Default)]
struct Block {
    /// Optional command extracted from the prompt+input region
    command: Option<String>,
    /// Output bytes, decoded
    output: Vec<u8>,
    /// Exit status (from OSC 133 D)
    exit: Option<i32>,
    /// Wall-clock start (from log timestamp of the StartPrompt)
    start_micros: Option<i64>,
    /// Wall-clock end (from CommandStatus)
    end_micros: Option<i64>,
}

/// Parse a line of the log file: `<unix_micros>\t<event_type>\t<base64_payload>`.
fn parse_log_line(line: &str) -> Option<(i64, &str, Vec<u8>)> {
    let mut parts = line.splitn(3, '\t');
    let ts: i64 = parts.next()?.parse().ok()?;
    let event = parts.next()?;
    let payload = parts.next().unwrap_or("");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.as_bytes())
        .ok()?;
    Some((ts, event, bytes))
}

/// Render a session log file to markdown given the corresponding index entry.
pub fn render_log(log_path: &Path, entry: &IndexEntry, cfg: &RenderConfig) -> Result<RenderOutput> {
    let raw = std::fs::read_to_string(log_path)
        .with_context(|| format!("read {}", log_path.display()))?;

    // Walk events, grouping by OSC 133 block boundaries.
    let mut blocks: Vec<Block> = Vec::new();
    let mut osc133_active = false;
    let mut current = Block::default();
    let mut in_prompt = false;
    let mut in_input = false;

    let mut total_bytes: u64 = 0;

    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((ts, event, payload)) = parse_log_line(line) else {
            continue;
        };
        match event {
            "out" => {
                total_bytes += payload.len() as u64;
                if !osc133_active {
                    // accumulate into a sentinel "fallback" block
                    if blocks.is_empty() {
                        blocks.push(Block::default());
                    }
                    blocks.last_mut().unwrap().output.extend_from_slice(&payload);
                } else if in_prompt {
                    // OSC 133 prompt region: discard the prompt bytes; only
                    // capture the user-typed command bytes (input region).
                } else if in_input {
                    let cmd = strip_ansi(&payload);
                    let cmd = cmd.trim_end_matches(|c: char| c == '\n' || c == '\r').to_string();
                    if !cmd.is_empty() {
                        current.command = Some(match current.command.take() {
                            Some(prev) => format!("{prev}{cmd}"),
                            None => cmd,
                        });
                    }
                } else {
                    current.output.extend_from_slice(&payload);
                }
            }
            "block_prompt" => {
                osc133_active = true;
                // New prompt → close the previous block.
                if !current.output.is_empty() || current.command.is_some() {
                    blocks.push(std::mem::take(&mut current));
                }
                current.start_micros = Some(ts);
                in_prompt = true;
                in_input = false;
            }
            "block_input" => {
                in_prompt = false;
                in_input = true;
            }
            "block_output" => {
                in_prompt = false;
                in_input = false;
            }
            "block_exit" => {
                // Payload is the decimal exit code as bytes
                let s = String::from_utf8_lossy(&payload);
                current.exit = s.trim().parse().ok();
                current.end_micros = Some(ts);
                blocks.push(std::mem::take(&mut current));
                in_prompt = false;
                in_input = false;
            }
            "in" => {
                // Input bytes — only used to fill in the command if no
                // OSC 133 input region was reported (e.g. some shells echo
                // the keystrokes as `out`).
                if osc133_active && in_input && current.command.is_none() {
                    let cmd = strip_ansi(&payload);
                    let cmd = cmd.trim_end_matches(|c: char| c == '\n' || c == '\r').to_string();
                    if !cmd.is_empty() {
                        current.command = Some(cmd);
                    }
                }
            }
            _ => {}
        }
    }

    // flush the trailing in-progress block
    if !current.output.is_empty() || current.command.is_some() || current.start_micros.is_some() {
        blocks.push(current);
    }
    // Drop completely-empty trailing blocks.
    blocks.retain(|b| !b.output.is_empty() || b.command.is_some() || b.exit.is_some());

    let total_lines = blocks
        .iter()
        .map(|b| String::from_utf8_lossy(&b.output).lines().count() as u64)
        .sum::<u64>();

    // Build markdown.
    let mut md = String::new();
    let mut redaction_count = 0u64;

    write_frontmatter(
        &mut md,
        entry,
        osc133_active,
        cfg.redaction_enabled,
        blocks.len() as u64,
        total_lines,
        total_bytes,
        // redaction_count is patched at the end of the function
        0,
    );

    let title_ts = entry
        .started_at
        .split('+')
        .next()
        .unwrap_or(&entry.started_at)
        .replace('T', " ");
    writeln!(&mut md, "# Unterm session — {}\n", title_ts).ok();

    if !osc133_active {
        writeln!(
            &mut md,
            "> {}\n",
            crate::i18n::t("recording.fallback_notice")
        )
        .ok();
        let body_text = blocks
            .iter()
            .map(|b| String::from_utf8_lossy(&b.output).into_owned())
            .collect::<Vec<_>>()
            .join("");
        let stripped = strip_ansi(body_text.as_bytes());
        let (rendered, n) = if cfg.redaction_enabled {
            redact::redact(&stripped, &cfg.custom_patterns)
        } else {
            (stripped, 0)
        };
        redaction_count += n;
        writeln!(&mut md, "```\n{}\n```", rendered.trim_end()).ok();
    } else {
        for (idx, block) in blocks.iter().enumerate() {
            let cmd = block
                .command
                .as_deref()
                .unwrap_or("(no command captured)")
                .trim();
            writeln!(&mut md, "## Block {} — `{}`\n", idx + 1, cmd).ok();
            let exit_str = match block.exit {
                Some(e) => format!("exit: {}", e),
                None => "exit: ?".to_string(),
            };
            let dur_str = match (block.start_micros, block.end_micros) {
                (Some(s), Some(e)) if e >= s => {
                    let secs = (e - s) as f64 / 1_000_000.0;
                    format!(" · {:.2}s", secs)
                }
                _ => String::new(),
            };
            let ts_str = block
                .start_micros
                .map(format_hms)
                .unwrap_or_else(|| "--:--:--".to_string());
            writeln!(&mut md, "> {}{} · {}\n", exit_str, dur_str, ts_str).ok();
            let stripped = strip_ansi(&block.output);
            let (rendered, n) = if cfg.redaction_enabled {
                redact::redact(&stripped, &cfg.custom_patterns)
            } else {
                (stripped, 0)
            };
            redaction_count += n;
            writeln!(&mut md, "```\n{}\n```\n", rendered.trim_end()).ok();
        }
    }

    // Patch the redaction_count line in frontmatter (we wrote 0 first).
    md = md.replacen(
        "redaction_count: 0",
        &format!("redaction_count: {}", redaction_count),
        1,
    );

    Ok(RenderOutput {
        markdown: md,
        block_count: blocks.len() as u64,
        redaction_count,
        osc133_active,
        total_lines,
        bytes_raw: total_bytes,
    })
}

fn write_frontmatter(
    out: &mut String,
    entry: &IndexEntry,
    osc133_active: bool,
    redaction_active: bool,
    block_count: u64,
    total_lines: u64,
    bytes_raw: u64,
    redaction_count: u64,
) {
    writeln!(out, "---").ok();
    writeln!(out, "unterm_session_id: {}", entry.unterm_session_id).ok();
    writeln!(out, "tab_id: {}", entry.tab_id).ok();
    match &entry.project_path {
        Some(p) => writeln!(out, "project_path: {}", p).ok(),
        None => writeln!(out, "project_path: null").ok(),
    };
    writeln!(out, "project_slug: {}", entry.project_slug).ok();
    writeln!(out, "shell: {}", env_var_or("SHELL", "/bin/sh")).ok();
    writeln!(
        out,
        "hostname: {}",
        hostname::get()
            .ok()
            .and_then(|s| s.into_string().ok())
            .unwrap_or_default()
    )
    .ok();
    writeln!(out, "unterm_version: {}", config::wezterm_version()).ok();
    writeln!(out, "started_at: {}", entry.started_at).ok();
    match &entry.ended_at {
        Some(e) => writeln!(out, "ended_at: {}", e).ok(),
        None => writeln!(out, "ended_at: null").ok(),
    };
    writeln!(
        out,
        "exit_reason: {}",
        entry.exit_reason.as_deref().unwrap_or("unknown")
    )
    .ok();
    writeln!(out, "osc133_active: {}", osc133_active).ok();
    writeln!(out, "block_count: {}", block_count).ok();
    writeln!(out, "total_lines: {}", total_lines).ok();
    writeln!(out, "bytes_raw: {}", bytes_raw).ok();
    if entry.trace_ids.is_empty() {
        writeln!(out, "trace_ids: []").ok();
    } else {
        let inner = entry
            .trace_ids
            .iter()
            .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "trace_ids: [{}]", inner).ok();
    }
    writeln!(out, "redaction_active: {}", redaction_active).ok();
    writeln!(out, "redaction_count: {}", redaction_count).ok();
    match &entry.parent_session_id {
        Some(p) => writeln!(out, "parent_session_id: {}", p).ok(),
        None => writeln!(out, "parent_session_id: null").ok(),
    };
    writeln!(out, "---\n").ok();
}

fn env_var_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn format_hms(micros: i64) -> String {
    let secs = micros / 1_000_000;
    if let Some(dt) = DateTime::from_timestamp(secs, 0) {
        let local = dt.with_timezone(&chrono::Local);
        local.format("%H:%M:%S").to_string()
    } else {
        "--:--:--".to_string()
    }
}

/// Strip ANSI/CSI/OSC escape sequences from a byte slice. We don't
/// depend on a dedicated crate to avoid pulling extra deps; the
/// implementation is a small state machine. Output is UTF-8 (lossy).
pub fn strip_ansi(input: &[u8]) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == 0x1b {
            // ESC. Look at the next byte to decide.
            if i + 1 >= input.len() {
                break;
            }
            match input[i + 1] {
                b'[' => {
                    // CSI: ESC [ ... <final byte 0x40-0x7e>
                    i += 2;
                    while i < input.len() && !(0x40..=0x7e).contains(&input[i]) {
                        i += 1;
                    }
                    i += 1;
                }
                b']' => {
                    // OSC: ESC ] ... (BEL or ESC \)
                    i += 2;
                    while i < input.len() {
                        if input[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                b'P' | b'X' | b'^' | b'_' => {
                    // DCS / SOS / PM / APC: terminated by ST (ESC \) or BEL
                    i += 2;
                    while i < input.len() {
                        if input[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                b'(' | b')' | b'*' | b'+' => {
                    // Charset designation: 1 more byte
                    i += 3;
                }
                _ => {
                    // Two-byte ESC (e.g. ESC =, ESC >)
                    i += 2;
                }
            }
        } else if b == b'\r' {
            // Drop bare CR; keep LF
            i += 1;
        } else if b == 0x07 {
            // BEL — drop
            i += 1;
        } else if b == 0x08 {
            // Backspace — try to remove the last char if it's printable.
            if let Some(&last) = out.last() {
                if last != b'\n' && last >= 0x20 {
                    out.pop();
                }
            }
            i += 1;
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
