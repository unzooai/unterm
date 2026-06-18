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
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "mcp_methods": MCP_METHODS,
        "cli_commands": CLI_COMMANDS,
        "keybindings": keybindings_inventory(),
    }))
}
