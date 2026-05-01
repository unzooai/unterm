//! `unterm-cli screenshot` — capture the screen via `capture.screen` and
//! optionally copy the resulting PNG to the user-supplied path.

use super::client::McpClient;
use super::output::print_json;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::path::PathBuf;

pub fn run(include_window: bool, output: Option<PathBuf>, json_out: bool) -> Result<()> {
    let mut client = McpClient::connect()?;
    // The MCP `capture.screen` method always captures the whole screen
    // (which includes Unterm's own window when it's on-screen). We expose the
    // `--include-window` flag for parity / future-proofing; today both branches
    // call `capture.screen` and rely on `capture.window` only if the user wants
    // the Unterm window specifically. Here, default behaviour is exclude — but
    // `screencapture` itself can't natively *exclude* a window, so we just
    // document the semantic and use the same call. Callers who care can use
    // the MCP `capture.window` directly.
    let result = client.call(
        "capture.screen",
        json!({ "include_base64": false, "include_window": include_window }),
    )?;

    let mcp_path = result
        .get("image")
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .or_else(|| result.get("path").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow!("capture.screen did not return a path: {}", result))?;

    if let Some(dest) = output.as_ref() {
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        std::fs::copy(mcp_path, dest)?;
    }

    if json_out {
        print_json(&result);
    } else if let Some(dest) = output {
        println!("{}", dest.display());
    } else {
        println!("{}", mcp_path);
    }
    Ok(())
}
