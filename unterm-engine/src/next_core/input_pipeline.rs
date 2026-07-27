pub(super) const PASTE_CHUNK_BYTES: usize = 4096;

#[allow(dead_code)]
pub(super) fn paste_payload(text: &str, bracketed: bool) -> String {
    if bracketed {
        format!("\x1b[200~{text}\x1b[201~")
    } else {
        text.to_string()
    }
}

pub(super) fn application_cursor_input(input: &str, enabled: bool) -> String {
    if !enabled {
        return input.to_string();
    }

    input
        .replace("\x1b[A", "\x1bOA")
        .replace("\x1b[B", "\x1bOB")
        .replace("\x1b[C", "\x1bOC")
        .replace("\x1b[D", "\x1bOD")
        .replace("\x1b[H", "\x1bOH")
        .replace("\x1b[F", "\x1bOF")
        .replace("\x1b[1~", "\x1bOH")
        .replace("\x1b[4~", "\x1bOF")
}

fn split_utf8_chunks(text: &str, max_bytes: usize) -> Vec<&str> {
    if text.is_empty() {
        return vec![""];
    }
    let max_bytes = max_bytes.max(1);
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut last_boundary = 0;
    for (idx, _) in text.char_indices().skip(1) {
        if idx - start > max_bytes {
            let end = last_boundary.max(start);
            if end == start {
                continue;
            }
            chunks.push(&text[start..end]);
            start = end;
        }
        last_boundary = idx;
    }
    chunks.push(&text[start..]);
    chunks
}

pub(super) fn paste_chunks(text: &str, bracketed: bool) -> Vec<String> {
    let text_chunks = split_utf8_chunks(text, PASTE_CHUNK_BYTES);
    if !bracketed {
        return text_chunks.into_iter().map(str::to_string).collect();
    }

    let mut chunks = Vec::with_capacity(text_chunks.len() + 2);
    chunks.push("\x1b[200~".to_string());
    chunks.extend(text_chunks.into_iter().map(str::to_string));
    chunks.push("\x1b[201~".to_string());
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_payload_wraps_bracketed_text() {
        assert_eq!(paste_payload("plain", false), "plain");
        assert_eq!(
            paste_payload("line1\nline2", true),
            "\x1b[200~line1\nline2\x1b[201~"
        );
    }

    #[test]
    fn paste_chunks_do_not_split_utf8() {
        let text = format!("{}{}", "a".repeat(PASTE_CHUNK_BYTES), "你".repeat(3));
        let chunks = paste_chunks(&text, false);
        assert_eq!(chunks.concat(), text);
        assert!(chunks.len() > 1);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.is_char_boundary(chunk.len())));
    }

    #[test]
    fn bracketed_paste_chunks_keep_markers_intact() {
        let text = "x".repeat(PASTE_CHUNK_BYTES + 10);
        let chunks = paste_chunks(&text, true);
        assert_eq!(chunks.first().map(String::as_str), Some("\x1b[200~"));
        assert_eq!(chunks.last().map(String::as_str), Some("\x1b[201~"));
        assert_eq!(chunks[1..chunks.len() - 1].concat(), text);
    }

    #[test]
    fn application_cursor_input_translates_navigation_keys() {
        assert_eq!(
            application_cursor_input("\x1b[A\x1b[B\x1b[C\x1b[D", true),
            "\x1bOA\x1bOB\x1bOC\x1bOD"
        );
        assert_eq!(
            application_cursor_input("\x1b[H\x1b[F\x1b[1~\x1b[4~", true),
            "\x1bOH\x1bOF\x1bOH\x1bOF"
        );
        assert_eq!(application_cursor_input("x\x1b[C你", true), "x\x1bOC你");
        assert_eq!(application_cursor_input("\x1b[C", false), "\x1b[C");
    }
}
