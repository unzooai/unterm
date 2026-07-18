//! Text shaping helpers shared by compact sidebar surfaces.
//!
//! These helpers operate in terminal display columns rather than bytes or
//! scalar values. That keeps CJK labels within their visual budget and avoids
//! cutting a combining sequence or emoji in half.

use finl_unicode::grapheme_clusters::Graphemes;
use termwiz::cell::unicode_column_width;

const ELLIPSIS: &str = "\u{2026}";

/// Preserve both ends of a label while fitting it into `max_cols` display
/// columns. This is useful for project names and paths where the beginning
/// carries the repository/root identity and the end carries the most specific
/// directory name.
///
/// The tail receives the spare column when the available width is odd. In a
/// dense project list that makes similarly-prefixed paths easier to tell apart.
pub(crate) fn ellipsize_middle(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if unicode_column_width(text, None) <= max_cols {
        return text.to_owned();
    }

    let ellipsis_width = unicode_column_width(ELLIPSIS, None);
    if max_cols <= ellipsis_width {
        return ELLIPSIS.to_owned();
    }

    let graphemes = Graphemes::new(text).collect::<Vec<_>>();
    let available = max_cols - ellipsis_width;
    let head_budget = available / 2;
    let tail_budget = available - head_budget;

    let mut head = String::new();
    let mut head_cols = 0;
    let mut head_count = 0;
    for grapheme in &graphemes {
        let width = unicode_column_width(grapheme, None);
        if head_cols + width > head_budget {
            break;
        }
        head.push_str(grapheme);
        head_cols += width;
        head_count += 1;
    }

    let mut tail = Vec::new();
    let mut tail_cols = 0;
    for (idx, grapheme) in graphemes.iter().enumerate().rev() {
        if idx < head_count {
            break;
        }
        let width = unicode_column_width(grapheme, None);
        if tail_cols + width > tail_budget {
            break;
        }
        tail.push(*grapheme);
        tail_cols += width;
    }

    let mut result = head;
    result.push_str(ELLIPSIS);
    for grapheme in tail.into_iter().rev() {
        result.push_str(grapheme);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::ellipsize_middle;
    use termwiz::cell::unicode_column_width;

    #[test]
    fn leaves_labels_that_already_fit_unchanged() {
        assert_eq!(ellipsize_middle("unterm", 6), "unterm");
        assert_eq!(ellipsize_middle("项目", 4), "项目");
    }

    #[test]
    fn keeps_the_identity_and_specific_tail() {
        assert_eq!(
            ellipsize_middle("project-terminal-render", 12),
            "proje…render"
        );
        assert_eq!(ellipsize_middle(r"D:\code\unterm", 12), r"D:\co…unterm");
    }

    #[test]
    fn measures_cjk_by_display_columns() {
        let result = ellipsize_middle("超级终端项目窗口", 9);
        assert_eq!(result, "超级…窗口");
        assert!(unicode_column_width(&result, None) <= 9);
    }

    #[test]
    fn never_splits_a_grapheme_cluster() {
        let combining = "Cafe\u{301}-project-window";
        let result = ellipsize_middle(combining, 9);
        assert!(result.starts_with("Cafe\u{301}"));
        assert!(unicode_column_width(&result, None) <= 9);
    }

    #[test]
    fn handles_tiny_and_empty_budgets() {
        assert_eq!(ellipsize_middle("long", 0), "");
        assert_eq!(ellipsize_middle("long", 1), "\u{2026}");
        assert_eq!(ellipsize_middle("", 1), "");
    }
}
