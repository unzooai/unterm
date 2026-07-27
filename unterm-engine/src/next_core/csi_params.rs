pub(super) const SGR_UNDERLINE_STYLE_BASE: usize = 10_000;

pub(super) fn parse_sgr(raw_params: &str) -> Vec<usize> {
    let raw_params = raw_params.trim_start_matches('?');
    if raw_params.is_empty() {
        return vec![0];
    }

    let mut params = Vec::new();
    for part in raw_params.split(';') {
        let part = part.trim();
        if part.is_empty() {
            params.push(0);
        } else if part.starts_with("38:") || part.starts_with("48:") || part.starts_with("58:") {
            params.extend(parse_colon_color_sgr(part));
        } else if let Some(underline) = parse_colon_underline_sgr(part) {
            params.push(underline);
        } else if let Some((first, _)) = part.split_once(':') {
            params.push(first.trim().parse::<usize>().unwrap_or(0));
        } else {
            params.push(part.parse::<usize>().unwrap_or(0));
        }
    }

    if params.is_empty() {
        vec![0]
    } else {
        params
    }
}

pub(super) fn parse_numbers(raw_params: &str) -> Vec<usize> {
    let raw_params = raw_params
        .trim_start_matches('?')
        .trim_end_matches(|c: char| !c.is_ascii_digit() && c != ';' && c != ':');
    if raw_params.is_empty() {
        return Vec::new();
    }
    raw_params
        .split(';')
        .map(|part| part.trim().parse::<usize>().unwrap_or(0))
        .collect()
}

pub(super) fn rect_from_numbers(
    numbers: &[usize],
    rows: usize,
    cols: usize,
) -> (usize, usize, usize, usize) {
    let top = numbers.first().copied().filter(|n| *n > 0).unwrap_or(1);
    let left = numbers.get(1).copied().filter(|n| *n > 0).unwrap_or(1);
    let bottom = numbers.get(2).copied().filter(|n| *n > 0).unwrap_or(rows);
    let right = numbers.get(3).copied().filter(|n| *n > 0).unwrap_or(cols);
    (
        top.saturating_sub(1),
        left.saturating_sub(1),
        bottom.saturating_sub(1),
        right.saturating_sub(1),
    )
}

fn parse_colon_color_sgr(part: &str) -> Vec<usize> {
    let mut pieces = part.split(':');
    let target = pieces
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(0);
    let mode = pieces
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(0);

    match mode {
        5 => pieces
            .find_map(|part| part.parse::<usize>().ok())
            .map(|color| vec![target, 5, color])
            .unwrap_or_else(|| vec![target]),
        2 => {
            let values = pieces
                .filter_map(|part| part.parse::<usize>().ok())
                .collect::<Vec<_>>();
            if values.len() >= 3 {
                let start = values.len().saturating_sub(3);
                vec![
                    target,
                    2,
                    values[start],
                    values[start + 1],
                    values[start + 2],
                ]
            } else {
                vec![target]
            }
        }
        _ => vec![target],
    }
}

fn parse_colon_underline_sgr(part: &str) -> Option<usize> {
    let (prefix, value) = part.split_once(':')?;
    if prefix.trim() != "4" {
        return None;
    }
    let value = value
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .parse::<usize>()
        .ok()?;
    Some(SGR_UNDERLINE_STYLE_BASE + value.min(5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_sgr_as_reset() {
        assert_eq!(parse_sgr(""), vec![0]);
        assert_eq!(parse_sgr(";1"), vec![0, 1]);
    }

    #[test]
    fn parses_colon_extended_color_sgr() {
        assert_eq!(parse_sgr("38:5:123"), vec![38, 5, 123]);
        assert_eq!(parse_sgr("48:2::1:2:3"), vec![48, 2, 1, 2, 3]);
        assert_eq!(parse_sgr("58:2:0:4:5:6"), vec![58, 2, 4, 5, 6]);
    }

    #[test]
    fn parses_colon_underline_sgr() {
        assert_eq!(parse_sgr("4:3"), vec![SGR_UNDERLINE_STYLE_BASE + 3]);
        assert_eq!(parse_sgr("4:99"), vec![SGR_UNDERLINE_STYLE_BASE + 5]);
    }

    #[test]
    fn parses_csi_numbers_with_private_prefix_and_suffix() {
        assert_eq!(parse_numbers("?1;2;3$r"), vec![1, 2, 3]);
        assert_eq!(parse_numbers(" 4;bad;6 "), vec![4, 0, 6]);
    }

    #[test]
    fn resolves_rect_defaults_to_screen_size() {
        assert_eq!(rect_from_numbers(&[2, 3, 4, 5], 24, 80), (1, 2, 3, 4));
        assert_eq!(rect_from_numbers(&[0, 0], 24, 80), (0, 0, 23, 79));
    }
}
