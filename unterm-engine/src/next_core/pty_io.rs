pub(super) fn decode_pty_chunk(pending: &mut Vec<u8>, bytes: &[u8]) -> Option<String> {
    pending.extend_from_slice(bytes);
    match std::str::from_utf8(pending.as_slice()) {
        Ok(text) => {
            let text = text.to_string();
            pending.clear();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        Err(err) if err.error_len().is_none() => {
            let valid_up_to = err.valid_up_to();
            if valid_up_to == 0 {
                return None;
            }
            let text = String::from_utf8(pending[..valid_up_to].to_vec()).ok()?;
            pending.drain(..valid_up_to);
            Some(text)
        }
        Err(_) => {
            let text = String::from_utf8_lossy(pending.as_slice()).to_string();
            pending.clear();
            Some(text)
        }
    }
}

#[derive(Default)]
pub(super) struct StartupOutputFilter {
    buffer: String,
    done: bool,
    suppressing_powershell_warning: bool,
}

impl StartupOutputFilter {
    pub(super) fn filter(&mut self, chunk: String) -> Option<String> {
        if self.done {
            return non_empty(chunk);
        }

        self.buffer.push_str(&chunk);
        if self.suppressing_powershell_warning {
            if let Some(prompt) = first_powershell_prompt(&self.buffer) {
                self.buffer.clear();
                self.done = true;
                return Some(format!("\x1b[H{prompt}"));
            }
            if self.buffer.len() <= 8192 {
                return None;
            }
            self.done = true;
            return non_empty(strip_powershell_startup_noise(&std::mem::take(&mut self.buffer)));
        }
        if startup_buffer_is_blank(&self.buffer) {
            return None;
        }
        if startup_chunk_is_control_only(&self.buffer) {
            return non_empty(std::mem::take(&mut self.buffer));
        }
        if let Some(filtered) = remove_powershell_startup_warning(&self.buffer) {
            self.suppressing_powershell_warning = true;
            if let Some(prompt) = first_powershell_prompt(&filtered) {
                self.buffer.clear();
                self.done = true;
                return Some(format!("\x1b[H{prompt}"));
            }
            self.buffer.clear();
            return None;
        }

        if may_be_split_powershell_warning(&self.buffer) {
            return None;
        }

        self.done = true;
        non_empty(trim_blank_prefix_before_prompt(std::mem::take(&mut self.buffer)))
    }
}

fn non_empty(text: String) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

const POWERSHELL_SCREEN_READER_WARNINGS: &[&str] = &[
    "WARNING: PowerShell detected that you might be using a screen reader and has disabled PSReadLine for compatibility purposes. If you want to re-enable it, run 'Import-Module PSReadLine'.",
];

fn remove_powershell_startup_warning(text: &str) -> Option<String> {
    for warning in POWERSHELL_SCREEN_READER_WARNINGS {
        if let Some(start) = text.find(warning) {
            let mut end = start + warning.len();
            if text[end..].starts_with("\r\n") {
                end += 2;
            } else if text[end..].starts_with('\n') {
                end += 1;
            }
            let mut filtered = text.to_string();
            filtered.replace_range(start..end, "");
            return Some(trim_startup_blank_prefix(filtered));
        }
    }
    for marker in ["Import-Module PSReadLine", "ort-Module PSReadLine"] {
        if let Some(marker_start) = text.find(marker) {
            let line_start = text[..marker_start]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let line_end = text[marker_start..]
                .find('\n')
                .map(|idx| marker_start + idx + 1)
                .unwrap_or(text.len());
            let mut filtered = text.to_string();
            filtered.replace_range(line_start..line_end, "");
            return Some(trim_startup_blank_prefix(filtered));
        }
    }
    None
}

fn trim_startup_blank_prefix(mut text: String) -> String {
    while text.starts_with("\r\n") {
        text.drain(..2);
    }
    while text.starts_with('\n') || text.starts_with('\r') {
        text.drain(..1);
    }
    text
}

fn startup_buffer_is_blank(text: &str) -> bool {
    text.len() <= 64 && text.chars().all(|ch| ch == '\r' || ch == '\n')
}

fn startup_chunk_is_control_only(text: &str) -> bool {
    text.len() <= 4096
        && text.starts_with('\x1b')
        && !text.contains("PS ")
        && !text.contains("PSReadLine")
}

fn trim_blank_prefix_before_prompt(text: String) -> String {
    let trimmed = trim_startup_blank_prefix(text.clone());
    if trimmed.starts_with("PS ") {
        trimmed
    } else {
        text
    }
}

fn first_powershell_prompt(text: &str) -> Option<String> {
    let start = text.find("PS ")?;
    let rest = &text[start..];
    let mut end = rest.find('>')? + 1;
    if rest[end..].starts_with(' ') {
        end += 1;
    }
    Some(rest[..end].to_string())
}

fn may_be_split_powershell_warning(text: &str) -> bool {
    const MAX_STARTUP_WARNING_BYTES: usize = 512;
    if text.len() > MAX_STARTUP_WARNING_BYTES {
        return false;
    }
    POWERSHELL_SCREEN_READER_WARNINGS
        .iter()
        .any(|warning| warning_prefix_is_at_end(text, warning))
}

fn warning_prefix_is_at_end(text: &str, warning: &str) -> bool {
    warning
        .char_indices()
        .skip(1)
        .any(|(idx, _)| text.ends_with(&warning[..idx]))
}

pub(super) fn append_bounded_output(output: &mut String, chunk: &str, max_bytes: usize) {
    output.push_str(chunk);
    trim_to_utf8_boundary(output, max_bytes);
}

pub(super) fn strip_powershell_startup_noise(text: &str) -> String {
    remove_powershell_startup_warning(text).unwrap_or_else(|| text.to_string())
}

pub(super) fn trim_to_utf8_boundary(text: &mut String, max_bytes: usize) {
    if text.len() <= max_bytes {
        return;
    }
    let keep_from = text.len() - max_bytes;
    let keep_from = text
        .char_indices()
        .map(|(idx, _)| idx)
        .find(|idx| *idx >= keep_from)
        .unwrap_or(0);
    text.drain(..keep_from);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_incomplete_utf8_sequence() {
        let mut pending = Vec::new();
        let bytes = "\u{4f60}".as_bytes();

        assert_eq!(decode_pty_chunk(&mut pending, &bytes[..1]), None);
        assert_eq!(decode_pty_chunk(&mut pending, &bytes[1..2]), None);
        assert_eq!(
            decode_pty_chunk(&mut pending, &bytes[2..]),
            Some("\u{4f60}".to_string())
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn emits_valid_prefix_before_incomplete_suffix() {
        let mut pending = Vec::new();
        let mut bytes = b"ok ".to_vec();
        bytes.extend_from_slice(&"\u{4f60}".as_bytes()[..1]);

        assert_eq!(
            decode_pty_chunk(&mut pending, &bytes),
            Some("ok ".to_string())
        );
        assert_eq!(pending, vec!["\u{4f60}".as_bytes()[0]]);
    }

    #[test]
    fn replaces_invalid_utf8_and_clears_pending() {
        let mut pending = Vec::new();

        assert_eq!(
            decode_pty_chunk(&mut pending, &[0xff, b'a']),
            Some("\u{fffd}a".to_string())
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn bounded_output_preserves_recent_text() {
        let mut output = String::from("abcdef");

        append_bounded_output(&mut output, "gh", 4);

        assert_eq!(output, "efgh");
    }

    #[test]
    fn bounded_output_does_not_split_utf8() {
        let mut output = String::from("ab\u{4f60}\u{597d}");

        append_bounded_output(&mut output, "cd", 6);

        assert_eq!(output, "\u{597d}cd");
        assert!(std::str::from_utf8(output.as_bytes()).is_ok());
    }

    #[test]
    fn startup_filter_removes_split_powershell_warning() {
        let mut filter = StartupOutputFilter::default();

        assert_eq!(
            filter.filter("WARNING: PowerShell detected that you might be using ".to_string()),
            None
        );
        let output = filter
            .filter("a screen reader and has disabled PSReadLine for compatibility purposes. If you want to re-enable it, run 'Import-Module PSReadLine'.\r\nPS C:\\Users\\Alex> ".to_string())
            .unwrap();

        assert_eq!(output, "\u{1b}[HPS C:\\Users\\Alex> ");
    }

    #[test]
    fn startup_filter_removes_split_warning_tail() {
        let mut filter = StartupOutputFilter::default();

        let output = filter
            .filter("ort-Module PSReadLine\"銆俓r\nPS C:\\Users\\Alex> ".to_string())
            .unwrap();

        assert_eq!(output, "\u{1b}[HPS C:\\Users\\Alex> ");
    }

    #[test]
    fn strips_powershell_warning_tail_from_any_chunk() {
        assert_eq!(
            strip_powershell_startup_noise("ort-Module PSReadLine\"銆俓r\nPS C:\\Users\\Alex> "),
            "PS C:\\Users\\Alex> "
        );
    }

    #[test]
    fn startup_filter_drops_blank_lines_left_by_warning_removal() {
        assert_eq!(
            strip_powershell_startup_noise(
                "\r\n\r\n\r\nort-Module PSReadLine\"\r\nPS C:\\Users\\Alex> "
            ),
            "PS C:\\Users\\Alex> "
        );
    }

    #[test]
    fn startup_filter_waits_on_initial_blank_lines_before_prompt() {
        let mut filter = StartupOutputFilter::default();

        assert_eq!(filter.filter("\r\n\r\n\r\n".to_string()), None);
        assert_eq!(
            filter.filter("PS C:\\Users\\Alex> ".to_string()).unwrap(),
            "PS C:\\Users\\Alex> "
        );
    }

    #[test]
    fn startup_filter_passes_normal_output() {
        let mut filter = StartupOutputFilter::default();

        assert_eq!(filter.filter("PS C:\\Users\\Alex> ".to_string()).unwrap(), "PS C:\\Users\\Alex> ");
        assert_eq!(filter.filter("echo ok\r\n".to_string()).unwrap(), "echo ok\r\n");
    }
}
