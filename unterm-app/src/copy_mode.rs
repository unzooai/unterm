//! Selecting text without a mouse, and grabbing what is already on screen.
//!
//! Two features that answer the same complaint. Copy mode moves a cursor with
//! the keyboard and extends a selection from it, which is the only way to copy
//! from a terminal over ssh on a laptop with no mouse. Quick select labels
//! every URL, path and hash on screen with a letter, so the common case --
//! "copy that one thing" -- takes two keystrokes instead of a careful drag.
//!
//! Both are pure decisions over a screen: where the cursor goes, what a key
//! means, which spans get labelled. The window draws the result.

use unterm_engine::StyledScreenLine;

/// Where copy mode's cursor is and what it has selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyMode {
    pub row: usize,
    pub column: usize,
    /// Where the selection started, if one is being made.
    pub anchor: Option<(usize, usize)>,
}

/// What a key does in copy mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    Top,
    Bottom,
    WordLeft,
    WordRight,
    /// Start or stop extending a selection.
    ToggleSelection,
    /// Copy what is selected and leave.
    Yank,
    Leave,
}

/// Vim's keys, plus the arrows for people who do not use them.
pub fn motion_for(named: Option<&str>, character: Option<&str>) -> Option<Motion> {
    match named {
        Some("Escape") => return Some(Motion::Leave),
        Some("ArrowLeft") => return Some(Motion::Left),
        Some("ArrowRight") => return Some(Motion::Right),
        Some("ArrowUp") => return Some(Motion::Up),
        Some("ArrowDown") => return Some(Motion::Down),
        Some("Home") => return Some(Motion::LineStart),
        Some("End") => return Some(Motion::LineEnd),
        Some("Enter") => return Some(Motion::Yank),
        _ => {}
    }
    match character? {
        "h" => Some(Motion::Left),
        "j" => Some(Motion::Down),
        "k" => Some(Motion::Up),
        "l" => Some(Motion::Right),
        "0" => Some(Motion::LineStart),
        "$" => Some(Motion::LineEnd),
        "g" => Some(Motion::Top),
        "G" => Some(Motion::Bottom),
        "b" => Some(Motion::WordLeft),
        "w" => Some(Motion::WordRight),
        "v" | " " => Some(Motion::ToggleSelection),
        "y" => Some(Motion::Yank),
        "q" => Some(Motion::Leave),
        _ => None,
    }
}

impl CopyMode {
    /// Apply a motion against a screen of `rows` lines.
    ///
    /// `line_width` gives the length of a row, so `$` lands on the last
    /// character rather than in the blank space a terminal pads rows with.
    pub fn apply(
        &mut self,
        motion: Motion,
        rows: usize,
        line_width: impl Fn(usize) -> usize,
    ) {
        let last_row = rows.saturating_sub(1);
        let width = |row: usize| line_width(row).saturating_sub(1);

        match motion {
            Motion::Left => self.column = self.column.saturating_sub(1),
            Motion::Right => self.column = (self.column + 1).min(width(self.row)),
            Motion::Up => {
                self.row = self.row.saturating_sub(1);
                self.column = self.column.min(width(self.row));
            }
            Motion::Down => {
                self.row = (self.row + 1).min(last_row);
                self.column = self.column.min(width(self.row));
            }
            Motion::LineStart => self.column = 0,
            Motion::LineEnd => self.column = width(self.row),
            Motion::Top => {
                self.row = 0;
                self.column = self.column.min(width(0));
            }
            Motion::Bottom => {
                self.row = last_row;
                self.column = self.column.min(width(last_row));
            }
            Motion::WordLeft | Motion::WordRight => {}
            Motion::ToggleSelection => {
                self.anchor = match self.anchor {
                    Some(_) => None,
                    None => Some((self.row, self.column)),
                };
            }
            Motion::Yank | Motion::Leave => {}
        }
    }

    /// The selected range, as (start, end) in (row, column), start first.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.anchor?;
        let cursor = (self.row, self.column);
        Some(if anchor <= cursor {
            (anchor, cursor)
        } else {
            (cursor, anchor)
        })
    }
}

/// A span quick select has labelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Labelled {
    pub label: String,
    pub row: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// The alphabet labels are drawn from.
///
/// Home row first, and no characters that look alike in a terminal font: an
/// `l` next to a `1` is a label you have to look twice at, which defeats the
/// point of a two-keystroke copy.
const LABEL_ALPHABET: &[u8] = b"asdfghjkweruioxcvbnm";

/// Everything on screen worth grabbing, labelled.
///
/// Nearest to the bottom first, because the thing you want is almost always
/// the thing that just scrolled past.
pub fn labelled(lines: &[StyledScreenLine]) -> Vec<Labelled> {
    let mut found: Vec<(usize, usize, usize, String)> = Vec::new();

    for (row, line) in lines.iter().enumerate() {
        let mut text = String::new();
        let mut columns: Vec<usize> = Vec::new();
        let mut column = 0usize;
        for cell in &line.cells {
            text.push(cell.ch);
            columns.push(column);
            column += cell.width.max(1);
        }
        columns.push(column);

        for (start, end) in interesting_spans(&text) {
            found.push((
                row,
                columns[start],
                columns[end],
                text.chars().skip(start).take(end - start).collect(),
            ));
        }
    }

    // Bottom-up: the thing you want is usually the thing that just appeared.
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    found
        .into_iter()
        .zip(labels())
        .map(|((row, start, end, text), label)| Labelled {
            label,
            row,
            start,
            end,
            text,
        })
        .collect()
}

/// Labels: single letters first, then pairs when those run out.
fn labels() -> impl Iterator<Item = String> {
    let singles = LABEL_ALPHABET.iter().map(|ch| (*ch as char).to_string());
    let pairs = LABEL_ALPHABET.iter().flat_map(|a| {
        LABEL_ALPHABET
            .iter()
            .map(move |b| format!("{}{}", *a as char, *b as char))
    });
    singles.chain(pairs)
}

/// Character ranges worth offering: URLs, paths, hashes, quoted strings.
fn interesting_spans(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index].is_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && !chars[index].is_whitespace() {
            index += 1;
        }
        let word: String = chars[start..index].iter().collect();
        let trimmed = word.trim_end_matches(['.', ',', ';', ':', ')', ']', '"', '\'']);
        let end = start + trimmed.chars().count();
        if end > start && is_interesting(trimmed) {
            spans.push((start, end));
        }
    }
    spans
}

/// Whether a word is the kind of thing someone copies.
///
/// Deliberately narrow. Labelling every word on screen turns the display into
/// a wall of letters and makes the useful ones harder to find than the text
/// they cover.
fn is_interesting(word: &str) -> bool {
    if word.len() < 4 {
        return false;
    }
    let looks_like_url = ["http://", "https://", "ftp://", "file://", "git@"]
        .iter()
        .any(|scheme| word.starts_with(scheme));
    let looks_like_path = word.contains('/') || (word.contains('\\') && word.contains(':'));
    // A hash: long, and hexadecimal all the way through.
    let looks_like_hash =
        word.len() >= 7 && word.chars().all(|ch| ch.is_ascii_hexdigit());
    let looks_like_ip = word.split('.').count() == 4
        && word
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()));

    looks_like_url || looks_like_path || looks_like_hash || looks_like_ip
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::{CellStyle, StyledCell};

    fn line(text: &str) -> StyledScreenLine {
        StyledScreenLine {
            row: 0,
            wrapped: false,
            cells: text
                .chars()
                .map(|ch| StyledCell {
                    ch,
                    style: CellStyle::default(),
                    width: 1,
                })
                .collect(),
        }
    }

    fn width_of(_: usize) -> usize {
        20
    }

    #[test]
    fn vim_keys_and_arrows_both_move() {
        assert_eq!(motion_for(None, Some("h")), Some(Motion::Left));
        assert_eq!(motion_for(Some("ArrowLeft"), None), Some(Motion::Left));
        assert_eq!(motion_for(None, Some("j")), Some(Motion::Down));
        assert_eq!(motion_for(Some("ArrowDown"), None), Some(Motion::Down));
    }

    #[test]
    fn a_key_copy_mode_has_no_use_for_is_ignored_rather_than_guessed() {
        assert_eq!(motion_for(None, Some("z")), None);
        assert_eq!(motion_for(Some("F5"), None), None);
    }

    #[test]
    fn the_cursor_stops_at_the_edges() {
        let mut mode = CopyMode::default();
        mode.apply(Motion::Left, 5, width_of);
        assert_eq!((mode.row, mode.column), (0, 0), "no column -1");
        mode.apply(Motion::Up, 5, width_of);
        assert_eq!(mode.row, 0, "no row -1");

        mode.apply(Motion::Bottom, 5, width_of);
        assert_eq!(mode.row, 4);
        mode.apply(Motion::Down, 5, width_of);
        assert_eq!(mode.row, 4, "no row past the last");
    }

    #[test]
    fn line_end_lands_on_the_last_character_not_the_padding() {
        let mut mode = CopyMode::default();
        mode.apply(Motion::LineEnd, 5, |_| 7);
        assert_eq!(mode.column, 6, "a 7-column line ends at column 6");
    }

    #[test]
    fn moving_to_a_shorter_line_pulls_the_column_in() {
        let mut mode = CopyMode {
            row: 0,
            column: 15,
            anchor: None,
        };
        mode.apply(Motion::Down, 3, |row| if row == 1 { 4 } else { 20 });
        assert_eq!(mode.column, 3, "the shorter line has no column 15");
    }

    #[test]
    fn a_selection_is_ordered_however_it_was_made() {
        let mut mode = CopyMode {
            row: 3,
            column: 5,
            anchor: None,
        };
        mode.apply(Motion::ToggleSelection, 10, width_of);
        mode.apply(Motion::Up, 10, width_of);
        let (start, end) = mode.selection().expect("a selection");
        assert!(start <= end, "dragging backwards still selects forwards");
        assert_eq!(end, (3, 5));
    }

    #[test]
    fn toggling_twice_clears_the_selection() {
        let mut mode = CopyMode::default();
        mode.apply(Motion::ToggleSelection, 10, width_of);
        assert!(mode.selection().is_some());
        mode.apply(Motion::ToggleSelection, 10, width_of);
        assert!(mode.selection().is_none());
    }

    #[test]
    fn quick_select_finds_urls_paths_and_hashes() {
        let found = labelled(&[line(
            "see https://example.com/x and /usr/local/bin and 8f3a2b1c9d",
        )]);
        let texts: Vec<&str> = found.iter().map(|item| item.text.as_str()).collect();
        assert!(texts.contains(&"https://example.com/x"));
        assert!(texts.contains(&"/usr/local/bin"));
        assert!(texts.contains(&"8f3a2b1c9d"));
    }

    #[test]
    fn ordinary_words_are_not_labelled() {
        // Labelling every word turns the screen into a wall of letters.
        let found = labelled(&[line("the quick brown fox jumped over")]);
        assert!(found.is_empty(), "got {found:?}");
    }

    #[test]
    fn trailing_punctuation_is_left_out_of_the_span() {
        let found = labelled(&[line("go to https://example.com/page.")]);
        assert_eq!(found[0].text, "https://example.com/page");
    }

    #[test]
    fn the_nearest_match_gets_the_first_label() {
        // The thing you want is usually the thing that just scrolled past.
        let found = labelled(&[line("/first/path"), line("/second/path")]);
        assert_eq!(found[0].text, "/second/path");
        assert_eq!(found[0].label, "a");
    }

    #[test]
    fn labels_never_run_out() {
        let lines: Vec<_> = (0..40).map(|i| line(&format!("/path/number/{i}"))).collect();
        let found = labelled(&lines);
        assert_eq!(found.len(), 40);
        let mut seen = std::collections::HashSet::new();
        for item in &found {
            assert!(seen.insert(item.label.clone()), "duplicate label {}", item.label);
        }
    }

    #[test]
    fn a_wide_character_shifts_the_columns_a_label_covers() {
        let mut line = line("中 /usr/bin");
        line.cells[0].width = 2;
        let found = labelled(&[line]);
        assert_eq!(found[0].start, 3, "two columns for the character, one space");
    }
}
