use crate::{ScreenSearchMatch, SearchMode};

pub(super) fn find_matches(
    lines: &[String],
    pattern: &str,
    mode: SearchMode,
    max_results: usize,
) -> Vec<ScreenSearchMatch> {
    if pattern.is_empty() || max_results == 0 {
        return Vec::new();
    }

    match mode {
        SearchMode::CaseSensitive => literal_matches(lines, pattern, max_results),
        SearchMode::CaseInsensitive => {
            // Both sides lowered, so the match positions land in the lowered
            // line and the columns are counted there too. Lowering can change
            // a line's length, but only for characters where any answer is a
            // judgement call.
            let pattern = pattern.to_lowercase();
            let lines: Vec<String> = lines.iter().map(|line| line.to_lowercase()).collect();
            literal_matches(&lines, &pattern, max_results)
        }
    }
}

fn literal_matches(lines: &[String], pattern: &str, max_results: usize) -> Vec<ScreenSearchMatch> {
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

    fn find(lines: &[String], pattern: &str, mode: SearchMode) -> Vec<ScreenSearchMatch> {
        find_matches(lines, pattern, mode, 10)
    }

    #[test]
    fn returns_character_columns_for_multibyte_text() {
        let lines = vec!["a你b你".to_string()];

        let matches = find(&lines, "你", SearchMode::CaseSensitive);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].row, 0);
        assert_eq!(matches[0].col, 1);
        assert_eq!(matches[1].col, 3);
    }

    #[test]
    fn respects_max_results() {
        let lines = vec!["aaa".to_string(), "aaa".to_string()];

        let matches = find_matches(&lines, "a", SearchMode::CaseSensitive, 4);

        assert_eq!(matches.len(), 4);
        assert_eq!(matches[3].row, 1);
        assert_eq!(matches[3].col, 0);
    }

    #[test]
    fn empty_pattern_or_zero_limit_returns_no_matches() {
        let lines = vec!["abc".to_string()];

        assert!(find(&lines, "", SearchMode::CaseSensitive).is_empty());
        assert!(find_matches(&lines, "a", SearchMode::CaseSensitive, 0).is_empty());
    }

    #[test]
    fn case_sensitive_mode_takes_case_at_its_word() {
        let lines = vec!["Error error".to_string()];

        let matches = find(&lines, "Error", SearchMode::CaseSensitive);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].col, 0);
    }

    #[test]
    fn case_insensitive_mode_matches_either_case() {
        let lines = vec!["Error error".to_string()];

        let matches = find(&lines, "eRRor", SearchMode::CaseInsensitive);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].col, 0);
        assert_eq!(matches[1].col, 6);
    }

}
