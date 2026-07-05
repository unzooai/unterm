//! End-to-end cold-start phase timing.
//!
//! `init()` runs at the very top of main(); `mark()` logs elapsed-since-main
//! at each phase boundary, so one log file shows where startup time goes.
//! The pre-main segment (exe/DLL image load before main ran) is captured on
//! Windows by diffing the kernel's process-creation timestamp against the
//! clock at `init()`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static MAIN_ENTRY: OnceLock<Instant> = OnceLock::new();
static PRE_MAIN_MS: OnceLock<u64> = OnceLock::new();
static FIRST_PAINT_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    MAIN_ENTRY.get_or_init(Instant::now);
    if let Some(ms) = pre_main_ms() {
        PRE_MAIN_MS.set(ms).ok();
    }
}

#[cfg(windows)]
fn pre_main_ms() -> Option<u64> {
    use std::mem::MaybeUninit;
    use winapi::shared::minwindef::FILETIME;
    use winapi::um::processthreadsapi::{GetCurrentProcess, GetProcessTimes};
    use winapi::um::sysinfoapi::GetSystemTimeAsFileTime;

    fn to_u64(ft: &FILETIME) -> u64 {
        ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
    }

    unsafe {
        let mut creation = MaybeUninit::<FILETIME>::zeroed();
        let mut exit = MaybeUninit::<FILETIME>::zeroed();
        let mut kernel = MaybeUninit::<FILETIME>::zeroed();
        let mut user = MaybeUninit::<FILETIME>::zeroed();
        if GetProcessTimes(
            GetCurrentProcess(),
            creation.as_mut_ptr(),
            exit.as_mut_ptr(),
            kernel.as_mut_ptr(),
            user.as_mut_ptr(),
        ) == 0
        {
            return None;
        }
        let mut now = MaybeUninit::<FILETIME>::zeroed();
        GetSystemTimeAsFileTime(now.as_mut_ptr());
        // FILETIME is in 100ns units
        Some(to_u64(&now.assume_init()).saturating_sub(to_u64(&creation.assume_init())) / 10_000)
    }
}

#[cfg(not(windows))]
fn pre_main_ms() -> Option<u64> {
    None
}

/// Log once, right after the logger is ready.
pub fn log_pre_main() {
    if let Some(ms) = PRE_MAIN_MS.get() {
        // debug: CLI invocations (--version, --help, cli subcommands) run
        // through main() too and must not print timing noise to stderr.
        // Measure with WEZTERM_LOG=unterm::startup_timing=debug.
        log::debug!("startup-timing: pre-main (exe/dll load): {ms}ms");
    }
}

pub fn mark(label: &str) {
    if let Some(start) = MAIN_ENTRY.get() {
        log::debug!(
            "startup-timing: {label}: {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}

/// Called from paint_impl on every frame; logs only the first one.
pub fn mark_first_paint() {
    if !FIRST_PAINT_LOGGED.swap(true, Ordering::Relaxed) {
        mark("first frame painted");
    }
}
