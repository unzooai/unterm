use crate::termwindow::box_model::ComputedElement;
use crate::TermWindow;
use config::keyassignment::KeyAssignment;
use downcast_rs::{impl_downcast, Downcast};
use std::cell::Ref;
use wezterm_term::{KeyCode, KeyModifiers, MouseEvent};

pub trait Modal: Downcast {
    fn perform_assignment(
        &self,
        _assignment: &KeyAssignment,
        _term_window: &mut TermWindow,
    ) -> bool {
        false
    }
    /// IME composition status changed while this modal owns the keyboard.
    /// A modal with a text input should display the in-progress
    /// composition inline and return true; returning true suppresses the
    /// terminal-side composing overlay, which would otherwise paint the
    /// marked text at the PANE cursor behind the modal — with a CJK input
    /// method that made palettes look dead ("输入的地方不支持输入").
    fn advise_compose(&self, _status: &::window::DeadKeyStatus) -> bool {
        false
    }
    /// Pixel rect of the modal's text caret, so the OS positions the IME
    /// candidate window next to the modal's input instead of the pane
    /// cursor. None = fall back to the pane cursor position.
    fn ime_cursor_rect(&self, _term_window: &TermWindow) -> Option<::window::Rect> {
        None
    }
    fn mouse_event(&self, event: MouseEvent, term_window: &mut TermWindow) -> anyhow::Result<()>;
    fn key_down(
        &self,
        key: KeyCode,
        mods: KeyModifiers,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<bool>;
    fn computed_element(
        &self,
        term_window: &mut TermWindow,
    ) -> anyhow::Result<Ref<'_, [ComputedElement]>>;
    fn reconfigure(&self, term_window: &mut TermWindow);
}
impl_downcast!(Modal);
