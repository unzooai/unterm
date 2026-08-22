//! The terminal, as a program.
//!
//! Four lines, on purpose. Everything is in `lib.rs`, because a bin target
//! that carries its own unit tests collides with itself: cargo calls both
//! the terminal and its test harness `kind=bin, name=unterm` and uplifts
//! both to `target/debug/unterm.exe`, so `CARGO_BIN_EXE_unterm` named
//! whichever linked last. `tests/version_exit.rs` failed about half the
//! time on that, asking a libtest harness to print a version.
// A GUI, not a console program: without this, every Explorer/Start-menu
// launch on Windows drags a black console window along with the terminal.
// Console-launched invocations reattach in main() so `unterm --version`
// still prints where it was typed.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    unterm_app::main()
}
