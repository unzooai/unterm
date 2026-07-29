//! A searchable list of things you can do.
//!
//! One overlay serves two jobs, because they are the same job with different
//! rows in it: the command palette lists what the terminal can do, and the
//! launcher lists what it can start. Both are "type a few letters, pick the
//! one you meant".
//!
//! The matching is subsequence-based rather than substring, because that is
//! what people expect from a palette: `nt` should find "New Tab". The scoring
//! then has to earn its keep -- a subsequence match will find almost
//! everything, so the order it puts them in *is* the feature.

/// What picking a row does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// One of the front end's key actions, by the same name an agent sees.
    Action(crate::keys::Action),
    /// Start a new tab running a named program.
    Launch { program: String },
}

/// A row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    /// Shown dimmed after the label: a key chord, a path, a description.
    pub hint: String,
    pub command: Command,
}

/// An open palette.
#[derive(Clone, Debug, Default)]
pub struct Palette {
    pub query: String,
    pub entries: Vec<Entry>,
    /// Indices into `entries`, best first.
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl Palette {
    pub fn new(entries: Vec<Entry>) -> Self {
        let mut palette = Self {
            query: String::new(),
            entries,
            matches: Vec::new(),
            selected: 0,
        };
        palette.refilter();
        palette
    }

    /// The rows to show, in order.
    pub fn visible(&self) -> Vec<&Entry> {
        self.matches.iter().filter_map(|i| self.entries.get(*i)).collect()
    }

    pub fn current(&self) -> Option<&Entry> {
        self.matches
            .get(self.selected)
            .and_then(|i| self.entries.get(*i))
    }

    /// Move the selection, wrapping.
    pub fn step(&mut self, delta: isize) {
        if self.matches.is_empty() {
            self.selected = 0;
            return;
        }
        let len = self.matches.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(len) as usize;
    }

    /// Rebuild the match list for the current query.
    pub fn refilter(&mut self) {
        let query = self.query.to_lowercase();
        let mut scored: Vec<(i32, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| score(&entry.label, &query).map(|s| (s, index)))
            .collect();
        // Best score first; ties keep the order the entries were given in, so
        // an empty query shows the list as its author arranged it.
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.matches = scored.into_iter().map(|(_, index)| index).collect();
        self.selected = 0;
    }
}

/// How well `label` matches `query`, or None if it does not.
///
/// Higher is better. The rules, in the order they matter:
///
/// - A match at the start of a word beats one in the middle, so `nt` finds
///   "New Tab" before "Print Nothing".
/// - Consecutive characters beat scattered ones, so `tab` finds "New Tab"
///   before "Toggle Alt Buffer".
/// - A shorter label beats a longer one, so an exact command beats one that
///   merely contains it.
fn score(label: &str, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let haystack: Vec<char> = label.to_lowercase().chars().collect();
    let needle: Vec<char> = query.chars().collect();

    let mut total = 0i32;
    let mut at = 0usize;
    let mut previous: Option<usize> = None;

    for wanted in needle {
        let found = haystack[at..].iter().position(|ch| *ch == wanted)? + at;

        let starts_word = found == 0
            || haystack
                .get(found.wrapping_sub(1))
                .map(|ch| !ch.is_alphanumeric())
                .unwrap_or(false);
        if starts_word {
            total += 8;
        }
        // Weighted above a word start: a run of letters the user typed
        // together is a stronger signal than the same letters scattered
        // across word boundaries. Without this, "tab" finds "Toggle Alt
        // Buffer" before "New Tab", because three word-starts outscore one
        // word-start and two neighbours.
        if previous == Some(found.wrapping_sub(1)) {
            total += 10;
        }
        previous = Some(found);
        at = found + 1;
    }

    // Shorter labels win ties: an exact command beats one that contains it.
    total -= haystack.len() as i32 / 8;
    Some(total)
}

/// What a key press means to an open palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Type(String),
    Backspace,
    Step(isize),
    Accept,
    Close,
    NotOurs,
}

/// Decide what an open palette does with a key.
///
/// It takes the whole keyboard while it is open, apart from the chords that
/// are not text -- a palette that let a keystroke through to the shell would
/// be running commands in the pane behind it.
pub fn key_for(named: Option<&str>, character: Option<&str>, ctrl: bool) -> Key {
    match named {
        Some("Escape") => return Key::Close,
        Some("Enter") => return Key::Accept,
        Some("Backspace") => return Key::Backspace,
        Some("ArrowDown") => return Key::Step(1),
        Some("ArrowUp") => return Key::Step(-1),
        _ => {}
    }
    match character {
        Some(text) if !ctrl => Key::Type(text.to_string()),
        _ => Key::NotOurs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Action;

    fn entry(label: &str) -> Entry {
        Entry {
            label: label.to_string(),
            hint: String::new(),
            command: Command::Action(Action::Copy),
        }
    }

    fn palette(labels: &[&str]) -> Palette {
        Palette::new(labels.iter().map(|label| entry(label)).collect())
    }

    #[test]
    fn an_empty_query_shows_everything_in_order() {
        let palette = palette(&["New Tab", "Close Tab", "Copy"]);
        assert_eq!(
            palette.visible().iter().map(|e| e.label.as_str()).collect::<Vec<_>>(),
            ["New Tab", "Close Tab", "Copy"]
        );
    }

    #[test]
    fn initials_find_the_command_they_stand_for() {
        let mut palette = palette(&["Print Nothing", "New Tab", "Close Tab"]);
        palette.query = "nt".to_string();
        palette.refilter();
        assert_eq!(
            palette.current().map(|e| e.label.as_str()),
            Some("New Tab"),
            "a word-start match should beat one in the middle"
        );
    }

    #[test]
    fn consecutive_letters_beat_scattered_ones() {
        let mut palette = palette(&["Toggle Alt Buffer", "New Tab"]);
        palette.query = "tab".to_string();
        palette.refilter();
        assert_eq!(palette.current().map(|e| e.label.as_str()), Some("New Tab"));
    }

    #[test]
    fn an_exact_command_beats_one_that_merely_contains_it() {
        let mut palette = palette(&["Copy Selection And Close", "Copy"]);
        palette.query = "copy".to_string();
        palette.refilter();
        assert_eq!(palette.current().map(|e| e.label.as_str()), Some("Copy"));
    }

    #[test]
    fn a_query_nothing_matches_leaves_nothing_selected() {
        let mut palette = palette(&["New Tab", "Copy"]);
        palette.query = "zzz".to_string();
        palette.refilter();
        assert!(palette.visible().is_empty());
        assert!(palette.current().is_none());
    }

    #[test]
    fn matching_ignores_case() {
        let mut palette = palette(&["New Tab"]);
        palette.query = "NEW".to_string();
        palette.refilter();
        assert_eq!(palette.visible().len(), 1);
    }

    #[test]
    fn stepping_wraps_at_both_ends() {
        let mut palette = palette(&["a", "b", "c"]);
        palette.step(-1);
        assert_eq!(palette.selected, 2);
        palette.step(1);
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn stepping_with_nothing_matched_answers_rather_than_dividing_by_zero() {
        let mut palette = palette(&[]);
        palette.step(1);
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn narrowing_the_query_puts_the_selection_back_at_the_top() {
        // The best match for the new query, not whatever index the old
        // selection happened to be -- which would land on an unrelated row.
        let mut palette = palette(&["New Tab", "Close Tab", "Copy"]);
        palette.step(2);
        palette.query = "tab".to_string();
        palette.refilter();
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn the_palette_takes_the_keyboard_while_it_is_open() {
        assert_eq!(key_for(None, Some("n"), false), Key::Type("n".to_string()));
        assert_eq!(key_for(Some("Enter"), None, false), Key::Accept);
        assert_eq!(key_for(Some("Escape"), None, false), Key::Close);
        assert_eq!(key_for(Some("ArrowDown"), None, false), Key::Step(1));
        assert_eq!(key_for(Some("ArrowUp"), None, false), Key::Step(-1));
    }

    #[test]
    fn a_control_chord_is_not_text_to_search_for() {
        assert_eq!(key_for(None, Some("c"), true), Key::NotOurs);
    }

    #[test]
    fn a_launcher_row_carries_the_program_it_starts() {
        let entry = Entry {
            label: "pwsh".to_string(),
            hint: "PowerShell".to_string(),
            command: Command::Launch {
                program: "pwsh.exe".to_string(),
            },
        };
        let palette = Palette::new(vec![entry]);
        assert_eq!(
            palette.current().map(|e| &e.command),
            Some(&Command::Launch {
                program: "pwsh.exe".to_string()
            })
        );
    }
}
