//! The window handle the Lua `gui` module used to hand out.
//!
//! The module is gone with the callbacks; the handle itself is how several
//! surfaces still refer to a window, so it stays.

pub mod guiwin;
