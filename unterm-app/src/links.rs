//! Links a program marked, and links a program merely printed.
//!
//! Two different things, and both are expected to work. A program can mark
//! text as a link explicitly with OSC 8 -- `ls --hyperlink`, `gcc`'s
//! diagnostics, anything that knows it is writing to a terminal. Far more
//! often it just prints a URL, and the user still expects to be able to click
//! it.
//!
//! Opening is deliberately behind a modifier. A terminal where a stray click
//! launches a browser is a terminal you cannot select text in, and a program
//! that prints a URL should not be able to make one click open it.

use unterm_engine::StyledScreenLine;

/// Where a link is on screen and what it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Row within the viewport.
    pub row: usize,
    /// Half-open column range.
    pub start: usize,
    pub end: usize,
    pub uri: String,
}

impl Link {
    pub fn covers(&self, row: usize, column: usize) -> bool {
        self.row == row && column >= self.start && column < self.end
    }
}

/// Characters that end a bare URL.
///
/// Trailing punctuation is excluded separately: a URL at the end of a
/// sentence should not swallow the full stop.
fn is_url_char(ch: char) -> bool {
    !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '<' | '>' | '`' | '(' | ')' | '[' | ']')
}

/// Punctuation a sentence is likely to have put after a URL rather than in it.
fn trailing_noise(ch: char) -> bool {
    matches!(ch, '.' | ',' | ';' | ':' | '!' | '?')
}

const SCHEMES: &[&str] = &["https://", "http://", "file://", "ftp://", "mailto:"];

/// Every link on a row: the ones a program marked, and the ones it printed.
///
/// Marked links win where they overlap. A program that took the trouble to
/// say what a span points at knows better than a pattern does -- the text can
/// be a title while the link goes somewhere else entirely.
pub fn links_in_row(row: usize, line: &StyledScreenLine) -> Vec<Link> {
    let mut links = marked_links(row, line);
    let taken: Vec<(usize, usize)> = links.iter().map(|link| (link.start, link.end)).collect();

    for found in printed_links(row, line) {
        let overlaps = taken
            .iter()
            .any(|(start, end)| found.start < *end && found.end > *start);
        if !overlaps {
            links.push(found);
        }
    }
    links.sort_by_key(|link| link.start);
    links
}

/// Links a program marked with OSC 8.
fn marked_links(row: usize, line: &StyledScreenLine) -> Vec<Link> {
    let mut links: Vec<Link> = Vec::new();
    let mut column = 0usize;
    for cell in &line.cells {
        let width = cell.width.max(1);
        if let Some(uri) = cell.style.hyperlink.as_deref() {
            match links.last_mut() {
                // The same link continuing: one span, not one per cell.
                Some(last) if last.end == column && last.uri == uri => last.end = column + width,
                _ => links.push(Link {
                    row,
                    start: column,
                    end: column + width,
                    uri: uri.to_string(),
                }),
            }
        }
        column += width;
    }
    links
}

/// URLs a program merely printed.
fn printed_links(row: usize, line: &StyledScreenLine) -> Vec<Link> {
    // Column per character, so a match in the text maps back to the grid --
    // a wide character occupies two columns and one `char`.
    let mut text = String::new();
    let mut columns: Vec<usize> = Vec::new();
    let mut column = 0usize;
    for cell in &line.cells {
        text.push(cell.ch);
        columns.push(column);
        column += cell.width.max(1);
    }
    columns.push(column);

    let chars: Vec<char> = text.chars().collect();
    let mut links = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        let rest: String = chars[index..].iter().collect();
        let Some(scheme) = SCHEMES.iter().find(|scheme| rest.starts_with(**scheme)) else {
            index += 1;
            continue;
        };
        let mut end = index + scheme.chars().count();
        while end < chars.len() && is_url_char(chars[end]) {
            end += 1;
        }
        while end > index && trailing_noise(chars[end - 1]) {
            end -= 1;
        }
        // A scheme and nothing after it is not a link.
        if end > index + scheme.chars().count() {
            links.push(Link {
                row,
                start: columns[index],
                end: columns[end],
                uri: chars[index..end].iter().collect(),
            });
        }
        index = end.max(index + 1);
    }
    links
}

/// Whether a click should open a link.
///
/// Behind Ctrl, because a terminal where a stray click launches a browser is
/// a terminal you cannot select text in.
pub fn opens_on_click(ctrl: bool) -> bool {
    ctrl
}

/// Hand a URI to the desktop.
///
/// Only schemes we recognise: a cell can claim any string as its link, and
/// handing an arbitrary one to the shell is how a printed line becomes a
/// command someone else chose.
pub fn open(uri: &str) -> std::io::Result<()> {
    if !SCHEMES.iter().any(|scheme| uri.starts_with(scheme)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("refusing to open a link with an unrecognised scheme: {uri}"),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        // `start` needs an empty title first, or it treats the URL as one.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", uri])
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(uri).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(uri)
            .spawn()
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_engine::{CellStyle, StyledCell};

    fn line(text: &str, link: Option<&str>) -> StyledScreenLine {
        let mut style = CellStyle::default();
        style.hyperlink = link.map(String::from);
        StyledScreenLine {
            row: 0,
            wrapped: false,
            cells: text
                .chars()
                .map(|ch| StyledCell {
                    ch,
                    style: style.clone(),
                    width: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn a_printed_url_is_a_link() {
        let links = links_in_row(0, &line("see https://example.com now", None));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "https://example.com");
        assert_eq!(links[0].start, 4);
    }

    #[test]
    fn a_url_at_the_end_of_a_sentence_does_not_swallow_the_full_stop() {
        let links = links_in_row(0, &line("go to https://example.com.", None));
        assert_eq!(links[0].uri, "https://example.com");
    }

    #[test]
    fn a_url_in_brackets_stops_at_the_bracket() {
        let links = links_in_row(0, &line("(https://example.com)", None));
        assert_eq!(links[0].uri, "https://example.com");
    }

    #[test]
    fn a_bare_scheme_is_not_a_link() {
        assert!(links_in_row(0, &line("https://", None)).is_empty());
    }

    #[test]
    fn several_urls_on_a_row_are_all_found() {
        let links = links_in_row(0, &line("http://a.com and http://b.com", None));
        assert_eq!(links.len(), 2);
        assert_eq!(links[1].uri, "http://b.com");
    }

    #[test]
    fn a_marked_link_spans_its_cells_as_one() {
        let links = links_in_row(0, &line("click", Some("https://example.com")));
        assert_eq!(links.len(), 1);
        assert_eq!((links[0].start, links[0].end), (0, 5));
        assert_eq!(links[0].uri, "https://example.com");
    }

    #[test]
    fn a_marked_link_wins_over_the_text_that_looks_like_one() {
        // The text can be a title while the link goes somewhere else, and the
        // program that said so knows better than a pattern does.
        let line = line("https://decoy.example", Some("https://real.example"));
        let links = links_in_row(0, &line);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].uri, "https://real.example");
    }

    #[test]
    fn a_click_only_opens_with_the_modifier() {
        assert!(!opens_on_click(false), "a stray click must not open a browser");
        assert!(opens_on_click(true));
    }

    #[test]
    fn an_unrecognised_scheme_is_refused_rather_than_handed_to_the_shell() {
        // A cell can claim any string as its link.
        for uri in [
            "javascript:alert(1)",
            "vbscript:x",
            "data:text/html,<script>",
            "\\\\evil\\share",
            "C:\\Windows\\System32\\calc.exe",
        ] {
            assert!(
                open(uri).is_err(),
                "{uri} should not reach the desktop opener"
            );
        }
    }

    #[test]
    fn covering_a_column_is_half_open_at_the_end() {
        let link = Link {
            row: 2,
            start: 4,
            end: 8,
            uri: "https://example.com".to_string(),
        };
        assert!(link.covers(2, 4));
        assert!(link.covers(2, 7));
        assert!(!link.covers(2, 8), "the end column is past the link");
        assert!(!link.covers(3, 5), "another row is another link");
    }

    #[test]
    fn a_wide_character_before_a_url_shifts_its_columns() {
        let mut line = line("中 https://example.com", None);
        line.cells[0].width = 2;
        let links = links_in_row(0, &line);
        assert_eq!(
            links[0].start, 3,
            "two columns for the character, one for the space"
        );
    }
}
