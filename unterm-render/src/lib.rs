//! Turning next-core's render commands into something a GPU can draw.
//!
//! next-core could already find a font, shape a run and rasterize a glyph, but
//! nothing joined those to a renderer, so the pixels came from the GUI's font
//! stack and next-core's font modules had no caller at all. This crate is that
//! join, and it is the piece that has to exist before the terminal can draw
//! without WezTerm.

pub mod atlas;
pub mod text;
