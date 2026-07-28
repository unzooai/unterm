//! Finding something in what has already scrolled past.
//!
//! The engine does the matching -- it owns the scrollback and knows where the
//! rows are. What lives here is the part a front end owns: what the bar says,
//! which match is current, and how moving between them behaves at the ends.

use unterm_engine::ScreenSearchMatch;

/// An open search.
#[derive(Clone, Debug, Default)]
pub struct Search {
    pub pattern: String,
    pub matches: Vec<ScreenSearchMatch>,
    /// Which match is current, if there are any.
    pub selected: usize,
}

impl Search {
    /// The match the viewport should be showing.
    pub fn current(&self) -> Option<&ScreenSearchMatch> {
        self.matches.get(self.selected)
    }

    /// Move to the next match, wrapping.
    ///
    /// Wrapping because a search that stops at the last match leaves the user
    /// pressing a key that does nothing, with no way to tell that from a
    /// search that has broken.
    pub fn step(&mut self, delta: isize) {
        if self.matches.is_empty() {
            self.selected = 0;
            return;
        }
        let len = self.matches.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    /// Keep the current match pointing at the same place after a re-search.
    ///
    /// Typing another character narrows the results; jumping back to the
    /// first one every keystroke drags the viewport away from what the user
    /// was already looking at.
    pub fn adopt(&mut self, matches: Vec<ScreenSearchMatch>) {
        let previous = self.current().map(|found| (found.row, found.col));
        self.selected = previous
            .and_then(|(row, col)| {
                matches
                    .iter()
                    .position(|found| found.row == row && found.col == col)
            })
            .unwrap_or(0);
        self.matches = matches;
        if self.selected >= self.matches.len() {
            self.selected = 0;
        }
    }

    /// What the bar says.
    pub fn label(&self) -> String {
        if self.pattern.is_empty() {
            return "Search: ".to_string();
        }
        if self.matches.is_empty() {
            return format!("Search: {}  (no matches)", self.pattern);
        }
        format!(
            "Search: {}  ({}/{})",
            self.pattern,
            self.selected + 1,
            self.matches.len()
        )
    }
}

/// What a key press means to an open search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// Extend the pattern, and search again.
    Type(String),
    /// Shorten the pattern, and search again.
    Backspace,
    /// Move through the results.
    Step(isize),
    /// Close the search.
    Close,
    /// Not the search's: the shell should have it.
    NotOurs,
}

/// Decide what an open search does with a key.
///
/// Separated from the window so the rules can be checked without a keyboard:
/// which keys the search takes decides which keys the shell never sees, and
/// taking one too many is how a search bar breaks a program underneath it.
pub fn key_for(
    named: Option<&str>,
    character: Option<&str>,
    ctrl: bool,
    shift: bool,
) -> Key {
    match named {
        Some("Escape") => return Key::Close,
        Some("Enter") => return Key::Step(if shift { -1 } else { 1 }),
        Some("Backspace") => return Key::Backspace,
        _ => {}
    }
    match character {
        // Ctrl+something is a command, not text to search for.
        Some(text) if !ctrl => Key::Type(text.to_string()),
        _ => Key::NotOurs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(row: i64, col: usize) -> ScreenSearchMatch {
        ScreenSearchMatch {
            row,
            col,
            text: "x".to_string(),
        }
    }

    fn search(rows: &[i64]) -> Search {
        Search {
            pattern: "x".to_string(),
            matches: rows.iter().map(|row| found(*row, 0)).collect(),
            selected: 0,
        }
    }

    #[test]
    fn stepping_past_the_last_match_wraps_to_the_first() {
        let mut s = search(&[1, 2, 3]);
        s.step(1);
        s.step(1);
        assert_eq!(s.selected, 2);
        s.step(1);
        assert_eq!(s.selected, 0, "a key that does nothing reads as broken");
    }

    #[test]
    fn stepping_back_from_the_first_wraps_to_the_last() {
        let mut s = search(&[1, 2, 3]);
        s.step(-1);
        assert_eq!(s.selected, 2);
    }

    #[test]
    fn stepping_with_no_matches_answers_rather_than_dividing_by_zero() {
        let mut s = Search::default();
        s.step(1);
        assert_eq!(s.selected, 0);
        assert!(s.current().is_none());
    }

    #[test]
    fn narrowing_the_pattern_keeps_the_match_the_user_was_looking_at() {
        let mut s = search(&[10, 20, 30]);
        s.step(1);
        assert_eq!(s.current().map(|f| f.row), Some(20));

        // The second match survives the narrower search; the others do not.
        s.adopt(vec![found(20, 0), found(40, 0)]);
        assert_eq!(
            s.current().map(|f| f.row),
            Some(20),
            "the viewport should not jump away from what was on screen"
        );
    }

    #[test]
    fn a_match_that_no_longer_exists_falls_back_to_the_first() {
        let mut s = search(&[10, 20, 30]);
        s.step(1);
        s.adopt(vec![found(50, 0), found(60, 0)]);
        assert_eq!(s.current().map(|f| f.row), Some(50));
    }

    #[test]
    fn searching_for_nothing_leaves_the_selection_somewhere_defined() {
        let mut s = search(&[10, 20]);
        s.step(1);
        s.adopt(Vec::new());
        assert_eq!(s.selected, 0);
        assert!(s.current().is_none());
    }

    #[test]
    fn the_bar_says_where_in_the_results_you_are() {
        let mut s = search(&[1, 2, 3]);
        assert_eq!(s.label(), "Search: x  (1/3)");
        s.step(1);
        assert_eq!(s.label(), "Search: x  (2/3)");
    }

    #[test]
    fn a_search_with_no_results_says_so_rather_than_showing_zero_of_zero() {
        let s = Search {
            pattern: "nope".to_string(),
            ..Default::default()
        };
        assert_eq!(s.label(), "Search: nope  (no matches)");
    }

    #[test]
    fn typing_extends_the_pattern() {
        assert_eq!(
            key_for(None, Some("a"), false, false),
            Key::Type("a".to_string())
        );
        assert_eq!(
            key_for(None, Some("中"), false, false),
            Key::Type("中".to_string())
        );
    }

    #[test]
    fn enter_steps_and_shift_enter_steps_back() {
        assert_eq!(key_for(Some("Enter"), None, false, false), Key::Step(1));
        assert_eq!(key_for(Some("Enter"), None, false, true), Key::Step(-1));
    }

    #[test]
    fn escape_closes_and_backspace_shortens() {
        assert_eq!(key_for(Some("Escape"), None, false, false), Key::Close);
        assert_eq!(key_for(Some("Backspace"), None, false, false), Key::Backspace);
    }

    #[test]
    fn a_control_chord_is_not_text_to_search_for() {
        // Ctrl+C while searching has to reach the shell, or a program that
        // is running under the search can never be stopped.
        assert_eq!(key_for(None, Some("c"), true, false), Key::NotOurs);
    }

    #[test]
    fn a_key_the_search_has_no_use_for_reaches_the_shell() {
        assert_eq!(key_for(Some("F5"), None, false, false), Key::NotOurs);
        assert_eq!(key_for(Some("ArrowUp"), None, false, false), Key::NotOurs);
    }
}
