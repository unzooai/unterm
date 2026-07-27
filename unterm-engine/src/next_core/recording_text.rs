pub(super) fn yaml_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }
    let inner = values
        .iter()
        .map(|value| format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

pub(super) fn redact_text(text: &str) -> (String, u64) {
    let mut redaction_count = 0;
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut words = Vec::new();
        for word in line.split_whitespace() {
            if looks_sensitive_token(word) {
                redaction_count += 1;
                words.push("[REDACTED]");
            } else {
                words.push(word);
            }
        }
        if words.is_empty() {
            lines.push(String::new());
        } else {
            lines.push(words.join(" "));
        }
    }
    let mut rendered = lines.join("\n");
    if text.ends_with('\n') {
        rendered.push('\n');
    }
    (rendered, redaction_count)
}

fn looks_sensitive_token(word: &str) -> bool {
    let trimmed = word.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '`' | ',' | ';' | ':' | ')' | ']' | '}' | '(' | '[' | '{'
        )
    });
    let lower = trimmed.to_ascii_lowercase();
    let has_secret_key = lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("auth");
    if has_secret_key && trimmed.contains('=') {
        return true;
    }
    trimmed.len() >= 24
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '+' | '='))
}

pub(super) fn strip_ansi(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i >= bytes.len() {
                break;
            }
            match bytes[i] {
                b'[' => {
                    i += 1;
                    while i < bytes.len() {
                        let byte = bytes[i];
                        i += 1;
                        if (0x40..=0x7e).contains(&byte) {
                            break;
                        }
                    }
                }
                b']' => {
                    i += 1;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {
                    i += 1;
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secret_like_tokens_and_preserves_line_end() {
        let (text, count) =
            redact_text("hello token=super-secret-value\nplain abcdefghijklmnopqrstuvwx\n");

        assert_eq!(count, 2);
        assert_eq!(text, "hello [REDACTED]\nplain [REDACTED]\n");
    }

    #[test]
    fn strips_csi_and_osc_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi("a\x1b]0;title\x07b"), "ab");
        assert_eq!(strip_ansi("a\x1b]8;;url\x1b\\b"), "ab");
    }

    #[test]
    fn formats_yaml_string_arrays_with_escaping() {
        assert_eq!(yaml_string_array(&[]), "[]");
        assert_eq!(
            yaml_string_array(&["trace-1".to_string(), "quote\"slash\\".to_string()]),
            "[\"trace-1\", \"quote\\\"slash\\\\\"]"
        );
    }
}
