use crate::ScreenSearchMatch;

pub(super) fn find_matches(
    lines: &[String],
    pattern: &str,
    max_results: usize,
) -> Vec<ScreenSearchMatch> {
    if pattern.is_empty() || max_results == 0 {
        return Vec::new();
    }

    let mut matches = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        for (byte_col, _) in line.match_indices(pattern) {
            matches.push(ScreenSearchMatch {
                row: row as i64,
                col: line[..byte_col].chars().count(),
                text: line.clone(),
            });
            if matches.len() >= max_results {
                return matches;
            }
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_character_columns_for_multibyte_text() {
        let lines = vec!["a你b你".to_string()];

        let matches = find_matches(&lines, "你", 10);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].row, 0);
        assert_eq!(matches[0].col, 1);
        assert_eq!(matches[1].col, 3);
    }

    #[test]
    fn respects_max_results() {
        let lines = vec!["aaa".to_string(), "aaa".to_string()];

        let matches = find_matches(&lines, "a", 4);

        assert_eq!(matches.len(), 4);
        assert_eq!(matches[3].row, 1);
        assert_eq!(matches[3].col, 0);
    }

    #[test]
    fn empty_pattern_or_zero_limit_returns_no_matches() {
        let lines = vec!["abc".to_string()];

        assert!(find_matches(&lines, "", 10).is_empty());
        assert!(find_matches(&lines, "a", 0).is_empty());
    }
}
