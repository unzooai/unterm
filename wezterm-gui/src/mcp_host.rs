//! What this front end answers for the MCP surface.
//!
//! Three things the surface cannot do for itself: render a pane's scrollback
//! to a PNG, capture another application's window, and say what the keys do.
//! Each needs something only a front end has -- the font stack, an OS window
//! API, the key table -- so they come through here.

use anyhow::Result;
use serde_json::{json, Value};
use unterm_engine::McpHost;

pub struct GuiMcpHost;

pub fn install() {
    unterm_engine::set_mcp_host(&GuiMcpHost);
}

impl McpHost for GuiMcpHost {
    fn render_scrollback_png(
        &self,
        pane_id: Option<usize>,
        path: &std::path::Path,
        max_rows: usize,
        dpi: usize,
    ) -> Result<Value> {
        use crate::engine::ScrollbackImageEngine;

        let opts = unterm_services::scrollback_options::ScrollbackPngOptions { max_rows, dpi };
        let rendered = crate::engine::current().render_scrollback_png(pane_id, path, &opts)?;
        Ok(json!({
            "path": rendered.image.path,
            "width": rendered.image.width,
            "height": rendered.image.height,
            "rows": rendered.image.rows,
            "truncated": rendered.image.truncated,
            "session_id": rendered.session_id,
            "renderer": rendered.renderer,
        }))
    }

    fn key_assignments(&self) -> Vec<Value> {
        let config = config::configuration();
        crate::inputmap::InputMap::new(&config)
            .keys
            .default
            .iter()
            .map(|((key, mods), entry)| {
                json!({
                    "key": format!("{key:?}"),
                    "mods": format!("{mods:?}"),
                    "action": format!("{:?}", entry.action),
                })
            })
            .collect()
    }
}
