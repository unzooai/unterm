#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OscCommand {
    Title(String),
    CurrentDir(String),
    Hyperlink(Option<String>),
}

pub(super) fn parse(sequence: &str) -> Option<OscCommand> {
    let (kind, value) = sequence.split_once(';')?;
    match kind {
        "0" | "2" if !value.is_empty() => Some(OscCommand::Title(value.to_string())),
        "7" => parse_osc7_cwd(value).map(OscCommand::CurrentDir),
        "8" => parse_osc8_hyperlink(value).map(OscCommand::Hyperlink),
        _ => None,
    }
}

fn parse_osc8_hyperlink(value: &str) -> Option<Option<String>> {
    let (_params, uri) = value.split_once(';')?;
    if uri.is_empty() {
        Some(None)
    } else {
        Some(Some(uri.to_string()))
    }
}

fn parse_osc7_cwd(value: &str) -> Option<String> {
    let uri = value.strip_prefix("file://")?;
    let path = if uri.starts_with('/') {
        uri
    } else {
        let slash = uri.find('/')?;
        &uri[slash..]
    };
    let decoded = percent_decode(path)?;
    if decoded.is_empty() {
        return None;
    }
    #[cfg(windows)]
    {
        let mut path = decoded;
        let bytes = path.as_bytes();
        if path.starts_with('/')
            && bytes.len() >= 4
            && bytes[2] == b':'
            && bytes[1].is_ascii_alphabetic()
        {
            path.remove(0);
        }
        Some(path.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        Some(decoded)
    }
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%' {
            let hi = *bytes.get(idx + 1)?;
            let lo = *bytes.get(idx + 2)?;
            out.push(hex_value(hi)? << 4 | hex_value(lo)?);
            idx += 3;
        } else {
            out.push(bytes[idx]);
            idx += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_updates() {
        assert_eq!(
            parse("0;Codex Pane"),
            Some(OscCommand::Title("Codex Pane".to_string()))
        );
        assert_eq!(
            parse("2;Tab Title"),
            Some(OscCommand::Title("Tab Title".to_string()))
        );
        assert_eq!(parse("2;"), None);
    }

    #[test]
    fn parses_osc8_hyperlink_start_and_end() {
        assert_eq!(
            parse("8;id=1;https://example.com"),
            Some(OscCommand::Hyperlink(Some(
                "https://example.com".to_string()
            )))
        );
        assert_eq!(parse("8;;"), Some(OscCommand::Hyperlink(None)));
    }

    #[test]
    fn rejects_invalid_percent_encoded_cwd() {
        assert_eq!(parse("7;file://localhost/tmp/%zz"), None);
    }

    #[test]
    fn parses_osc7_cwd_with_host() {
        let parsed = parse("7;file://localhost/tmp/my%20project");
        #[cfg(windows)]
        assert_eq!(
            parsed,
            Some(OscCommand::CurrentDir("\\tmp\\my project".to_string()))
        );
        #[cfg(not(windows))]
        assert_eq!(
            parsed,
            Some(OscCommand::CurrentDir("/tmp/my project".to_string()))
        );
    }
}
