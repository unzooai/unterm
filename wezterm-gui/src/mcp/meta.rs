//! `meta.surface` — single-call inventory of everything an agent can drive
//! Unterm with: MCP methods, CLI subcommands, and current keybindings.
//!
//! Designed for two consumers:
//!   1. AI agents that just connected and want one round-trip discovery
//!      of "what can I do here?" without scraping --help output.
//!   2. The Reference tab in Web Settings — the same JSON renders into a
//!      searchable in-app cheatsheet, so users discover the surface area
//!      without having to read external docs.
//!
//! The MCP method list is the source of truth: dispatch in `handler.rs`
//! reads from `MCP_METHODS` so adding a new method in one place is
//! enough. The CLI list is currently a separate maintained array because
//! the GUI binary can't directly introspect the CLI binary's clap tree;
//! discipline (and the matching `unterm-cli reference` self-test) keeps
//! it honest. Keybindings are read live from the current `InputMap`,
//! so they never drift.
//!
//! Keep `summary` strings short and imperative — they show up in the
//! Reference UI as a one-line description next to each entry.
use anyhow::Result;
use serde_json::{json, Value};

pub use unterm_agents::mcp_meta::{CLI_COMMANDS, MCP_METHODS};

const WEZTERM_UNSUPPORTED_METHODS: &[&str] = &["session.env", "session.set_env"];
const NEXT_CORE_UNSUPPORTED_METHODS: &[&str] = &[];

fn engine_unsupported_methods(engine: &str) -> Vec<&'static str> {
    let mut methods = if engine == "next-core" {
        NEXT_CORE_UNSUPPORTED_METHODS.to_vec()
    } else {
        WEZTERM_UNSUPPORTED_METHODS.to_vec()
    };
    methods.sort_unstable();
    methods.dedup();
    methods
}

pub fn engine_capabilities(engine: &str) -> Value {
    let unsupported_methods = engine_unsupported_methods(engine);
    let supported_methods: Vec<_> = MCP_METHODS
        .iter()
        .map(|method| method.name)
        .filter(|name| !unsupported_methods.contains(name))
        .collect();
    let engine_limited_methods = if engine == "next-core" {
        vec![
            json!({
                "name": "screen.search",
                "limitation": "goto updates next-core's logical viewport; real GUI viewport integration comes with the next-core renderer",
            }),
            json!({
                "name": "screen.scroll",
                "limitation": "goto/apply updates next-core's logical viewport; real GUI viewport integration comes with the next-core renderer",
            }),
            json!({
                "name": "cockpit.inbox",
                "limitation": "pane location metadata is synthetic until next-core owns real GUI tabs/windows",
            }),
            json!({
                "name": "capture.scrollback",
                "limitation": "renders styled cell PNGs from next-core scrollback; full theme palette and bold/italic font matching parity are still in progress",
            }),
            json!({
                "name": "instance.focus",
                "limitation": "focuses the current host GUI window; next-core does not own native window lifecycle yet",
            }),
            json!({
                "name": "instance.set_title",
                "limitation": "persists server_info title metadata; native window title ownership stays with the host GUI until next-core owns windows",
            }),
        ]
    } else {
        Vec::new()
    };
    let health_metrics = if engine == "next-core" {
        vec![
            "input_writes",
            "input_bytes",
            "output_chunks",
            "output_bytes",
            "paste_count",
            "paste_text_bytes",
        ]
    } else {
        Vec::new()
    };

    json!({
        "engine": engine,
        "supported_methods": supported_methods,
        "unsupported_methods": unsupported_methods,
        "engine_limited_methods": engine_limited_methods,
        "diagnostics": {
            "health_io_summary": engine == "next-core",
            "launch_context": engine == "next-core",
            "default_shell_launch_decision": true,
            "session_create_launch_decision": true,
            "workspace_restore_launch_plan": true,
            "styled_scrollback_png": engine == "next-core",
            "pty_write_confirmation": true,
            "recording_block_markdown": engine == "next-core",
            "validated_capture_scrollback_pane_ids": true,
            "host_window_bridge": true,
            "native_window_lifecycle": false,
            "health_metrics": health_metrics,
        },
    })
}

/// Read the effective keybindings from the current config and return them
/// as serializable rows. The InputMap is built fresh from `config::configuration()`
/// so the listing reflects the user's actual unterm.lua at call time.
pub fn keybindings_inventory() -> Vec<Value> {
    use crate::inputmap::InputMap;

    let config = config::configuration();
    let map = InputMap::new(&config);
    let mut out = Vec::new();

    // (InputMap.leader is private; we surface it indirectly by listing
    // entries whose mods contain LEADER. The default table iteration
    // below picks those up alongside everything else.)

    // Default key table
    for ((key, mods), entry) in &map.keys.default {
        out.push(json!({
            "table": "default",
            "key": format!("{key:?}"),
            "mods": format!("{mods:?}"),
            "action": format!("{:?}", entry.action),
        }));
    }

    // Named key tables (vi-mode, etc.)
    let mut named_tables: Vec<_> = map.keys.by_name.keys().collect();
    named_tables.sort();
    for table_name in named_tables {
        if let Some(table) = map.keys.by_name.get(table_name) {
            for ((key, mods), entry) in table {
                out.push(json!({
                    "table": table_name,
                    "key": format!("{key:?}"),
                    "mods": format!("{mods:?}"),
                    "action": format!("{:?}", entry.action),
                }));
            }
        }
    }

    out
}

/// `meta.surface` MCP handler.
pub fn surface(_params: &Value) -> Result<Value> {
    let engine = crate::engine::selected_engine_name();
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "engine": engine,
        "engine_capabilities": engine_capabilities(engine),
        "mcp_methods": MCP_METHODS,
        "cli_commands": CLI_COMMANDS,
        "keybindings": keybindings_inventory(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings_at<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
        value[key]
            .as_array()
            .expect("expected array")
            .iter()
            .map(|v| v.as_str().expect("expected string"))
            .collect()
    }

    #[test]
    fn wezterm_capabilities_mark_only_explicit_stubs_unsupported() {
        let caps = engine_capabilities("wezterm");
        let unsupported = strings_at(&caps, "unsupported_methods");

        assert!(unsupported.contains(&"session.env"));
        assert!(unsupported.contains(&"session.set_env"));
        assert!(!unsupported.contains(&"capture.scrollback"));

        let supported = strings_at(&caps, "supported_methods");
        assert!(supported.contains(&"session.input"));
        assert!(supported.contains(&"capture.scrollback"));
        assert_eq!(caps["diagnostics"]["health_io_summary"], false);
        assert_eq!(caps["diagnostics"]["launch_context"], false);
        assert_eq!(caps["diagnostics"]["default_shell_launch_decision"], true);
        assert_eq!(caps["diagnostics"]["session_create_launch_decision"], true);
        assert_eq!(caps["diagnostics"]["workspace_restore_launch_plan"], true);
        assert_eq!(caps["diagnostics"]["pty_write_confirmation"], true);
        assert_eq!(caps["diagnostics"]["recording_block_markdown"], false);
        assert_eq!(
            caps["diagnostics"]["validated_capture_scrollback_pane_ids"],
            true
        );
        assert_eq!(caps["diagnostics"]["host_window_bridge"], true);
        assert_eq!(caps["diagnostics"]["native_window_lifecycle"], false);
    }

    #[test]
    fn next_core_capabilities_expose_styled_scrollback_png_renderer() {
        let caps = engine_capabilities("next-core");
        let unsupported = strings_at(&caps, "unsupported_methods");

        assert!(!unsupported.contains(&"session.set_env"));
        assert!(!unsupported.contains(&"session.env"));
        assert!(!unsupported.contains(&"capture.scrollback"));

        let supported = strings_at(&caps, "supported_methods");
        assert!(supported.contains(&"session.input"));
        assert!(supported.contains(&"session.env"));
        assert!(supported.contains(&"session.set_env"));
        assert!(supported.contains(&"screen.text"));
        assert!(supported.contains(&"capture.scrollback"));

        let limited = caps["engine_limited_methods"]
            .as_array()
            .expect("expected limited method array");
        assert!(limited
            .iter()
            .any(|item| item["name"].as_str() == Some("screen.search")));
        let search = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("screen.search"))
            .expect("screen search limitation");
        assert!(search["limitation"]
            .as_str()
            .expect("limitation text")
            .contains("logical viewport"));
        let scroll = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("screen.scroll"))
            .expect("screen scroll limitation");
        assert!(scroll["limitation"]
            .as_str()
            .expect("limitation text")
            .contains("goto/apply"));
        let inbox = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("cockpit.inbox"))
            .expect("cockpit inbox limitation");
        assert!(inbox["limitation"]
            .as_str()
            .expect("limitation text")
            .contains("synthetic"));
        let capture = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("capture.scrollback"))
            .expect("capture scrollback limitation");
        let capture_limitation = capture["limitation"].as_str().expect("limitation text");
        assert!(capture_limitation.contains("styled cell PNGs"));
        assert!(capture_limitation.contains("bold/italic font matching"));
        let focus = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("instance.focus"))
            .expect("instance focus limitation");
        assert!(focus["limitation"]
            .as_str()
            .expect("limitation text")
            .contains("host GUI window"));
        let title = limited
            .iter()
            .find(|item| item["name"].as_str() == Some("instance.set_title"))
            .expect("instance title limitation");
        assert!(title["limitation"]
            .as_str()
            .expect("limitation text")
            .contains("server_info title metadata"));

        assert_eq!(caps["diagnostics"]["health_io_summary"], true);
        assert_eq!(caps["diagnostics"]["launch_context"], true);
        assert_eq!(caps["diagnostics"]["default_shell_launch_decision"], true);
        assert_eq!(caps["diagnostics"]["session_create_launch_decision"], true);
        assert_eq!(caps["diagnostics"]["workspace_restore_launch_plan"], true);
        assert_eq!(caps["diagnostics"]["styled_scrollback_png"], true);
        assert_eq!(caps["diagnostics"]["pty_write_confirmation"], true);
        assert_eq!(caps["diagnostics"]["recording_block_markdown"], true);
        assert_eq!(
            caps["diagnostics"]["validated_capture_scrollback_pane_ids"],
            true
        );
        assert_eq!(caps["diagnostics"]["host_window_bridge"], true);
        assert_eq!(caps["diagnostics"]["native_window_lifecycle"], false);
        let metrics = strings_at(&caps["diagnostics"], "health_metrics");
        assert!(metrics.contains(&"input_writes"));
        assert!(metrics.contains(&"output_bytes"));
        assert!(metrics.contains(&"paste_count"));
    }
}
