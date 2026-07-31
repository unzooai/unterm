//! System-level notifications, for OSC 9 / OSC 777 and finished downloads.
//!
//! On Windows this is a Shell_NotifyIcon balloon rather than a WinRT toast:
//! a balloon needs no AppUserModelID, no Start-menu shortcut and no COM
//! apartment, which means it works from this crate without knowing anything
//! about how the process was installed or launched. Windows renders it
//! through the same notification center as a toast anyway.
//!
//! A balloon must hang off a notification-area icon, and that icon must hang
//! off a window owned by a thread that pumps messages. None of our callers
//! can promise a message loop -- some are async executors, some are the GUI
//! thread mid-dispatch -- so the icon lives on its own dedicated thread,
//! created on first use, and callers talk to it over a channel.

/// Show a system notification. Best effort: an error means the platform
/// refused (no shell, no session), not that the caller did anything wrong.
#[cfg(not(windows))]
pub fn notify(_title: &str, _body: &str) -> anyhow::Result<()> {
    // Nothing here yet; macOS and Linux front ends have their own paths.
    Ok(())
}

/// Show a system notification. Best effort: an error means the platform
/// refused (no shell, no session), not that the caller did anything wrong.
#[cfg(windows)]
pub fn notify(title: &str, body: &str) -> anyhow::Result<()> {
    windows_impl::notify(title, body)
}

#[cfg(windows)]
mod windows_impl {
    use anyhow::{anyhow, Context as _};
    use std::sync::mpsc::{self, RecvTimeoutError, Sender};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    /// One notification to deliver, and where to say how it went. The reply
    /// channel is what makes `notify` a Result instead of fire-and-forget.
    struct Request {
        title: String,
        body: String,
        done: Sender<Result<(), String>>,
    }

    /// The one worker thread's inbox. `Mutex` rather than relying on
    /// `Sender: Sync`, so this compiles the same on every toolchain we meet.
    static WORKER: OnceLock<Mutex<Sender<Request>>> = OnceLock::new();

    pub fn notify(title: &str, body: &str) -> anyhow::Result<()> {
        let inbox = WORKER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            // The thread is never joined: it owns the tray icon for the life
            // of the process, and the process exiting is what removes it.
            std::thread::Builder::new()
                .name("unterm-toast".to_string())
                .spawn(move || worker(rx))
                .expect("spawn the toast worker thread");
            Mutex::new(tx)
        });

        let (done_tx, done_rx) = mpsc::channel();
        inbox
            .lock()
            .expect("toast inbox lock")
            .send(Request {
                title: title.to_string(),
                body: body.to_string(),
                done: done_tx,
            })
            .map_err(|_| anyhow!("the toast worker thread has exited"))?;

        // Shell_NotifyIcon itself returns immediately, so a long wait here
        // means the worker is wedged; give up rather than block the caller.
        match done_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(msg)) => Err(anyhow!(msg)).context("show a notification balloon"),
            Err(_) => Err(anyhow!("the toast worker did not answer")),
        }
    }

    /// The dedicated thread: owns the hidden window and the tray icon, and
    /// pumps messages so the shell can talk back to the icon.
    fn worker(rx: mpsc::Receiver<Request>) {
        let hwnd = match unsafe { create_message_window() } {
            Ok(hwnd) => Some(hwnd),
            Err(err) => {
                // Keep answering requests with the reason instead of dying,
                // so callers get an error rather than a hang.
                log::warn!("toast window unavailable: {err:#}");
                None
            }
        };
        let window_err = "no window to host the notification icon";
        let mut icon_added = false;

        loop {
            unsafe { pump_messages() };
            // A short timeout so the pump keeps running between requests;
            // the shell sends the icon messages at its own pace.
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(req) => {
                    let result = match hwnd {
                        Some(hwnd) => unsafe {
                            deliver(hwnd, &mut icon_added, &req.title, &req.body)
                        }
                        .map_err(|err| format!("{err:#}")),
                        None => Err(window_err.to_string()),
                    };
                    // The caller may have timed out and gone; that is fine.
                    let _ = req.done.send(result);
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    /// A message-only window for the icon to belong to. It never shows and
    /// never paints; it exists because NOTIFYICONDATA requires an hWnd.
    unsafe fn create_message_window() -> anyhow::Result<winapi::shared::windef::HWND> {
        use winapi::um::libloaderapi::GetModuleHandleW;
        use winapi::um::winuser::{
            CreateWindowExW, DefWindowProcW, RegisterClassW, HWND_MESSAGE, WNDCLASSW,
        };

        let class_name = wide("UntermToastHost");
        let hinstance = GetModuleHandleW(std::ptr::null());

        let mut wc: WNDCLASSW = std::mem::zeroed();
        // DefWindowProc directly: the window has no behavior of its own.
        wc.lpfnWndProc = Some(DefWindowProcW);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        if RegisterClassW(&wc) == 0 {
            // Only this thread registers the class, so "already exists" is
            // not expected; any failure here is a real one.
            return Err(anyhow!(
                "RegisterClassW failed: {}",
                winapi::um::errhandlingapi::GetLastError()
            ));
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            return Err(anyhow!(
                "CreateWindowExW failed: {}",
                winapi::um::errhandlingapi::GetLastError()
            ));
        }
        Ok(hwnd)
    }

    /// Drain this thread's message queue. The shell posts to the icon's
    /// window; if nothing ever dispatches, the shell decides the icon's
    /// owner is dead.
    unsafe fn pump_messages() {
        use winapi::um::winuser::{DispatchMessageW, PeekMessageW, TranslateMessage, PM_REMOVE};
        let mut msg = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    /// Put the balloon up, adding the tray icon first if it is not there.
    unsafe fn deliver(
        hwnd: winapi::shared::windef::HWND,
        icon_added: &mut bool,
        title: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        use winapi::um::shellapi::{
            Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE,
            NIM_MODIFY, NOTIFYICONDATAW,
        };
        use winapi::um::winuser::{LoadIconW, IDI_APPLICATION};

        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        // Everything in one struct: NIM_ADD with NIF_INFO both creates the
        // icon and shows the balloon, so first use needs only one call.
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_INFO;
        // The stock application icon; the balloon shows our title text, and
        // extracting the exe's own icon is not worth a resource dependency.
        nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        copy_wide(&mut nid.szTip, "Unterm");
        copy_wide(&mut nid.szInfoTitle, title);
        copy_wide(&mut nid.szInfo, body);
        nid.dwInfoFlags = NIIF_INFO;

        if *icon_added && Shell_NotifyIconW(NIM_MODIFY, &mut nid) != 0 {
            return Ok(());
        }
        // Either the icon was never added, or Explorer restarted and forgot
        // it (NIM_MODIFY fails then). Adding again covers both.
        if Shell_NotifyIconW(NIM_ADD, &mut nid) != 0 {
            *icon_added = true;
            return Ok(());
        }
        // NIM_ADD can also fail because the shell still thinks the icon
        // exists; clear the stale one and try once more.
        Shell_NotifyIconW(NIM_DELETE, &mut nid);
        if Shell_NotifyIconW(NIM_ADD, &mut nid) != 0 {
            *icon_added = true;
            return Ok(());
        }
        *icon_added = false;
        Err(anyhow!(
            "Shell_NotifyIcon refused the balloon; is a shell running in this session?"
        ))
    }

    /// A NUL-terminated wide string for Win32.
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Fill one of NOTIFYICONDATA's fixed-size text fields, truncating on a
    /// character boundary so a clipped surrogate pair never reaches the
    /// shell, and always leaving room for the terminating NUL.
    fn copy_wide(dest: &mut [u16], s: &str) {
        let mut pos = 0;
        let mut unit_buf = [0u16; 2];
        for ch in s.chars() {
            let units = ch.encode_utf16(&mut unit_buf);
            if pos + units.len() >= dest.len() {
                break;
            }
            dest[pos..pos + units.len()].copy_from_slice(units);
            pos += units.len();
        }
        dest[pos] = 0;
    }
}

#[cfg(all(test, windows))]
mod tests {
    /// A smoke test only: a headless session has no shell to show a balloon,
    /// and that is allowed to come back as Err. What is not allowed is a
    /// panic or a hang.
    #[test]
    fn notify_does_not_panic() {
        let _ = super::notify("test", "body");
    }
}
