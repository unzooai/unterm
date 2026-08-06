//! Notices when the user switches input sources.
//!
//! Switching away from an input method mid-composition is how the orphan
//! preedit is born: the old method never ends its composition, no `Ime`
//! event ever arrives, and the stranded marked text swallows every editing
//! key from then on. macOS announces the switch on the distributed
//! notification center — `kTISNotifySelectedKeyboardInputSourceChanged` —
//! so listen there, raise a flag, and let the event loop's next tick clear
//! the ghost before anyone types into it.

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

static SOURCE_CHANGED: AtomicBool = AtomicBool::new(false);

/// Whether the input source changed since the last time somebody asked.
/// Reading takes the flag down, so one switch is answered once.
pub fn input_source_changed() -> bool {
    SOURCE_CHANGED.swap(false, Ordering::AcqRel)
}

extern "C" fn on_source_changed(
    _center: *mut c_void,
    _observer: *mut c_void,
    _name: *const c_void,
    _object: *const c_void,
    _user_info: *const c_void,
) {
    SOURCE_CHANGED.store(true, Ordering::Release);
}

/// Register the observer. Call once, from the main thread, before the event
/// loop runs — the distributed center delivers on the main run loop, which
/// winit drives.
pub fn start() {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;

    type CFNotificationCenterRef = *mut c_void;
    #[allow(non_snake_case)]
    extern "C" {
        fn CFNotificationCenterGetDistributedCenter() -> CFNotificationCenterRef;
        fn CFNotificationCenterAddObserver(
            center: CFNotificationCenterRef,
            observer: *const c_void,
            callback: extern "C" fn(
                *mut c_void,
                *mut c_void,
                *const c_void,
                *const c_void,
                *const c_void,
            ),
            name: *const c_void,
            object: *const c_void,
            suspension_behavior: isize,
        );
    }
    // Delivery may coalesce while the app naps; the latest state is the only
    // one we act on, so coalescing loses nothing.
    const DELIVER_AND_COALESCE: isize = 4; // CFNotificationSuspensionBehaviorCoalesce
    let name = CFString::new("com.apple.Carbon.TISNotifySelectedKeyboardInputSourceChanged");
    // SAFETY: the center is process-global; the observer pointer is only an
    // identity token (we never remove the observer, it lives as long as the
    // process); the name CFString outlives the call and the center copies it.
    unsafe {
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetDistributedCenter(),
            std::ptr::null(),
            on_source_changed,
            name.as_concrete_TypeRef() as *const c_void,
            std::ptr::null(),
            DELIVER_AND_COALESCE,
        );
    }
}
