//! What the keys do, in one place.
//!
//! Split out of the event handler for two reasons. An agent asking the MCP
//! surface `meta.keybindings` should get the same answer the window acts on,
//! not a second list that drifts from it. And a chain of `matches!` arms
//! scattered through a winit callback is a poor place to look up what a key
//! does.

use winit::keyboard::{Key, NamedKey};

/// Which way a pane-focus key points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Direction::Left => "Left",
            Direction::Right => "Right",
            Direction::Up => "Up",
            Direction::Down => "Down",
        }
    }
}

/// Something the front end does in response to a key, rather than sending it
/// to the shell.
///
/// Also the command palette's rows: an action a key can reach is an action a
/// palette should list, and keeping them one list is what stops the two from
/// drifting apart.
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
    Search,
    CommandPalette,
    Launcher,
    CopyMode,
    QuickSelect,
    CockpitInbox,
    GitPanel,
    Composer,
    ThemePicker,
    LeftTabBar,
    DirJump,
    NewWindow,
    ClosePane,
    ZoomPane,
    /// Show the files under the pane's directory, down the left edge.
    TreeSidebar,
    /// Send a crew of agents at one task, each in its own worktree.
    FleetLaunch,
    /// Throw away the scrollback, keeping what is on screen.
    ClearScrollback,
    /// Throw away the scrollback and the screen with it.
    ClearScreen,
    /// Put a letter on every pane and go to the one that is typed.
    SelectPane,
    /// The same, but exchange the chosen pane with the one in front.
    SwapPane,
    FocusPane(Direction),
    /// One-based, as the key is labelled.
    SelectTab(u8),
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    ToggleFullScreen,
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
            Action::Search => "Search",
            Action::CommandPalette => "CommandPalette",
            Action::Launcher => "Launcher",
            Action::CopyMode => "CopyMode",
            Action::QuickSelect => "QuickSelect",
            Action::CockpitInbox => "CockpitInbox",
            Action::GitPanel => "GitPanel",
            Action::Composer => "Composer",
            Action::ThemePicker => "ThemePicker",
            Action::LeftTabBar => "LeftTabBar",
            Action::DirJump => "DirJump",
            Action::NewWindow => "NewWindow",
            Action::ClosePane => "ClosePane",
            Action::ZoomPane => "ZoomPane",
            Action::TreeSidebar => "TreeSidebar",
            Action::FleetLaunch => "FleetLaunch",
            Action::ClearScrollback => "ClearScrollback",
            Action::ClearScreen => "ClearScreen",
            Action::SelectPane => "SelectPane",
            Action::SwapPane => "SwapPane",
            Action::FocusPane(Direction::Left) => "FocusPaneLeft",
            Action::FocusPane(Direction::Right) => "FocusPaneRight",
            Action::FocusPane(Direction::Up) => "FocusPaneUp",
            Action::FocusPane(Direction::Down) => "FocusPaneDown",
            Action::SelectTab(1) => "SelectTab1",
            Action::SelectTab(2) => "SelectTab2",
            Action::SelectTab(3) => "SelectTab3",
            Action::SelectTab(4) => "SelectTab4",
            Action::SelectTab(5) => "SelectTab5",
            Action::SelectTab(6) => "SelectTab6",
            Action::SelectTab(7) => "SelectTab7",
            Action::SelectTab(8) => "SelectTab8",
            Action::SelectTab(_) => "SelectTab9",
            Action::IncreaseFontSize => "IncreaseFontSize",
            Action::DecreaseFontSize => "DecreaseFontSize",
            Action::ResetFontSize => "ResetFontSize",
            Action::ToggleFullScreen => "ToggleFullScreen",
        }
    }
}

impl Action {
    /// What the palette calls this.
    ///
    /// Separate from `name()`, which is the stable identifier an agent sees:
    /// renaming a row in the UI should not change what a script matches on.
    pub fn label(self) -> &'static str {
        match self {
            Action::Copy => "Copy",
            Action::Paste => "Paste",
            Action::SplitRight => "Split Right",
            Action::SplitDown => "Split Down",
            Action::ScrollPageUp => "Scroll Page Up",
            Action::ScrollPageDown => "Scroll Page Down",
            Action::NewTab => "New Tab",
            Action::NextTab => "Next Tab",
            Action::PreviousTab => "Previous Tab",
            Action::CloseTab => "Close Tab",
            Action::Search => "Search",
            Action::CommandPalette => "Command Palette",
            Action::Launcher => "New Tab With...",
            Action::CopyMode => "Copy Mode",
            Action::QuickSelect => "Quick Select",
            Action::CockpitInbox => "Agent Inbox",
            Action::GitPanel => "Git Status",
            Action::Composer => "Prompt Queue",
            Action::ThemePicker => "Theme",
            Action::LeftTabBar => "Left Tab Bar",
            Action::DirJump => "Go to Directory",
            Action::NewWindow => "New Window",
            Action::ClosePane => "Close Pane",
            Action::ZoomPane => "Zoom Pane",
            Action::TreeSidebar => "File Tree",
            Action::FleetLaunch => "Launch Fleet",
            Action::ClearScrollback => "Clear Scrollback",
            Action::ClearScreen => "Clear Screen",
            Action::SelectPane => "Select Pane",
            Action::SwapPane => "Swap Pane",
            Action::FocusPane(Direction::Left) => "Focus Pane Left",
            Action::FocusPane(Direction::Right) => "Focus Pane Right",
            Action::FocusPane(Direction::Up) => "Focus Pane Up",
            Action::FocusPane(Direction::Down) => "Focus Pane Down",
            // One row for the whole family: nine that differ by a digit push
            // everything else off a short list.
            Action::SelectTab(_) => "Select Tab By Number",
            Action::IncreaseFontSize => "Increase Font Size",
            Action::DecreaseFontSize => "Decrease Font Size",
            Action::ResetFontSize => "Reset Font Size",
            Action::ToggleFullScreen => "Full Screen",
        }
    }
}

/// Which modifiers a binding needs held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    /// Alt is a modifier the shell wants too -- Alt+B and Alt+F are readline's
    /// word motions -- so it appears here on arrow keys and nothing else.
    pub alt: bool,
}

impl Mods {
    pub fn name(self) -> &'static str {
        match (self.ctrl, self.shift, self.alt) {
            (true, true, false) => "CTRL|SHIFT",
            (true, false, false) => "CTRL",
            (false, true, false) => "SHIFT",
            (false, false, true) => "ALT",
            (true, false, true) => "CTRL|ALT",
            (false, true, true) => "SHIFT|ALT",
            (true, true, true) => "CTRL|SHIFT|ALT",
            (false, false, false) => "NONE",
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
    Arrow(Direction),
    /// F1 and up.
    Function(u8),
}

impl Trigger {
    pub fn name(self) -> String {
        match self {
            Trigger::Char(c) => c.to_ascii_uppercase().to_string(),
            Trigger::PageUp => "PageUp".to_string(),
            Trigger::PageDown => "PageDown".to_string(),
            Trigger::Tab => "Tab".to_string(),
            Trigger::Arrow(direction) => direction.name().to_string(),
            Trigger::Function(number) => format!("F{number}"),
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
            (Trigger::Arrow(Direction::Left), Key::Named(NamedKey::ArrowLeft)) => true,
            (Trigger::Arrow(Direction::Right), Key::Named(NamedKey::ArrowRight)) => true,
            (Trigger::Arrow(Direction::Up), Key::Named(NamedKey::ArrowUp)) => true,
            (Trigger::Arrow(Direction::Down), Key::Named(NamedKey::ArrowDown)) => true,
            (Trigger::Function(number), Key::Named(named)) => function_number(*named) == Some(number),
            _ => false,
        }
    }
}

/// Which function key this is, if it is one.
fn function_number(named: NamedKey) -> Option<u8> {
    Some(match named {
        NamedKey::F1 => 1,
        NamedKey::F2 => 2,
        NamedKey::F3 => 3,
        NamedKey::F4 => 4,
        NamedKey::F5 => 5,
        NamedKey::F6 => 6,
        NamedKey::F7 => 7,
        NamedKey::F8 => 8,
        NamedKey::F9 => 9,
        NamedKey::F10 => 10,
        NamedKey::F11 => 11,
        NamedKey::F12 => 12,
        _ => return None,
    })
}

pub struct Binding {
    pub mods: Mods,
    pub trigger: Trigger,
    pub action: Action,
}

const CTRL_SHIFT: Mods = Mods {
    ctrl: true,
    shift: true,
    alt: false,
};
const CTRL_SHIFT_ALT: Mods = Mods {
    ctrl: true,
    shift: true,
    alt: true,
};
const CTRL: Mods = Mods {
    ctrl: true,
    shift: false,
    alt: false,
};
const SHIFT: Mods = Mods {
    ctrl: false,
    shift: true,
    alt: false,
};
const ALT: Mods = Mods {
    ctrl: false,
    shift: false,
    alt: true,
};
const NONE: Mods = Mods {
    ctrl: false,
    shift: false,
    alt: false,
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
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('f'),
        action: Action::Search,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('p'),
        action: Action::CommandPalette,
    },
    // The launcher moves off N so a new window can have the key every
    // application puts it on.
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('l'),
        action: Action::Launcher,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('n'),
        action: Action::NewWindow,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('z'),
        action: Action::ZoomPane,
    },
    // The pane selector, on the letter the previous front end used. Swapping
    // is the same gesture with a different ending, so it is the same key with
    // a different hand on it -- and Alt rather than another letter, because
    // the two belong together in the muscle memory.
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('\''),
        action: Action::SelectPane,
    },
    Binding {
        mods: CTRL_SHIFT_ALT,
        trigger: Trigger::Char('\''),
        action: Action::SwapPane,
    },
    // Throw away the history. The screen is kept, because the reason to ask is
    // almost always "this pane has a hundred thousand lines of build output
    // behind it" and not "I want to lose what I am reading". Adding Alt takes
    // the screen too, which is `clear` with nothing left to scroll back to.
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('k'),
        action: Action::ClearScrollback,
    },
    // The file tree, on the letter every editor uses for it.
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('b'),
        action: Action::TreeSidebar,
    },
    // A crew of agents on one task, each in its own worktree. On the letter
    // the cockpit already uses for itself.
    Binding {
        mods: CTRL_SHIFT_ALT,
        trigger: Trigger::Char('a'),
        action: Action::FleetLaunch,
    },
    Binding {
        mods: CTRL_SHIFT_ALT,
        trigger: Trigger::Char('k'),
        action: Action::ClearScreen,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('q'),
        action: Action::ClosePane,
    },
    // Alt and an arrow moves between panes. Alt+letters stay with the shell,
    // which is where readline's word motions live.
    Binding {
        mods: ALT,
        trigger: Trigger::Arrow(Direction::Left),
        action: Action::FocusPane(Direction::Left),
    },
    Binding {
        mods: ALT,
        trigger: Trigger::Arrow(Direction::Right),
        action: Action::FocusPane(Direction::Right),
    },
    Binding {
        mods: ALT,
        trigger: Trigger::Arrow(Direction::Up),
        action: Action::FocusPane(Direction::Up),
    },
    Binding {
        mods: ALT,
        trigger: Trigger::Arrow(Direction::Down),
        action: Action::FocusPane(Direction::Down),
    },
    // Font size, on the keys every application uses. `=` rather than `+`
    // because that is the unshifted key, and a terminal that needed shift to
    // grow the text would be the only one.
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('='),
        action: Action::IncreaseFontSize,
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('+'),
        action: Action::IncreaseFontSize,
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('-'),
        action: Action::DecreaseFontSize,
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('0'),
        action: Action::ResetFontSize,
    },
    Binding {
        mods: NONE,
        trigger: Trigger::Function(11),
        action: Action::ToggleFullScreen,
    },
    // Ctrl and a digit goes to that tab. Nine is the last one, and reaches
    // the last tab however many there are, which is what every browser does.
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('1'),
        action: Action::SelectTab(1),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('2'),
        action: Action::SelectTab(2),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('3'),
        action: Action::SelectTab(3),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('4'),
        action: Action::SelectTab(4),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('5'),
        action: Action::SelectTab(5),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('6'),
        action: Action::SelectTab(6),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('7'),
        action: Action::SelectTab(7),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('8'),
        action: Action::SelectTab(8),
    },
    Binding {
        mods: CTRL,
        trigger: Trigger::Char('9'),
        action: Action::SelectTab(9),
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('x'),
        action: Action::CopyMode,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('s'),
        action: Action::QuickSelect,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('a'),
        action: Action::CockpitInbox,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('g'),
        action: Action::GitPanel,
    },
    Binding {
        mods: CTRL_SHIFT,
        trigger: Trigger::Char('j'),
        action: Action::Composer,
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
pub fn action_for(key: &Key, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
    BINDINGS
        .iter()
        .filter(|binding| {
            binding.mods.ctrl == ctrl && binding.mods.shift == shift && binding.mods.alt == alt
        })
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
            action_for(&character("C"), true, true, false),
            Some(Action::Copy),
            "shift capitalised the letter, which is not a different key"
        );
        assert_eq!(action_for(&character("c"), true, true, false), Some(Action::Copy));
    }

    #[test]
    fn plain_tab_still_completes_in_the_shell() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), false, false, false),
            None,
            "taking Tab would break every shell's completion"
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), true, false, false),
            Some(Action::NextTab)
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::Tab), true, true, false),
            Some(Action::PreviousTab)
        );
    }

    #[test]
    fn plain_ctrl_c_stays_the_programs_interrupt() {
        assert_eq!(
            action_for(&character("c"), true, false, false),
            None,
            "taking Ctrl+C would leave no way to stop a running program"
        );
    }

    #[test]
    fn unshifted_pages_belong_to_the_program() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::PageUp), false, false, false),
            None,
            "a pager needs its own PageUp"
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::PageUp), false, true, false),
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

#[cfg(test)]
mod added_binding_tests {
    use super::*;

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    /// Ctrl+= and Ctrl+- grow and shrink the text in every application that
    /// shows text. A terminal without them is the odd one out, and this one
    /// was: the whole family was missing.
    #[test]
    fn the_font_size_keys_exist() {
        assert_eq!(
            action_for(&character("="), true, false, false),
            Some(Action::IncreaseFontSize)
        );
        assert_eq!(
            action_for(&character("-"), true, false, false),
            Some(Action::DecreaseFontSize)
        );
        assert_eq!(
            action_for(&character("0"), true, false, false),
            Some(Action::ResetFontSize)
        );
    }

    /// On many layouts Ctrl+Shift+= is how `+` is typed, and people press it
    /// meaning "bigger" either way.
    #[test]
    fn the_shifted_plus_grows_the_text_too() {
        assert_eq!(
            action_for(&character("+"), true, false, false),
            Some(Action::IncreaseFontSize)
        );
    }

    #[test]
    fn ctrl_and_a_digit_goes_to_that_tab() {
        assert_eq!(
            action_for(&character("1"), true, false, false),
            Some(Action::SelectTab(1))
        );
        assert_eq!(
            action_for(&character("9"), true, false, false),
            Some(Action::SelectTab(9))
        );
    }

    /// Alt and an arrow moves between panes. Alt and a *letter* must not:
    /// Alt+B and Alt+F are readline's word motions, and taking them would
    /// break editing a long command line.
    #[test]
    fn alt_arrows_move_between_panes_and_alt_letters_do_not() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::ArrowLeft), false, false, true),
            Some(Action::FocusPane(Direction::Left))
        );
        assert_eq!(
            action_for(&Key::Named(NamedKey::ArrowDown), false, false, true),
            Some(Action::FocusPane(Direction::Down))
        );
        assert_eq!(
            action_for(&character("b"), false, false, true),
            None,
            "Alt+B is readline's word-back"
        );
        assert_eq!(
            action_for(&character("f"), false, false, true),
            None,
            "Alt+F is readline's word-forward"
        );
    }

    /// A plain arrow moves the shell's cursor and must keep doing so.
    #[test]
    fn an_unmodified_arrow_still_belongs_to_the_shell() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::ArrowLeft), false, false, false),
            None
        );
    }

    #[test]
    fn f11_goes_full_screen_and_the_other_function_keys_are_the_programs() {
        assert_eq!(
            action_for(&Key::Named(NamedKey::F11), false, false, false),
            Some(Action::ToggleFullScreen)
        );
        for key in [NamedKey::F1, NamedKey::F5, NamedKey::F10, NamedKey::F12] {
            assert_eq!(
                action_for(&Key::Named(key), false, false, false),
                None,
                "{key:?} belongs to whatever is running"
            );
        }
    }

    #[test]
    fn a_new_window_and_a_pane_of_ones_own_are_both_reachable() {
        assert_eq!(
            action_for(&character("n"), true, true, false),
            Some(Action::NewWindow)
        );
        assert_eq!(
            action_for(&character("q"), true, true, false),
            Some(Action::ClosePane)
        );
        assert_eq!(
            action_for(&character("z"), true, true, false),
            Some(Action::ZoomPane)
        );
        // The launcher moved to make room, and has to still be somewhere.
        assert_eq!(
            action_for(&character("l"), true, true, false),
            Some(Action::Launcher)
        );
    }

    /// Every action has a name and a label, and no two *different* actions
    /// share a name -- the name is what an agent matches on over MCP.
    ///
    /// Two keys reaching one action is fine and deliberate: Ctrl+= and Ctrl++
    /// both grow the text, because on many layouts they are the same physical
    /// key with and without shift.
    #[test]
    fn every_bound_action_is_named_distinctly() {
        let mut by_name: std::collections::HashMap<&str, Action> =
            std::collections::HashMap::new();
        for binding in BINDINGS {
            let name = binding.action.name();
            assert!(!name.is_empty(), "{:?} has no name", binding.action);
            assert!(!binding.action.label().is_empty(), "{name} has no label");
            if let Some(other) = by_name.insert(name, binding.action) {
                assert_eq!(
                    other, binding.action,
                    "{other:?} and {:?} are both called {name}",
                    binding.action
                );
            }
        }
    }
}
