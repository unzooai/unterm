//! What the keys do, in one place.
//!
//! Split out of the event handler for two reasons. An agent asking the MCP
//! surface `meta.keybindings` should get the same answer the window acts on,
//! not a second list that drifts from it. And a chain of `matches!` arms
//! scattered through a winit callback is a poor place to look up what a key
//! does.

use winit::keyboard::{Key, NamedKey};

/// Something the front end does in response to a key, rather than sending it
/// to the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Copy,
    Paste,
    SplitRight,
    SplitDown,
    ScrollPageUp,
    ScrollPageDown,
    NewTab,
    NextTab,
    PreviousTab,
    CloseTab,
}

impl Action {
    /// The name an agent sees. Kept stable: it is part of the MCP reply.
    pub fn name(self) -> &'static str {
        match self {
            Action::Copy => "Copy",
            Action::Paste => "Paste",
            Action::SplitRight => "SplitRight",
            Action::SplitDown => "SplitDown",
            Action::ScrollPageUp => "ScrollPageUp",
            Action::ScrollPageDown => "ScrollPageDown",
            Action::NewTab => "NewTab",
            Action::NextTab => "NextTab",
            Action::PreviousTab => "PreviousTab",
            Action::CloseTab => "CloseTab",
        }
    }
}

/// Which modifiers a binding needs held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
}

impl Mods {
    pub fn name(self) -> &'static str {
        match (self.ctrl, self.shift) {
            (true, true) => "CTRL|SHIFT",
            (true, false) => "CTRL",
            (false, true) => "SHIFT",
            (false, false) => "NONE",
        }
    }
}

/// The key a binding is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// A character, matched without regard to case: Ctrl+Shift+C is the same
    /// binding whether or not the shift also capitalised the letter.
    Char(char),
    PageUp,
    PageDown,
    Tab,
}

impl Trigger {
    pub fn name(self) -> String {
        match self {
            Trigger::Char(c) => c.to_ascii_uppercase().to_string(),
            Trigger::PageUp => "PageUp".to_string(),
            Trigger::PageDown => "PageDown".to_string(),
            Trigger::Tab => "Tab".to_string(),
        }
    }

    fn matches(self, key: &Key) -> bool {
        match (self, key) {
            (Trigger::Char(c), Key::Character(text)) => {
                text.eq_ignore_ascii_case(c.encode_utf8(&mut [0u8; 4]))
            }
            (Trigger::PageUp, Key::Named(NamedKey::PageUp)) => true,
            (Trigger::PageDown, Key::Named(NamedKey::PageDown)) => true,
            (Trigger::Tab, Key::Named(NamedKey::Tab)) => true,
            _ => false,
        }
    }
}

pub struct Binding {
    pub mods: Mods,
    pub trigger: Trigger,
    pub action: Action,
}

const CTRL_SHIFT: Mods = Mods {
    ctrl: true,
    shift: true,
};
const CTRL: Mods = Mods {
    ctrl: true,
    shift: false,
};
const SHIFT: Mods = Mods {
    ctrl: false,
    shift: true,
};

/// Every key this front end keeps for itself.
///
/// Everything not listed goes to the shell, which is why plain Ctrl+C is
/// absent: it has to stay interrupt or a running program can never be
/// stopped. Shift+Page scrolls the viewport for the same reason in reverse --
/// unshifted pages belong to the program, which is how a pager gets its keys.
pub const BINDINGS: &[Binding] = &[
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('c'),
        action: Action::Copy,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('v'),
        action: Action::Paste,
    },
    // Right and down, as every terminal spells them.
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('d'),
        action: Action::SplitRight,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('e'),
        action: Action::SplitDown,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('t'),
        action: Action::NewTab,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('w'),
        action: Action::CloseTab,
    },
    // Ctrl+Tab cycles, Ctrl+Shift+Tab cycles back -- the pair every tabbed
    // application uses. Plain Tab still belongs to the shell's completion.
    Binding {
        mods: CTRL,
        trigger: Trigger::Tab,
        action: Action::NextTab,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Tab,
        action: Action::PreviousTab,
    },
    Binding {
        mods: SHIFT,
        trigger: Trigger::PageUp,
        action: Action::ScrollPageUp,
    },
    Binding {
        mods: SHIFT,
        trigger: Trigger::PageDown,
        action: Action::ScrollPageDown,
    },
];

/// What this key press means to the front end, if anything.
///
/// The most specific binding wins: Ctrl+Shift+C is a copy, not a scroll that
/// happens to have shift held.
pub fn action_for(key: &Key, ctrl: bool, shift: bool) -> Option<Action> {
    BINDINGS
        .iter()
        .filter(|binding| binding.mods.ctrl == ctrl && binding.mods.shift == shift)
        .find(|binding| binding.trigger.matches(key))
        .map(|binding| binding.action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    #[test]
    fn ctrl_shift_c_copies_whatever_the_shift_did_to_the_letter() {
        assert_eq!(
            action_for(&character("C"), true, true),
            Some(Action::Copy),
            "shift capitalised the letter, which is not a different key"
        );
        assert_eq!(action_for(&character("c"), true, true), Some(Action::Copy));
    }

    #[test]
    fn plain_tab_still_completes_in_the_shell() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), false, false),
            None,
            "taking Tab would break every shell's completion"
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), true, false),
            Some(Action::NextTab)
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), true, true),
            Some(Action::PreviousTab)
        );
    }

    #[test]
    fn plain_ctrl_c_stays_the_programs_interrupt() {
        assert_eq!(
            action_for(&character("c"), true, false),
            None,
            "taking Ctrl+C would leave no way to stop a running program"
        );
    }

    #[test]
    fn unshifted_pages_belong_to_the_program() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::PageUp), false, false),
            None,
            "a pager needs its own PageUp"
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::PageUp), false, true),
            Some(Action::ScrollPageUp)
        );
    }

    #[test]
    fn every_binding_has_a_distinct_key() {
        for (i, a) in BINDINGS.iter().enumerate() {
            for b in &BINDINGS[i + 1..] {
                assert!(
                    !(a.mods == b.mods && a.trigger == b.trigger),
                    "{:?}+{:?} is bound twice",
                    a.mods,
                    a.trigger
                );
            }
        }
    }
}
