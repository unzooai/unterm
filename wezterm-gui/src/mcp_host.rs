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
            "path": rendered.image.path.display().to_string(),
            "width": rendered.image.width,
            "height": rendered.image.height,
            "rows": rendered.image.rows,
            "cols": rendered.image.cols,
            "truncated": rendered.image.truncated,
            "first_row": rendered.image.first_row,
            "session_id": rendered.session_id,
            "renderer": rendered.renderer,
        }))
    }

    fn key_assignments(&self) -> Vec<Value> {
        // Built fresh from the live config so the listing reflects what the
        // user's file actually says right now, named tables included.
        let map = crate::inputmap::InputMap::new(&config::configuration());
        let row = |table: &str, key: &dyn std::fmt::Debug, mods, action| {
            json!({
                "table": table,
                "key": format!("{key:?}"),
                "mods": format!("{mods:?}"),
                "action": format!("{action:?}"),
            })
        };

        let mut out: Vec<Value> = map
            .keys
            .default
            .iter()
            .map(|((key, mods), entry)| row("default", key, mods, &entry.action))
            .collect();

        let mut named: Vec<_> = map.keys.by_name.keys().collect();
        named.sort();
        for name in named {
            if let Some(table) = map.keys.by_name.get(name) {
                out.extend(
                    table
                        .iter()
                        .map(|((key, mods), entry)| row(name, key, mods, &entry.action)),
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The renderer this front end lends the MCP surface really produces a PNG.
    ///
    /// Covers the half of `capture.scrollback` that cannot live in the MCP
    /// crate: without a host there is nothing to render with, and the crate's
    /// own test asserts that honest refusal. This is the other side.
    #[test]
    fn hosting_the_surface_renders_a_scrollback_png() {
        install();
        let host = unterm_engine::mcp_host().expect("host installed");
        // The WezTerm engine wants a mux this test process does not run; the
        // renderer under test is next-core's either way.
        let previous = std::env::var("UNTERM_ENGINE").ok();
        std::env::set_var("UNTERM_ENGINE", "next-core");

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("scrollback.png");
        let mut command = portable_pty::CommandBuilder::new("cmd.exe");
        command.args(["/c", "echo mcp-host-render-check"]);
        let pane = unterm_engine::SessionEngine::create_session(
            &unterm_engine::next_core(),
            unterm_engine::CreateSessionRequest {
                cols: 80,
                rows: 4,
                command_dir: None,
                command: Some(command),
                env: vec![],
                launch_policy: Default::default(),
            },
        )
        .expect("create a next-core session");

        let rendered = host.render_scrollback_png(Some(pane.id), &path, 20, 48);
        let _ = unterm_engine::SessionEngine::destroy_session(
            &unterm_engine::next_core(),
            pane.id,
        );
        match previous {
            Some(value) => std::env::set_var("UNTERM_ENGINE", value),
            None => std::env::remove_var("UNTERM_ENGINE"),
        }
        let rendered = rendered.expect("render the scrollback");

        assert_eq!(rendered["renderer"]["engine"], "next-core");
        assert!(rendered["width"].as_u64().unwrap_or_default() > 0);
        assert!(rendered["height"].as_u64().unwrap_or_default() > 0);
        assert_eq!(
            std::fs::read(&path).expect("read the png")[..8],
            [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        );
    }
}
