//! GUI adapter for the engine-neutral terminal layer.
//!
//! The neutral traits and next-core implementation live in `unterm-engine`.
//! This module keeps the current WezTerm adapter available to GUI callers while
//! letting product services migrate away from WezTerm internals.

pub mod wezterm;

#[allow(unused_imports)]
pub use unterm_engine::{
    next_core, CellStyle, CreateSessionRequest, CursorSnapshot, DirtyRows, InputEngine,
    PaneDimensions, ScreenEngine, ScreenLine, ScreenSearchMatch, ScreenSnapshot,
    ScrollbackTextRequest, ScrollbackTextSnapshot, SessionActivitySnapshot, SessionEngine,
    SessionSnapshot, ShellSnapshot, SplitDirection, SplitSessionRequest, StyledCell, StyledColor,
    StyledScreenLine, StyledScreenSnapshot, TerminalEngine,
};

pub type CurrentTerminalEngine = wezterm::WezTermEngine;

pub fn current() -> CurrentTerminalEngine {
    wezterm::WezTermEngine
}
