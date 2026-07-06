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
        // Read the clipboard on a dedicated background thread. macOS
        // pasteboards are lazy: `read()` makes a synchronous cross-process
        // request to whichever app owns the clipboard (a browser, editor, IM),
        // and if that app is busy or the payload is large/rich, the request
        // blocks. Done on the GUI thread — where paste_from_clipboard is
        // called from the mouse/key handler — that froze the whole window for
        // as long as the source app took to answer ("右键粘贴卡半秒"). Off the
        // GUI thread it can't stall the UI; we hop back via window.notify to
        // send the paste once the text is in hand.
        let window_for_read = window.clone();
        let reader = promise::spawn::spawn_into_new_thread(move || {
            // get_clipboard resolves synchronously on this thread (on macOS it
            // performs the NSPasteboard read here); await drains the ready
            // future without blocking any executor.
            promise::spawn::block_on(window_for_read.get_clipboard(clipboard))
        });
        promise::spawn::spawn(async move {
            if let Ok(clip) = reader.await {
                window.notify(TermWindowNotif::Apply(Box::new(move |myself| {
                    if let Some(pane) = myself
                        .pane_state(pane_id)
                        .overlay
                        .as_ref()
                        .map(|overlay| overlay.pane.clone())
                        .or_else(|| {
                            let mux = Mux::get();
                            mux.get_pane(pane_id)
                        })
                    {
                        pane.send_paste(&clip).ok();
                    }
                })));
            }
        })
        .detach();
        self.maybe_scroll_to_bottom_for_input(&pane);
    }
}
