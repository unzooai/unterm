//! GuiWin refers to a GUI TermWindow rather than a mux window.
use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use config::keyassignment::{ClipboardCopyDestination, KeyAssignment};
use mux::pane::PaneId;
use mux::window::WindowId as MuxWindowId;
use mux::Mux;
use mux::pane::MuxPane;
use wezterm_render_escapes::lines_to_escapes;
use wezterm_dynamic::{FromDynamic, ToDynamic};
use wezterm_toast_notification::ToastNotification;
use window::{Connection, ConnectionOps, DeadKeyStatus, WindowOps, WindowState};

#[derive(Clone)]
pub struct GuiWin {
    pub mux_window_id: MuxWindowId,
    pub window: ::window::Window,
}

impl GuiWin {
    pub fn new(term_window: &TermWindow) -> Self {
        let window = term_window.window.clone().unwrap();
        let mux_window_id = term_window.mux_window_id;
        Self {
            window,
            mux_window_id,
        }
    }
}
