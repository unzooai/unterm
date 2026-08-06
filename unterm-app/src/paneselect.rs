//! Pick a pane by typing one letter.
//!
//! Four panes open and the one you want is the bottom right: reaching it means
//! either the mouse or three presses of a direction key. This puts a letter on
//! each pane and takes you there in one.
//!
//! The same letters the copy hints use, deliberately. Two label alphabets in
//! one product is two things to learn, and the reason that alphabet was chosen
//! -- home row first, nothing that looks like anything else in a terminal font
//! -- applies here for exactly the same reason.
//!
//! Swapping is here too, because it is the same gesture. Choosing where a pane
//! should go is choosing a pane; what happens next is the only difference, and
//! it is carried from the key that opened the selector rather than asked at
//! the end.

/// What choosing a pane does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Go there.
    Activate,
    /// Exchange it with the pane in front, and follow it.
    Swap,
}

/// An open selector.
#[derive(Clone, Debug)]
pub struct Selector {
    /// One label per pane, in the order the panes are laid out.
    pub labels: Vec<String>,
    /// What has been typed so far.
    pub typing: String,
    pub mode: Mode,
}

/// What a key press did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A label was completed: this pane, by its index in `labels`.
    Chose(usize),
    /// Still typing, or a correction.
    Typing,
    Cancelled,
    /// Not ours -- but nothing reaches the shell while this is open.
    Ignored,
}

impl Selector {
    /// A selector for `count` panes.
    pub fn new(count: usize, mode: Mode) -> Self {
        Self {
            labels: crate::copy_mode::labels_for(count),
            typing: String::new(),
            mode,
        }
    }

    /// Feed it a key.
    ///
    /// `character` is what the key produced, `named` the name of a key that
    /// produced nothing. Shift is not consulted: the alphabet is lower case,
    /// and a capital letter typed by accident should still pick the pane.
    pub fn key(&mut self, named: Option<&str>, character: Option<&str>, ctrl: bool) -> Outcome {
        if let Some(named) = named {
            return match named {
                "Escape" => Outcome::Cancelled,
                "Backspace" => {
                    self.typing.pop();
                    Outcome::Typing
                }
                _ => Outcome::Ignored,
            };
        }
        let Some(character) = character else {
            return Outcome::Ignored;
        };
        if ctrl {
            // The two chords the previous selector answered: `g` to give up
            // and `u` to start the label again.
            return match character.to_lowercase().as_str() {
                "g" => Outcome::Cancelled,
                "u" => {
                    self.typing.clear();
                    Outcome::Typing
                }
                _ => Outcome::Ignored,
            };
        }

        let typed = character.to_lowercase();
        if typed.chars().count() != 1 || !typed.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return Outcome::Ignored;
        }
        self.typing.push_str(&typed);

        if let Some(index) = self.labels.iter().position(|label| *label == self.typing) {
            return Outcome::Chose(index);
        }
        // Nothing can complete from here, so start again rather than making
        // the next letter useless: a label that cannot be reached is a
        // selector that has stopped responding.
        if !self
            .labels
            .iter()
            .any(|label| label.starts_with(&self.typing))
        {
            self.typing.clear();
        }
        Outcome::Typing
    }
}

/// The panes, in the order the labels go on them.
///
/// Reading order -- top to bottom, then left to right within a row -- because
/// that is the order the eye assigns letters to things it is looking at, and a
/// label list that jumps around makes the letters feel arbitrary.
pub fn reading_order(panes: &[crate::panes::PanePlacement]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..panes.len()).collect();
    order.sort_by(|a, b| {
        let (first, second) = (&panes[*a], &panes[*b]);
        first
            .origin
            .1
            .total_cmp(&second.origin.1)
            .then(first.origin.0.total_cmp(&second.origin.0))
    });
    order
}

// Named for what it tests rather than `tests`, because cargo's test filter is
// a plain substring: `select::tests::` would otherwise also select this
// module's tests, and the suite that counts them would count these too.
#[cfg(test)]
mod picker_tests {
    use super::*;

    fn placement(left: f32, top: f32) -> crate::panes::PanePlacement {
        crate::panes::PanePlacement {
            session_id: (left as usize) * 100 + top as usize,
            origin: (left, top),
            cols: 10,
            rows: 10,
        }
    }

    /// The same letters as the copy hints. Two label alphabets in one product
    /// is two things to learn.
    #[test]
    fn the_labels_are_the_ones_the_copy_hints_use() {
        let selector = Selector::new(3, Mode::Activate);
        assert_eq!(selector.labels, crate::copy_mode::labels_for(3));
    }

    #[test]
    fn a_label_takes_you_to_its_pane() {
        let mut selector = Selector::new(4, Mode::Activate);
        let second = selector.labels[1].clone();
        assert_eq!(selector.key(None, Some(&second), false), Outcome::Chose(1));
    }

    /// A capital letter is the same letter. Caps lock is not a reason for the
    /// selector to stop working.
    #[test]
    fn a_capital_letter_picks_the_same_pane() {
        let mut selector = Selector::new(4, Mode::Activate);
        let first = selector.labels[0].to_uppercase();
        assert_eq!(selector.key(None, Some(&first), false), Outcome::Chose(0));
    }

    /// Enough panes and the labels become pairs, and both letters are needed.
    #[test]
    fn a_two_letter_label_needs_both_letters() {
        let mut selector = Selector::new(40, Mode::Activate);
        let long = selector
            .labels
            .iter()
            .position(|label| label.chars().count() == 2)
            .expect("a list this long has pairs in it");
        let label = selector.labels[long].clone();
        let mut characters = label.chars();
        let first = characters.next().unwrap().to_string();
        let second = characters.next().unwrap().to_string();
        assert_eq!(selector.key(None, Some(&first), false), Outcome::Typing);
        assert_eq!(
            selector.key(None, Some(&second), false),
            Outcome::Chose(long)
        );
    }

    /// A letter that cannot lead anywhere starts the label again, rather than
    /// leaving the selector unable to answer anything.
    #[test]
    fn a_letter_that_leads_nowhere_starts_again() {
        let mut selector = Selector::new(2, Mode::Activate);
        assert_eq!(selector.key(None, Some("z"), false), Outcome::Typing);
        assert_eq!(selector.typing, "", "a dead end was kept");
        let first = selector.labels[0].clone();
        assert_eq!(selector.key(None, Some(&first), false), Outcome::Chose(0));
    }

    #[test]
    fn backspace_edits_and_escape_gives_up() {
        let mut selector = Selector::new(40, Mode::Activate);
        selector.typing = "as".into();
        assert_eq!(
            selector.key(Some("Backspace"), None, false),
            Outcome::Typing
        );
        assert_eq!(selector.typing, "a");
        assert_eq!(
            selector.key(Some("Escape"), None, false),
            Outcome::Cancelled
        );
    }

    /// The two chords the previous selector answered.
    #[test]
    fn control_g_gives_up_and_control_u_starts_again() {
        let mut selector = Selector::new(40, Mode::Activate);
        selector.typing = "as".into();
        assert_eq!(selector.key(None, Some("u"), true), Outcome::Typing);
        assert_eq!(selector.typing, "");
        assert_eq!(selector.key(None, Some("g"), true), Outcome::Cancelled);
    }

    /// Nothing reaches the shell while the selector is open: a keystroke
    /// through it would run in whichever pane happens to be in front.
    #[test]
    fn anything_else_is_swallowed_rather_than_passed_on() {
        let mut selector = Selector::new(4, Mode::Activate);
        assert_eq!(selector.key(Some("F5"), None, false), Outcome::Ignored);
        assert_eq!(selector.key(None, Some("!"), false), Outcome::Ignored);
        assert_eq!(selector.key(None, None, false), Outcome::Ignored);
    }

    /// Reading order: down first, then across within a row. Letters assigned
    /// in any other order feel arbitrary against the panes you are looking at.
    #[test]
    fn labels_are_assigned_in_reading_order() {
        let panes = vec![
            placement(200.0, 100.0),
            placement(0.0, 0.0),
            placement(200.0, 0.0),
            placement(0.0, 100.0),
        ];
        let order = reading_order(&panes);
        assert_eq!(order, vec![1, 2, 3, 0]);
    }

    /// One pane is still a selector rather than a special case: opening it
    /// with nothing to choose between should not panic or choose for you.
    #[test]
    fn a_single_pane_is_not_a_special_case() {
        let mut selector = Selector::new(1, Mode::Activate);
        assert_eq!(selector.labels.len(), 1);
        let only = selector.labels[0].clone();
        assert_eq!(selector.key(None, Some(&only), false), Outcome::Chose(0));
    }

    /// And the mode is carried from the key that opened it, so what happens
    /// after the choice never has to be asked.
    #[test]
    fn the_mode_is_carried_rather_than_asked() {
        for mode in [Mode::Activate, Mode::Swap] {
            assert_eq!(Selector::new(2, mode).mode, mode);
        }
    }
}
