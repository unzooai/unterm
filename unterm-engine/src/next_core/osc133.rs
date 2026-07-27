#[derive(Clone, Debug)]
pub(super) struct Marker {
    pub(super) kind: char,
    pub(super) exit_code: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) enum StreamItem<'a> {
    Text(&'a str),
    Marker(Marker),
}

pub(super) fn split_stream(text: &str) -> Vec<StreamItem<'_>> {
    let bytes = text.as_bytes();
    let mut items = Vec::new();
    let mut last_text = 0usize;
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] != 0x1b || bytes[i + 1] != b']' {
            i += 1;
            continue;
        }

        let content_start = i + 2;
        let Some((content_end, next)) = terminator(bytes, content_start) else {
            break;
        };
        let content = &text[content_start..content_end];
        let Some(marker) = parse_marker(content) else {
            i = next;
            continue;
        };

        if last_text < i {
            items.push(StreamItem::Text(&text[last_text..i]));
        }
        items.push(StreamItem::Marker(marker));
        last_text = next;
        i = next;
    }
    if last_text < text.len() {
        items.push(StreamItem::Text(&text[last_text..]));
    }
    items
}

fn terminator(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == 0x07 {
            return Some((i, i + 1));
        }
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
            return Some((i, i + 2));
        }
        i += 1;
    }
    None
}

fn parse_marker(content: &str) -> Option<Marker> {
    let rest = content.strip_prefix("133;")?;
    let mut parts = rest.split(';');
    let kind = parts.next()?.chars().next()?;
    if !matches!(kind, 'A' | 'B' | 'C' | 'D') {
        return None;
    }
    let exit_code = if kind == 'D' {
        parts
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    } else {
        None
    };
    Some(Marker { kind, exit_code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_bel_terminated_command_markers() {
        let items = split_stream("before\x1b]133;C\x07body\x1b]133;D;0\x07after");
        assert_eq!(items.len(), 5);
        assert!(matches!(items[0], StreamItem::Text("before")));
        assert!(matches!(items[1], StreamItem::Marker(ref marker) if marker.kind == 'C'));
        assert!(matches!(items[2], StreamItem::Text("body")));
        assert!(
            matches!(items[3], StreamItem::Marker(ref marker) if marker.kind == 'D' && marker.exit_code.as_deref() == Some("0"))
        );
        assert!(matches!(items[4], StreamItem::Text("after")));
    }

    #[test]
    fn splits_st_terminated_command_markers() {
        let items = split_stream("a\x1b]133;C\x1b\\b\x1b]133;D;1\x1b\\");
        assert_eq!(items.len(), 4);
        assert!(matches!(items[1], StreamItem::Marker(ref marker) if marker.kind == 'C'));
        assert!(
            matches!(items[3], StreamItem::Marker(ref marker) if marker.kind == 'D' && marker.exit_code.as_deref() == Some("1"))
        );
    }

    #[test]
    fn ignores_non_osc133_sequences_and_unterminated_markers() {
        let items = split_stream("a\x1b]0;title\x07b\x1b]133;C");
        assert_eq!(items.len(), 1);
        assert!(matches!(
            items[0],
            StreamItem::Text("a\x1b]0;title\x07b\x1b]133;C")
        ));
    }
}
