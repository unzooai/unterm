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

pub(super) fn append_bounded_output(output: &mut String, chunk: &str, max_bytes: usize) {
    output.push_str(chunk);
    trim_to_utf8_boundary(output, max_bytes);
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
        let bytes = "你".as_bytes();

        assert_eq!(decode_pty_chunk(&mut pending, &bytes[..1]), None);
        assert_eq!(decode_pty_chunk(&mut pending, &bytes[1..2]), None);
        assert_eq!(
            decode_pty_chunk(&mut pending, &bytes[2..]),
            Some("你".to_string())
        );
        assert!(pending.is_empty());
    }

    #[test]
    fn emits_valid_prefix_before_incomplete_suffix() {
        let mut pending = Vec::new();
        let mut bytes = b"ok ".to_vec();
        bytes.extend_from_slice(&"你".as_bytes()[..1]);

        assert_eq!(
            decode_pty_chunk(&mut pending, &bytes),
            Some("ok ".to_string())
        );
        assert_eq!(pending, vec!["你".as_bytes()[0]]);
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
        let mut output = String::from("ab你好");

        append_bounded_output(&mut output, "cd", 6);

        assert_eq!(output, "好cd");
        assert!(std::str::from_utf8(output.as_bytes()).is_ok());
    }
}
