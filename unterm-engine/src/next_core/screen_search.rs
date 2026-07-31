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
        SearchMode::Regex => {
            // A pattern that does not parse matches nothing. The user sees
            // the count fall to zero while typing and keeps typing; an error
            // would have to go somewhere, and half-typed patterns are the
            // common case, not the exception.
            let Ok(regex) = regex::Regex::new(pattern) else {
                return Vec::new();
            };
            let mut matches = Vec::new();
            for (row, line) in lines.iter().enumerate() {
                for found in regex.find_iter(line) {
                    // A pattern like `a*` matches emptiness everywhere;
                    // highlighting nothing at every column helps nobody.
                    if found.as_str().is_empty() {
                        continue;
                    }
                    matches.push(ScreenSearchMatch {
                        row: row as i64,
                        col: line[..found.start()].chars().count(),
                        text: line.clone(),
                    });
                    if matches.len() >= max_results {
                        return matches;
                    }
                }
            }
            matches
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

    #[test]
    fn regex_mode_matches_patterns_with_character_columns() {
        let lines = vec!["你x1 x22".to_string()];

        let matches = find(&lines, r"x\d+", SearchMode::Regex);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].col, 1);
        assert_eq!(matches[1].col, 4);
    }

    #[test]
    fn an_unfinished_regex_matches_nothing_rather_than_failing() {
        let lines = vec!["abc(def".to_string()];

        assert!(find(&lines, "abc(", SearchMode::Regex).is_empty());
    }

    #[test]
    fn a_regex_that_matches_emptiness_finds_nothing_to_show() {
        let lines = vec!["aaa bbb".to_string()];

        let matches = find(&lines, "b*", SearchMode::Regex);

        assert_eq!(matches.len(), 1, "only the non-empty match remains");
        assert_eq!(matches[0].col, 4);
    }
}
