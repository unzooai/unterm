//! `unterm-cli screenshot` — capture via Unterm's MCP server.
//!
//! Three modes:
//!   (default)        whole screen via `capture.screen`
//!   --scrollback     in-terminal long screenshot: the pane's ENTIRE
//!                    scrollback rendered to one tall PNG (`capture.scrollback`)
//!   --scroll-app /-title /-pid
//!                    out-of-terminal long screenshot: scroll + stitch another
//!                    app's window (`capture.window_scroll`, macOS)

use super::client::McpClient;
use super::output::print_json;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::path::PathBuf;

#[derive(Default)]
pub struct ScreenshotArgs {
    pub include_window: bool,
    /// Capture only Unterm's own window via the server's CGWindowID.
    /// Independent of foreground state — no screencapture(1) framing.
    pub self_window: bool,
    pub output: Option<PathBuf>,
    // in-terminal long screenshot
    pub scrollback: bool,
    pub pane: Option<u64>,
    pub max_rows: Option<u64>,
    pub dpi: Option<u64>,
    // external-window long screenshot
    pub scroll_app: Option<String>,
    pub scroll_title: Option<String>,
    pub scroll_pid: Option<u64>,
    pub max_frames: Option<u64>,
}

pub fn run(args: ScreenshotArgs, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;

    let external = args.scroll_app.is_some()
        || args.scroll_title.is_some()
        || args.scroll_pid.is_some();

    let result = if args.self_window {
        // `capture.window` with no filters defaults to the server's own pid
        // → captures Unterm's window via CGWindowList without requiring
        // foreground or any UI framing.
        client.call("capture.window", json!({ "include_base64": false }))?
    } else if args.scrollback {
        let mut params = json!({});
        if let Some(id) = args.pane {
            params["id"] = json!(id);
        }
        if let Some(n) = args.max_rows {
            params["max_rows"] = json!(n);
        }
        if let Some(n) = args.dpi {
            params["dpi"] = json!(n);
        }
        client.call("capture.scrollback", params)?
    } else if external {
        let mut params = json!({});
        if let Some(app) = &args.scroll_app {
            params["app"] = json!(app);
        }
        if let Some(title) = &args.scroll_title {
            params["title"] = json!(title);
        }
        if let Some(pid) = args.scroll_pid {
            params["pid"] = json!(pid);
        }
        if let Some(n) = args.max_frames {
            params["max_frames"] = json!(n);
        }
        client.call("capture.window_scroll", params)?
    } else {
        // The MCP `capture.screen` method always captures the whole screen
        // (which includes Unterm's own window when it's on-screen). We expose
        // the `--include-window` flag for parity / future-proofing.
        client.call(
            "capture.screen",
            json!({ "include_base64": false, "include_window": args.include_window }),
        )?
    };

    let mcp_path = result
        .get("image")
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .or_else(|| result.get("path").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("capture did not return a path: {}", result))?;

    if let Some(dest) = args.output.as_ref() {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        std::fs::copy(mcp_path, dest)?;
    }

    if json_out {
        print_json(&result);
    } else {
        if let Some(hint) = result.get("hint").and_then(|v| v.as_str()) {
            eprintln!("hint: {hint}");
        }
        if let Some(dest) = args.output {
            println!("{}", dest.display());
        } else {
            println!("{}", mcp_path);
        }
    }
    Ok(())
}
