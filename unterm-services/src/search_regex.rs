//! Regex search over scrollback lines, for the front end's search bar.
//!
//! The kernel's search speaks literal patterns only -- a regex engine is a
//! heavyweight dependency, and the kernel holds a budget that says no. The
//! front end fetches the same lines the kernel would search and matches them
//! here, producing the same match shape, so the bar's three modes stay one
//! code path from where the results are used.

use unterm_engine::ScreenSearchMatch;

/// Find `pattern` in `lines`, rows numbered from `first_row`.
///
/// A pattern that does not parse matches nothing: the user sees the count
/// fall to zero while typing and keeps typing -- half-typed patterns are the
/// common case, not the exception. Matches of emptiness (`a*` everywhere)
/// are skipped for the same reason 0.57.4 skipped them: highlighting nothing
/// at every column helps nobody.
pub fn find_matches(
    lines: &[String],
    first_row: i64,
    pattern: &str,
    max_results: usize,
) -> Vec<ScreenSearchMatch> {
    if pattern.is_empty() || max_results == 0 {
        return Vec::new();
    }
    let Ok(regex) = regex::Regex::new(pattern) else {
        return Vec::new();
    };
    let mut matches = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        for found in regex.find_iter(line) {
            if found.as_str().is_empty() {
                continue;
            }
            matches.push(ScreenSearchMatch {
                row: first_row + index as i64,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| text.to_string()).collect()
    }

    #[test]
    fn regex_matches_with_character_columns() {
        let matches = find_matches(&lines(&["你x1好 x22"]), 0, r"x\d+", 10);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].col, 1, "columns count characters, not bytes");
        assert_eq!(matches[1].col, 5);
    }

    #[test]
    fn an_unfinished_regex_matches_nothing_rather_than_failing() {
        assert!(find_matches(&lines(&["abcd"]), 0, "abc(", 10).is_empty());
    }

    #[test]
    fn a_regex_that_matches_emptiness_finds_nothing_to_show() {
        assert!(find_matches(&lines(&["aaa"]), 0, "b*", 10).is_empty());
    }

    #[test]
    fn rows_carry_the_offset_they_were_read_at() {
        let matches = find_matches(&lines(&["", "hit"]), 40, "hit", 10);
        assert_eq!(matches[0].row, 41);
    }
}
