use crate::termwindow::TermWindowNotif;
use crate::TermWindow;
use config::keyassignment::{ClipboardCopyDestination, ClipboardPasteSource};
use mux::pane::Pane;
use mux::Mux;
use std::sync::Arc;
use window::{Clipboard, WindowOps};

impl TermWindow {
    pub fn copy_to_clipboard(&self, clipboard: ClipboardCopyDestination, text: String) {
        let clipboard = match clipboard {
            ClipboardCopyDestination::Clipboard => [Some(Clipboard::Clipboard), None],
            ClipboardCopyDestination::PrimarySelection => [Some(Clipboard::PrimarySelection), None],
            ClipboardCopyDestination::ClipboardAndPrimarySelection => [
                Some(Clipboard::Clipboard),
                Some(Clipboard::PrimarySelection),
            ],
        };
        for &c in &clipboard {
            if let Some(c) = c {
                self.window.as_ref().unwrap().set_clipboard(c, text.clone());
            }
        }
    }

    pub fn paste_from_clipboard(&mut self, pane: &Arc<dyn Pane>, clipboard: ClipboardPasteSource) {
        self.paste_from_clipboard_with_feedback(pane, clipboard, false);
    }

    /// Paste asynchronously and optionally report the final result to the user.
    ///
    /// Keyboard-driven paste remains quiet. Direct mouse paste asks for
    /// feedback so that its notice reflects `send_paste`, rather than merely
    /// reflecting that an asynchronous clipboard read was started.
    pub(crate) fn paste_from_clipboard_with_feedback(
        &mut self,
        pane: &Arc<dyn Pane>,
        clipboard: ClipboardPasteSource,
        show_feedback: bool,
    ) {
        let pane_id = pane.pane_id();
        log::trace!(
            "paste_from_clipboard in pane {} {:?}",
            pane.pane_id(),
            clipboard
        );
        let window = self.window.as_ref().unwrap().clone();
        let clipboard = match clipboard {
            ClipboardPasteSource::Clipboard => Clipboard::Clipboard,
            ClipboardPasteSource::PrimarySelection => Clipboard::PrimarySelection,
        };
        // Read the clipboard on a dedicated background thread on platforms
        // where the backend may block while talking to another process. macOS
        // pasteboards are lazy; Windows clipboard ownership is process-global
        // and transiently contended by clipboard managers/RDP/previous owners.
        // Doing either read on the GUI event handler makes right-click paste
        // and normal typing feel sticky.
        #[cfg(any(target_os = "macos", windows))]
        {
            let window_for_read = window.clone();
            let reader = promise::spawn::spawn_into_new_thread(move || {
                // get_clipboard resolves synchronously on this thread for the
                // blocking backends; await drains the ready future without
                // blocking the GUI event loop.
                promise::spawn::block_on(window_for_read.get_clipboard(clipboard))
            });
            promise::spawn::spawn(async move {
                match reader.await {
                    Ok(clip) => notify_paste(window, pane_id, clip, show_feedback),
                    Err(err) => notify_paste_error(
                        window,
                        pane_id,
                        show_feedback,
                        format!("clipboard read failed: {err:#}"),
                    ),
                }
            })
            .detach();
        }
        // Other Unix backends return an async future without the same short,
        // synchronous retry loop in this call site.
        #[cfg(all(not(target_os = "macos"), not(windows)))]
        {
            let future = window.get_clipboard(clipboard);
            promise::spawn::spawn(async move {
                match future.await {
                    Ok(clip) => notify_paste(window, pane_id, clip, show_feedback),
                    Err(err) => notify_paste_error(
                        window,
                        pane_id,
                        show_feedback,
                        format!("clipboard read failed: {err:#}"),
                    ),
                }
            })
            .detach();
        }
        self.maybe_scroll_to_bottom_for_input(&pane);
    }
}

fn notify_paste(
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
    clip: String,
    show_feedback: bool,
) {
    if clip.is_empty() {
        // Empty clipboard: nothing to paste. Not a failure — no error toast,
        // and no empty bracketed-paste markers sent to the application.
        return;
    }
    window.notify(TermWindowNotif::Apply(Box::new(move |myself| {
        // Non-inserting lookup: `pane_state()` would create a fresh entry
        // keyed by a possibly-dead pane id that nothing ever cleans up.
        let overlay_pane = myself
            .pane_state
            .borrow()
            .get(&pane_id)
            .and_then(|state| state.overlay.as_ref().map(|overlay| overlay.pane.clone()));
        if let Some(pane) = overlay_pane.or_else(|| {
            let mux = Mux::get();
            mux.get_pane(pane_id)
        }) {
            match pane.send_paste(&clip) {
                Ok(()) if show_feedback => {
                    myself.show_ui_notice(crate::i18n::t("interaction.pasted"));
                }
                Ok(()) => {}
                Err(err) => {
                    let message = format!("clipboard paste failed: {err:#}");
                    log::warn!("{message} for pane {pane_id}");
                    if show_feedback {
                        myself.show_ui_notice(crate::i18n::t("interaction.paste_failed"));
                    }
                }
            }
        } else {
            let message = format!("clipboard paste target pane {pane_id} no longer exists");
            log::warn!("{message}");
            if show_feedback {
                myself.show_ui_notice(crate::i18n::t("interaction.paste_failed"));
            }
        }
    })));
}

fn notify_paste_error(
    window: ::window::Window,
    pane_id: mux::pane::PaneId,
    show_feedback: bool,
    message: String,
) {
    log::warn!("{message} for pane {pane_id}");
    if show_feedback {
        window.notify(TermWindowNotif::Apply(Box::new(move |myself| {
            myself.show_ui_notice(crate::i18n::t("interaction.paste_failed"));
        })));
    }
}
