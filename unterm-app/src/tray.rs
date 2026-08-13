//! The indicator a window leaves behind when it parks its sessions.
//!
//! Choosing "keep running in the background" at the close prompt used to end
//! with the process gone and nothing on screen. The Core still held the
//! shells and the agents were still working, but the only evidence of that
//! was the user's memory of a dialog they had dismissed -- which is another
//! way of spelling "lost". This module is the evidence: a menu-bar item on
//! macOS, a notification-area icon on Windows and Linux, saying how much is
//! still running and offering the window back.
//!
//! It is deliberately an *indicator*, not a second front end. It reports two
//! numbers and offers two verbs; everything else is behind "open window",
//! because a terminal's controls belong in the terminal.

/// What the indicator has to report: the sessions the Core is holding, and
/// the agents among them that are waiting on the user.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Status {
    pub sessions: usize,
    pub waiting: usize,
}

/// What the user asked of the indicator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Bring the window back onto the screen.
    Open,
    /// End everything the Core is holding and quit for real.
    QuitAll,
}

/// Menu identifiers: fixed strings rather than generated ids. The events
/// arrive on a process-global channel, and on Linux the items are built on a
/// different thread from the one that reads them, so a constant both sides
/// can name is what makes the round trip work without shared handles.
const HEADER: &str = "unterm.tray.status";
const OPEN: &str = "unterm.tray.open";
const QUIT: &str = "unterm.tray.quit";

/// Set when something other than the indicator asks for the window back.
///
/// Three things want a parked window returned and none of them runs on the
/// event loop: an agent calling `instance.focus`, macOS reopening the app
/// from the Dock or Finder, and the indicator's own menu. The first two land
/// here; the third arrives as a menu event, and `poll` folds them together so
/// the loop has one question to ask.
static WAKE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ask a parked process to put its window back.
pub fn request_wake() {
    WAKE.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Take the request, if there is one. Reading clears it.
fn wake_requested() -> bool {
    WAKE.swap(false, std::sync::atomic::Ordering::SeqCst)
}

/// Forget any request made while there was still a window to focus, so it
/// cannot un-park the window that is only now being parked.
pub fn clear_wake() {
    let _ = wake_requested();
}

/// Read the indicator's events, whichever thread its icon happens to be on.
///
/// The last action wins: a burst of clicks ending in "quit" means quit, and
/// one ending in "open" means open.
pub fn poll() -> Option<Action> {
    let mut action = wake_requested().then_some(Action::Open);
    while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
        if let Some(chosen) = action_for(event.id.as_ref()) {
            action = Some(chosen);
        }
    }
    // Double-clicking the icon is the shortcut past its own menu -- the
    // gesture people try first on Windows, and one that costs nothing to
    // honour wherever it is reported.
    while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
        if matches!(event, tray_icon::TrayIconEvent::DoubleClick { .. }) {
            action = Some(Action::Open);
        }
    }
    action
}

/// The menu row an id names, if it names one that does anything.
///
/// Separated from `poll` so the wiring can be tested: the ids travel as
/// strings through a channel muda owns, and a typo on either side would
/// otherwise show up as a menu item that silently does nothing.
fn action_for(id: &str) -> Option<Action> {
    match id {
        OPEN => Some(Action::Open),
        QUIT => Some(Action::QuitAll),
        _ => None,
    }
}

/// The words the indicator shows, in the user's language.
struct Labels {
    /// The disabled first row: what is still running.
    header: String,
    /// The hover text, which on Windows is the only text there is.
    tooltip: String,
    /// Text drawn beside the icon. Only worth the menu-bar space when
    /// somebody is actually waiting, so it is empty the rest of the time.
    title: String,
}

fn labels(status: Status) -> Labels {
    use unterm_services::i18n::t_args;
    let sessions = status.sessions.to_string();
    let waiting = status.waiting.to_string();
    // The same two sentences the close prompt uses, for the same reason:
    // "2 agents waiting, 3 sessions running" is a report someone can act on.
    let header = if status.waiting > 0 {
        t_args(
            "tray.status_waiting",
            &[("waiting", &waiting), ("sessions", &sessions)],
        )
    } else {
        t_args("tray.status", &[("sessions", &sessions)])
    };
    Labels {
        tooltip: t_args("tray.tooltip", &[("status", &header)]),
        title: if status.waiting > 0 { waiting } else { String::new() },
        header,
    }
}

/// The menu, identical on every platform: a disabled report, the way back to
/// the window, and the irreversible one last.
fn build_menu(labels: &Labels) -> Result<(tray_icon::menu::Menu, tray_icon::menu::MenuItem), tray_icon::menu::Error>
{
    use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
    use unterm_services::i18n::t;

    let menu = Menu::new();
    let header = MenuItem::with_id(HEADER, &labels.header, false, None);
    let open = MenuItem::with_id(OPEN, t("tray.open"), true, None);
    let quit = MenuItem::with_id(QUIT, t("tray.quit"), true, None);
    // Two separators, not one appended twice: a menu item is a handle, and
    // the same handle in two places is one item that moved.
    let above = PredefinedMenuItem::separator();
    let below = PredefinedMenuItem::separator();
    menu.append_items(&[&header, &above, &open, &below, &quit])?;
    Ok((menu, header))
}

/// The picture in the tray.
///
/// macOS renders a status item as a *template*: only the alpha survives and
/// the system tints the silhouette for the light or dark menu bar. Handing
/// it the app icon would put a black rounded square up there, so macOS gets
/// the mark alone. Windows and Linux draw a real icon, in colour, like every
/// neighbour in their tray.
fn icon() -> Option<tray_icon::Icon> {
    const MENU_BAR: &[u8] = include_bytes!("../../assets/icon/unterm-tray-44.png");
    const APP: &[u8] = include_bytes!("../../assets/icon/unterm-icon-256.png");
    let source = if cfg!(target_os = "macos") { MENU_BAR } else { APP };
    let image = image::load_from_memory(source).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    tray_icon::Icon::from_rgba(image.into_raw(), width, height).ok()
}

/// Show or hide the Dock tile for a process that has no window.
///
/// A parked Unterm has none, and a Dock icon that opens nothing when clicked
/// is worse than no Dock icon: the menu bar is where the app is now.
/// `Accessory` is the policy for exactly that -- menu-bar presence, no Dock
/// tile, no Cmd-Tab entry -- and `Regular` puts it all back with the window.
#[cfg(target_os = "macos")]
pub fn set_dock_visible(visible: bool) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    // NSApplicationActivationPolicyRegular / ...Accessory.
    let policy: isize = if visible { 0 } else { 1 };
    // SAFETY: called from the winit event loop, which is the main thread,
    // and NSApp exists by the time this process has had a window at all.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        if app.is_null() {
            return;
        }
        let _: bool = msg_send![app, setActivationPolicy: policy];
        if visible {
            // Returning from Accessory leaves the app behind whatever the
            // user looked at meanwhile; the window they just asked for has
            // to come back in front of it.
            let _: () = msg_send![app, activateIgnoringOtherApps: true];
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_visible(_visible: bool) {}

#[cfg(not(target_os = "linux"))]
pub use desktop::Tray;
#[cfg(target_os = "linux")]
pub use gtk_thread::Tray;

/// macOS and Windows: the icon belongs to a thread already running a native
/// event loop, and that is the thread winit is on.
#[cfg(not(target_os = "linux"))]
mod desktop {
    use super::{build_menu, icon, labels, Status};

    pub struct Tray {
        icon: tray_icon::TrayIcon,
        header: tray_icon::menu::MenuItem,
        shown: Status,
    }

    impl Tray {
        /// Put the indicator up, or report that this desktop has nowhere to
        /// put one. The caller needs that difference: parking sessions
        /// behind an indicator that never appeared is the invisible state
        /// this module exists to prevent.
        pub fn show(status: Status) -> Option<Self> {
            let labels = labels(status);
            let (menu, header) = build_menu(&labels)
                .map_err(|error| log::warn!("could not build the tray menu: {error}"))
                .ok()?;
            let mut builder = tray_icon::TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip(&labels.tooltip)
                .with_icon_as_template(true);
            if let Some(icon) = icon() {
                builder = builder.with_icon(icon);
            }
            if !labels.title.is_empty() {
                builder = builder.with_title(&labels.title);
            }
            match builder.build() {
                Ok(icon) => Some(Self {
                    icon,
                    header,
                    shown: status,
                }),
                Err(error) => {
                    log::warn!("no tray indicator on this desktop: {error}");
                    None
                }
            }
        }

        /// Redraw the counts, and only when they moved: rewriting the same
        /// text every tick makes some Windows shells flash their icon.
        pub fn update(&mut self, status: Status) {
            if status == self.shown {
                return;
            }
            self.shown = status;
            let labels = labels(status);
            self.header.set_text(&labels.header);
            let _ = self.icon.set_tooltip(Some(&labels.tooltip));
            if labels.title.is_empty() {
                self.icon.set_title(None::<&str>);
            } else {
                self.icon.set_title(Some(&labels.title));
            }
        }
    }
}

/// Linux: `tray-icon` speaks to libappindicator, which needs a gtk main loop
/// on the thread owning the icon. winit is running X11 or Wayland on the main
/// thread, not gtk, so the indicator gets a thread of its own. The window
/// never touches the icon -- it writes a `Status`, and the gtk thread notices.
#[cfg(target_os = "linux")]
mod gtk_thread {
    use super::{build_menu, icon, labels, Status};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    pub struct Tray {
        /// `None` means "take yourself down": the window came back, or this
        /// handle was dropped.
        wanted: Arc<Mutex<Option<Status>>>,
    }

    impl Tray {
        pub fn show(status: Status) -> Option<Self> {
            let wanted = Arc::new(Mutex::new(Some(status)));
            let worker = Arc::clone(&wanted);
            // The thread reports whether an icon actually appeared. Without
            // it the window would park its sessions behind an indicator that
            // silently failed to start on a headless or tray-less session --
            // the one outcome worse than not offering the choice at all.
            let (report, appeared) = mpsc::sync_channel::<bool>(1);
            std::thread::Builder::new()
                .name("unterm-tray".into())
                .spawn(move || run(worker, report, status))
                .ok()?;
            match appeared.recv_timeout(Duration::from_secs(5)) {
                Ok(true) => Some(Self { wanted }),
                _ => {
                    *wanted.lock().unwrap() = None;
                    None
                }
            }
        }

        pub fn update(&mut self, status: Status) {
            *self.wanted.lock().unwrap() = Some(status);
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            // The gtk thread sees the `None` on its next turn, drops the icon
            // and ends its own loop.
            *self.wanted.lock().unwrap() = None;
        }
    }

    fn run(wanted: Arc<Mutex<Option<Status>>>, report: mpsc::SyncSender<bool>, status: Status) {
        if gtk::init().is_err() {
            log::warn!("no gtk display for the tray indicator");
            let _ = report.send(false);
            return;
        }
        let labels = labels(status);
        let Ok((menu, header)) = build_menu(&labels)
            .map_err(|error| log::warn!("could not build the tray menu: {error}"))
        else {
            let _ = report.send(false);
            return;
        };
        let mut builder = tray_icon::TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(&labels.tooltip);
        if let Some(icon) = icon() {
            builder = builder.with_icon(icon);
        }
        let tray = match builder.build() {
            Ok(tray) => tray,
            Err(error) => {
                log::warn!("no tray indicator on this desktop: {error}");
                let _ = report.send(false);
                return;
            }
        };
        let _ = report.send(true);

        // Held in an Option so the loop can drop the icon itself: the panel
        // removes it when the indicator goes, and waiting for the thread to
        // unwind leaves a dead icon on the screen in between.
        let mut tray = Some(tray);
        let mut shown = status;
        gtk::glib::timeout_add_local(Duration::from_millis(250), move || {
            let next = *wanted.lock().unwrap();
            let Some(next) = next else {
                drop(tray.take());
                gtk::main_quit();
                return gtk::glib::ControlFlow::Break;
            };
            if next != shown {
                shown = next;
                let labels = labels(next);
                header.set_text(&labels.header);
                if let Some(tray) = tray.as_ref() {
                    let _ = tray.set_tooltip(Some(&labels.tooltip));
                    if labels.title.is_empty() {
                        tray.set_title(None::<&str>);
                    } else {
                        tray.set_title(Some(&labels.title));
                    }
                }
            }
            gtk::glib::ControlFlow::Continue
        });
        gtk::main();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_counts_waiting_agents_only_when_there_are_some() {
        let quiet = labels(Status {
            sessions: 3,
            waiting: 0,
        });
        assert!(quiet.header.contains('3'), "got {:?}", quiet.header);
        assert!(
            quiet.title.is_empty(),
            "the menu bar earns text only when somebody is waiting, got {:?}",
            quiet.title
        );

        let busy = labels(Status {
            sessions: 3,
            waiting: 2,
        });
        assert!(
            busy.header.contains('2') && busy.header.contains('3'),
            "got {:?}",
            busy.header
        );
        assert_eq!(busy.title, "2");
    }

    #[test]
    fn the_menus_ids_are_the_ones_the_reader_answers_to() {
        // `build_menu` labels its rows with these same constants, so this is
        // the round trip an event makes: item id out, action back.
        assert_eq!(action_for(OPEN), Some(Action::Open));
        assert_eq!(action_for(QUIT), Some(Action::QuitAll));
        // The report is a row, not a verb. Clicking it must do nothing --
        // and it is the row sitting directly above "open window".
        assert_eq!(action_for(HEADER), None);
        assert_eq!(action_for("unterm.tray.something.else"), None);
    }

    #[test]
    fn the_tooltip_names_the_product_and_carries_the_same_report() {
        let labels = labels(Status {
            sessions: 1,
            waiting: 0,
        });
        assert!(labels.tooltip.contains("Unterm"), "got {:?}", labels.tooltip);
        assert!(labels.tooltip.contains(&labels.header));
    }
}
