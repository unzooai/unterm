//! Answers macOS when it says "open these".
//!
//! Finder's right-click, a URL handed to the app, a folder dropped on the
//! Dock icon: they all arrive as `application:openURLs:` on the app
//! delegate — a method winit's delegate does not implement, so for a while
//! the extension asked, the app activated, and nothing happened. The method
//! is added to winit's own delegate class at runtime; the paths land in a
//! queue the event loop drains on its next tick.

#![cfg(target_os = "macos")]

use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{class, msg_send, sel};
use std::path::PathBuf;
use std::sync::Mutex;

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Paths macOS asked us to open since the last look.
pub fn drain() -> Vec<PathBuf> {
    std::mem::take(&mut *PENDING.lock().unwrap())
}

extern "C" fn open_urls(_this: *mut AnyObject, _cmd: Sel, _app: *mut AnyObject, urls: *mut AnyObject) {
    // SAFETY: AppKit hands us an NSArray<NSURL>; we only read it, on the
    // main thread, for the duration of this call.
    unsafe {
        let count: usize = msg_send![urls, count];
        let mut pending = PENDING.lock().unwrap();
        for index in 0..count {
            let url: *mut AnyObject = msg_send![urls, objectAtIndex: index];
            let is_file: bool = msg_send![url, isFileURL];
            if !is_file {
                continue;
            }
            let path: *mut AnyObject = msg_send![url, path];
            if path.is_null() {
                continue;
            }
            let utf8: *const std::os::raw::c_char = msg_send![path, UTF8String];
            if utf8.is_null() {
                continue;
            }
            let text = std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned();
            pending.push(PathBuf::from(text));
        }
    }
}

/// Teach the running application delegate to take openURLs. Call once, on
/// the main thread, after the event loop (and so the delegate) exists.
pub fn install() {
    // SAFETY: NSApp and its delegate exist once the event loop is built; we
    // add one method to the delegate's class, which is winit's own private
    // delegate class, not a shared framework one.
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        if delegate.is_null() {
            log::warn!("no application delegate to teach openURLs to");
            return;
        }
        let class: *const AnyClass = msg_send![delegate, class];
        let class = &*class;
        let sel = sel!(application:openURLs:);
        let types = std::ffi::CString::new("v@:@@").unwrap();
        let added = objc2::ffi::class_addMethod(
            class as *const _ as *mut _,
            sel,
            std::mem::transmute::<
                extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
                unsafe extern "C-unwind" fn(),
            >(open_urls),
            types.as_ptr(),
        );
        if !added.as_bool() {
            log::warn!("could not add openURLs handler (already present?)");
        }
        // AppKit decided what this delegate can answer when the delegate was
        // set -- before our method existed. A launch delivery still finds it,
        // a delivery to the running app does not. Setting the delegate again
        // makes AppKit look again.
        let () = msg_send![app, setDelegate: std::ptr::null_mut::<AnyObject>()];
        let () = msg_send![app, setDelegate: delegate];
    }
}

/// One line into `<state>/open.log`, so a wrong-folder report comes with
/// the folder that was actually delivered.
pub fn trace(message: &str) {
    let Some(dir) = unterm_protocol::state_dir() else {
        return;
    };
    use std::io::Write as _;
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("open.log"))
    {
        let _ = writeln!(file, "{stamp} {message}");
    }
}
