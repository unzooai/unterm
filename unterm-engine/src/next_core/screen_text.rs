pub(super) fn output_lines(output: &str) -> Vec<String> {
    output
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

pub(super) fn tail_lines(lines: &[String], limit: usize) -> Vec<String> {
    let start = lines.len().saturating_sub(limit);
    lines[start..].to_vec()
}

pub(super) fn bounded_range(
    line_count: usize,
    start_line: Option<i64>,
    end_line: Option<i64>,
    tail_lines: Option<i64>,
) -> (usize, usize) {
    let end = end_line
        .map(|end| end.max(0) as usize)
        .unwrap_or(line_count)
        .min(line_count);
    let mut start = start_line
        .map(|start| start.max(0) as usize)
        .unwrap_or(0)
        .min(end);
    if let Some(tail) = tail_lines {
        if tail > 0 {
            start = start.max(end.saturating_sub(tail as usize));
        }
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_lines_normalizes_crlf_and_trims_right_edge() {
        assert_eq!(
            output_lines("one\r\ntwo  \rthree\n"),
            vec!["one", "two", "three"]
        );
    }

    #[test]
    fn tail_lines_returns_recent_lines() {
        let lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];

        assert_eq!(tail_lines(&lines, 2), vec!["two", "three"]);
        assert_eq!(tail_lines(&lines, 10), lines);
    }

    #[test]
    fn bounded_range_clamps_and_applies_tail() {
        assert_eq!(bounded_range(100, Some(10), Some(90), Some(5)), (85, 90));
        assert_eq!(bounded_range(100, Some(-10), Some(200), None), (0, 100));
        assert_eq!(bounded_range(100, Some(80), Some(20), None), (20, 20));
        assert_eq!(bounded_range(100, None, Some(20), Some(0)), (0, 20));
    }
}
