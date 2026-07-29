//! The bar along the top: wordmark, tabs, actions, window buttons.
//!
//! One dark strip in the window's own colour, in place of the grey native
//! title bar with a separate tab strip under it. Three stacked bands of
//! different colours read as three windows; one band reads as a window.
//!
//! Laid out here, away from the event loop, because the interesting part is
//! what gets dropped as the window narrows and because a button that is drawn
//! in one place and clicked in another is a bug nobody can see in a
//! screenshot. Drawing and hit-testing both come from this one list.

/// How tall the bar is, in cells. Two, so it reads as chrome rather than as a
/// row of output that failed to scroll.
pub const ROWS: usize = 2;

/// Where the terminal area starts, in pixels from the top of the window.
///
/// The bar is always there, unlike the strip it replaces: it carries the
/// window buttons, and a window whose close button appears only once there
/// are two tabs is not a window.
pub fn terminal_top(metrics: unterm_render::quads::CellMetrics) -> f32 {
    metrics.height * ROWS as f32
}

/// What is left for the terminal once both bars have taken their rows.
///
/// Taken *out of* the terminal rather than drawn over it: a bar over the grid
/// hides a row the shell still believes in, and the cursor ends up somewhere
/// nobody can see.
pub fn terminal_height(
    window_height: f32,
    metrics: unterm_render::quads::CellMetrics,
) -> f32 {
    let taken = ROWS + crate::statusbar::ROWS;
    (window_height - metrics.height * taken as f32).max(metrics.height)
}

/// What a piece of the bar is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// The product's name. Not clickable; it is there so the window says what
    /// it is when it has no title bar to do it.
    Wordmark,
    Tab(usize),
    NewTab,
    /// One of the front-end actions the bar offers directly.
    Action(crate::keys::Action),
    /// The quick-action menu.
    Menu,
    Minimise,
    Maximise,
    Close,
}

/// A piece of the bar, placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub item: Item,
    pub column: usize,
    pub columns: usize,
    pub label: String,
}

impl Placed {
    fn new(item: Item, column: usize, label: &str) -> Self {
        Self {
            item,
            column,
            columns: width_of(label),
            label: label.to_string(),
        }
    }

    pub fn contains(&self, column: usize) -> bool {
        column >= self.column && column < self.column + self.columns
    }
}

/// The name in the corner.
const WORDMARK: &str = " Unterm ";
/// The window buttons, in the order Windows puts them.
const MINIMISE: &str = " \u{2500} ";
const MAXIMISE: &str = " \u{25A1} ";
const CLOSE: &str = " \u{2715} ";
/// The actions worth a button of their own: the ones reached constantly.
const NEW_TAB: &str = " + ";
const MENU: &str = " \u{25BE} ";

/// The action buttons, most useful first -- which is also the order they
/// survive in as the window narrows.
const ACTIONS: &[(crate::keys::Action, &str)] = &[
    (crate::keys::Action::CommandPalette, " \u{2318} "),
    (crate::keys::Action::SplitRight, " \u{2337} "),
    (crate::keys::Action::CockpitInbox, " \u{25C9} "),
];

/// Lay the bar out for a window `columns` wide.
///
/// The window buttons come first in the arithmetic and last in the list: they
/// are the one thing that must never be dropped or moved, because a window
/// with no title bar has no other way to be closed.
pub fn layout(tabs: usize, _active: usize, columns: usize) -> Vec<Placed> {
    let mut placed = Vec::new();
    if columns == 0 {
        return placed;
    }

    // Right-hand side, from the edge inwards.
    let buttons = [
        (Item::Close, CLOSE),
        (Item::Maximise, MAXIMISE),
        (Item::Minimise, MINIMISE),
    ];
    let mut right = columns;
    let mut window_buttons = Vec::new();
    for (item, label) in buttons {
        let wide = width_of(label);
        if right < wide {
            break;
        }
        right -= wide;
        window_buttons.push(Placed::new(item, right, label));
    }

    // Then the menu and the action buttons, while there is room to spare.
    //
    // "Room to spare" means room left over after the left-hand side has what
    // it needs: a bar of buttons with no tabs in it is a toolbar, and the tabs
    // are what the bar is for.
    let left_minimum = width_of(WORDMARK) + MIN_TAB + width_of(NEW_TAB);
    let mut chrome = Vec::new();
    let take = |label: &str, item: Item, right: &mut usize, chrome: &mut Vec<Placed>| {
        let wide = width_of(label);
        if right.saturating_sub(wide) < left_minimum {
            return;
        }
        *right -= wide;
        chrome.push(Placed::new(item, *right, label));
    };
    take(MENU, Item::Menu, &mut right, &mut chrome);
    for (action, label) in ACTIONS {
        take(label, Item::Action(*action), &mut right, &mut chrome);
    }

    // Left-hand side: the wordmark, then the tabs in the space that is left.
    // Everything here is bounded by `right`, which is where the chrome starts
    // -- without that the wordmark is drawn underneath the window buttons on a
    // narrow window, and a click lands on whichever was drawn last.
    let mut left = 0;
    if right >= width_of(WORDMARK) + MIN_TAB {
        placed.push(Placed::new(Item::Wordmark, 0, WORDMARK));
        left = width_of(WORDMARK);
    }

    let plus = width_of(NEW_TAB);
    let room = right.saturating_sub(left);
    let tab_room = room.saturating_sub(plus);
    if tabs > 0 && tab_room >= MIN_TAB {
        let each = (tab_room / tabs).clamp(MIN_TAB, MAX_TAB);
        let shown = (tab_room / each).min(tabs).max(1);
        for index in 0..shown {
            let label = tab_label(index, each);
            placed.push(Placed {
                item: Item::Tab(index),
                column: left + index * each,
                columns: each,
                label,
            });
        }
        left += shown * each;
    }
    if left + plus <= right {
        placed.push(Placed::new(Item::NewTab, left, NEW_TAB));
    }

    placed.extend(chrome);
    placed.extend(window_buttons);
    placed
}

/// Narrower than this and a tab cannot say which one it is.
const MIN_TAB: usize = 4;
/// Wider than this and two tabs look like a toolbar.
const MAX_TAB: usize = 24;

/// What a tab says: its number, and room after it for an agent's badge.
///
/// Which tab is active is shown by its background rather than by a mark in the
/// label. That leaves a glyph of room for the badge -- the thing that actually
/// needs to be seen from across a window.
fn tab_label(index: usize, width: usize) -> String {
    let text = format!(" {}", index + 1);
    let padding = width.saturating_sub(width_of(&text));
    format!("{text}{}", " ".repeat(padding))
}

/// Where a tab's agent badge goes: the column after its number.
pub fn badge_column(tab: &Placed) -> usize {
    tab.column + tab.label.trim_end().chars().count() + 1
}

/// The window button this piece is, if it is one.
pub fn window_button(item: Item) -> Option<crate::window_buttons::Button> {
    match item {
        Item::Minimise => Some(crate::window_buttons::Button::Minimise),
        Item::Maximise => Some(crate::window_buttons::Button::Maximise),
        Item::Close => Some(crate::window_buttons::Button::Close),
        _ => None,
    }
}

/// Which piece a click at `column` landed on.
pub fn hit(placed: &[Placed], column: usize) -> Option<Item> {
    placed
        .iter()
        .find(|piece| piece.contains(column))
        .map(|piece| piece.item)
}

/// Whether a click here should drag the window.
///
/// Everywhere the bar has nothing in it. A title bar you cannot drag the
/// window by is the thing people notice first about a window with no title
/// bar -- and the wordmark counts as empty for this, because it is the
/// obvious place to grab.
pub fn is_drag_handle(placed: &[Placed], column: usize) -> bool {
    match hit(placed, column) {
        None => true,
        Some(Item::Wordmark) => true,
        Some(_) => false,
    }
}

fn width_of(text: &str) -> usize {
    text.chars()
        .map(|ch| {
            let mut buffer = [0u8; 4];
            termwiz::cell::unicode_column_width(ch.encode_utf8(&mut buffer), None).max(1)
        })
        .sum()
}

/// Which tab a number key means, counting from one as the keys are labelled.
///
/// Nine is the last tab however many there are, which is what every browser
/// does -- someone with four tabs pressing Ctrl+9 means the last one, not
/// nothing. Other numbers past the end are nothing rather than the last one:
/// Ctrl+3 with two tabs open is a miss, and jumping somewhere unasked-for is
/// worse than staying put.
pub fn tab_for_number(number: u8, count: usize) -> Option<usize> {
    if count == 0 || number == 0 {
        return None;
    }
    if number >= 9 {
        return Some(count - 1);
    }
    let index = number as usize - 1;
    (index < count).then_some(index)
}

#[cfg(test)]
mod number_key_tests {
    use super::*;

    #[test]
    fn a_number_picks_the_tab_with_that_position() {
        assert_eq!(tab_for_number(1, 4), Some(0));
        assert_eq!(tab_for_number(3, 4), Some(2));
    }

    /// Nine is the last one, however many there are.
    #[test]
    fn nine_is_the_last_tab_not_the_ninth() {
        assert_eq!(tab_for_number(9, 4), Some(3));
        assert_eq!(tab_for_number(9, 1), Some(0));
        assert_eq!(tab_for_number(9, 12), Some(11));
    }

    /// And a number past the end is a miss, not a jump to the end -- those
    /// two rules only look inconsistent until you press Ctrl+3 by accident.
    #[test]
    fn a_number_past_the_last_tab_does_nothing() {
        assert_eq!(tab_for_number(3, 2), None);
        assert_eq!(tab_for_number(8, 2), None);
    }

    #[test]
    fn there_is_no_tab_when_there_are_no_tabs() {
        assert_eq!(tab_for_number(1, 0), None);
        assert_eq!(tab_for_number(9, 0), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(placed: &[Placed]) -> Vec<Item> {
        placed.iter().map(|piece| piece.item).collect()
    }

    #[test]
    fn a_wide_window_gets_everything() {
        let bar = layout(3, 0, 160);
        let items = items(&bar);
        assert!(items.contains(&Item::Wordmark));
        assert!(items.contains(&Item::Tab(0)));
        assert!(items.contains(&Item::Tab(2)));
        assert!(items.contains(&Item::NewTab));
        assert!(items.contains(&Item::Menu));
        assert!(items.contains(&Item::Close));
    }

    /// A window with no title bar has no other way to be closed. The window
    /// buttons are the one thing that survives every width.
    #[test]
    fn the_window_buttons_are_never_dropped() {
        for columns in 10..200 {
            let bar = layout(3, 0, columns);
            let items = items(&bar);
            assert!(items.contains(&Item::Close), "{columns} columns");
            assert!(items.contains(&Item::Minimise), "{columns} columns");
        }
    }

    /// And they stay flush to the right edge, where they are on Windows.
    #[test]
    fn the_close_button_is_the_rightmost_thing() {
        for columns in [40, 80, 160] {
            let bar = layout(2, 0, columns);
            let close = bar.iter().find(|p| p.item == Item::Close).unwrap();
            assert_eq!(close.column + close.columns, columns, "{columns} columns");
            for piece in &bar {
                assert!(
                    piece.column + piece.columns <= columns,
                    "{piece:?} runs off a {columns}-column bar"
                );
            }
        }
    }

    /// Nothing may overlap: two pieces sharing a column means a click does one
    /// thing and the drawing shows another.
    #[test]
    fn no_two_pieces_share_a_column() {
        for columns in 10..200 {
            let bar = layout(4, 1, columns);
            for column in 0..columns {
                let hits = bar.iter().filter(|p| p.contains(column)).count();
                assert!(hits <= 1, "{columns} columns: {hits} pieces at {column}");
            }
        }
    }

    /// Action buttons go before tabs do. A bar with buttons and no tabs is a
    /// toolbar; the tabs are what the bar is for.
    #[test]
    fn a_button_never_crowds_out_a_tab() {
        for columns in 10..200 {
            let bar = layout(2, 0, columns);
            let items = items(&bar);
            let has_button = items.iter().any(|item| matches!(item, Item::Action(_)));
            let has_tab = items.iter().any(|item| matches!(item, Item::Tab(_)));
            assert!(
                !has_button || has_tab,
                "{columns} columns: buttons but no tabs: {bar:?}"
            );
        }
    }

    /// And below some width the buttons are gone entirely, or they would be
    /// taking the room the terminal is for.
    #[test]
    fn a_narrow_window_has_no_action_buttons_at_all() {
        let narrow = layout(2, 0, 24);
        assert!(
            !items(&narrow).iter().any(|item| matches!(item, Item::Action(_))),
            "{narrow:?}"
        );
    }

    /// Tabs are numbered, and nothing in the label says which is active --
    /// that is the background's job, which leaves room for the agent badge.
    #[test]
    fn tabs_are_numbered_from_one() {
        let bar = layout(3, 1, 120);
        for index in 0..3 {
            let tab = bar.iter().find(|p| p.item == Item::Tab(index)).unwrap();
            assert!(tab.label.contains(&(index + 1).to_string()), "{:?}", tab.label);
        }
    }

    /// The badge sits after the number, inside the tab it belongs to. Drawn
    /// past the tab's own width it lands on the next tab, which points at the
    /// wrong pane.
    #[test]
    fn a_badge_goes_beside_its_tabs_number() {
        let bar = layout(3, 0, 120);
        for index in 0..3 {
            let tab = bar.iter().find(|p| p.item == Item::Tab(index)).unwrap();
            let badge = badge_column(tab);
            assert!(badge > tab.column, "{tab:?}");
            assert!(badge < tab.column + tab.columns, "{tab:?} badge at {badge}");
        }
    }

    /// Every tab is the same width, or the bar reflows as tabs are marked and
    /// unmarked and the whole thing twitches on every focus change.
    #[test]
    fn tabs_are_all_the_same_width() {
        let bar = layout(5, 2, 160);
        let widths: Vec<usize> = bar
            .iter()
            .filter(|p| matches!(p.item, Item::Tab(_)))
            .map(|p| p.columns)
            .collect();
        assert!(widths.windows(2).all(|pair| pair[0] == pair[1]), "{widths:?}");
    }

    /// A click is answered by the same list that drew the bar, so the two
    /// cannot disagree.
    #[test]
    fn a_click_finds_what_was_drawn_there() {
        let bar = layout(3, 0, 120);
        for piece in &bar {
            assert_eq!(
                hit(&bar, piece.column),
                Some(piece.item),
                "{piece:?} is not clickable where it was drawn"
            );
            let last = piece.column + piece.columns - 1;
            assert_eq!(hit(&bar, last), Some(piece.item), "{piece:?} right edge");
        }
    }

    /// The empty parts drag the window. Without this a window with no title
    /// bar cannot be moved, which is the first thing anyone tries.
    #[test]
    fn the_empty_parts_of_the_bar_drag_the_window() {
        let bar = layout(1, 0, 120);
        let close = bar.iter().find(|p| p.item == Item::Close).unwrap();
        assert!(!is_drag_handle(&bar, close.column), "a button is not a handle");

        let wordmark = bar.iter().find(|p| p.item == Item::Wordmark).unwrap();
        assert!(
            is_drag_handle(&bar, wordmark.column),
            "the wordmark is the obvious place to grab"
        );

        let empty = (0..120).find(|column| hit(&bar, *column).is_none());
        if let Some(empty) = empty {
            assert!(is_drag_handle(&bar, empty));
        }
    }

    /// A window too narrow for anything must still not panic, and must still
    /// have a way to be closed.
    #[test]
    fn a_tiny_window_still_has_a_close_button() {
        let bar = layout(1, 0, 3);
        assert_eq!(items(&bar), vec![Item::Close]);
        assert!(layout(1, 0, 0).is_empty());
    }

    /// One tab still gets a bar: unlike the old strip, this one carries the
    /// window buttons, so it cannot come and go with the tab count.
    #[test]
    fn one_tab_still_gets_a_bar() {
        let bar = layout(1, 0, 120);
        assert!(items(&bar).contains(&Item::Tab(0)));
        assert!(items(&bar).contains(&Item::Close));
    }
}
