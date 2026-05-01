//! `unterm-cli theme ...` — list/switch the GUI's preset theme by writing
//! `~/.unterm/theme.json`. The running GUI picks it up via its file watcher.
//!
//! No MCP method is involved: theme.json is the single source of truth and
//! `wezterm-gui/src/overlay/theme_selector.rs` reads/writes the same shape.

use super::i18n;
use super::output::print_json;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde_json::json;

/// Keep this list in sync with `wezterm-gui/src/overlay/theme_selector.rs::theme_presets()`.
const PRESETS: &[ThemePreset] = &[
    ThemePreset {
        id: "standard",
        name: "Standard",
        scheme: "Catppuccin Mocha",
        desc: "Balanced dark terminal style",
    },
    ThemePreset {
        id: "midnight",
        name: "Midnight",
        scheme: "Tokyo Night",
        desc: "Low-glare blue-black workspace",
    },
    ThemePreset {
        id: "daylight",
        name: "Daylight",
        scheme: "Builtin Solarized Light",
        desc: "Readable light mode for bright rooms",
    },
    ThemePreset {
        id: "classic",
        name: "Classic",
        scheme: "Builtin Tango Dark",
        desc: "Plain high-contrast terminal colors",
    },
];

struct ThemePreset {
    id: &'static str,
    name: &'static str,
    scheme: &'static str,
    desc: &'static str,
}

#[derive(Debug, Parser, Clone)]
pub struct ThemeCommand {
    #[command(subcommand)]
    pub sub: ThemeSubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum ThemeSubCommand {
    /// List all theme presets.
    List,
    /// Switch to the named theme preset.
    #[command(alias = "set")]
    Switch {
        /// Preset id (e.g. `standard`, `midnight`, `daylight`, `classic`).
        name: String,
    },
}

pub fn run(cmd: ThemeCommand, json_out: bool) -> Result<()> {
    match cmd.sub {
        ThemeSubCommand::List => {
            let active = read_active_theme();
            if json_out {
                let arr: Vec<_> = PRESETS
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "color_scheme": p.scheme,
                            "description": p.desc,
                            "active": Some(p.id) == active.as_deref(),
                        })
                    })
                    .collect();
                print_json(&json!({ "active": active, "presets": arr }));
            } else {
                let unset = i18n::t("cli.theme.unset");
                let active_name = active.clone().unwrap_or(unset);
                println!(
                    "{}",
                    i18n::t_args("cli.theme.active", &[("name", &active_name)])
                );
                println!();
                println!(
                    "{:<2} {:<10} {:<14} {:<28} {}",
                    "",
                    i18n::t("cli.theme.head.id"),
                    i18n::t("cli.theme.head.name"),
                    i18n::t("cli.theme.head.scheme"),
                    i18n::t("cli.theme.head.desc")
                );
                for p in PRESETS {
                    let marker = if Some(p.id) == active.as_deref() { "*" } else { " " };
                    let translated_name =
                        i18n::t(&format!("theme.preset.{}.name", p.id));
                    let translated_desc =
                        i18n::t(&format!("theme.preset.{}.desc", p.id));
                    println!(
                        "{:<2} {:<10} {:<14} {:<28} {}",
                        marker, p.id, translated_name, p.scheme, translated_desc
                    );
                }
            }
        }
        ThemeSubCommand::Switch { name } => {
            let preset = PRESETS
                .iter()
                .find(|p| p.id.eq_ignore_ascii_case(&name))
                .ok_or_else(|| {
                    anyhow!(
                        "{}",
                        i18n::t_args("cli.theme.unknown", &[("name", &name)])
                    )
                })?;
            write_theme(preset)?;
            if json_out {
                print_json(&json!({
                    "switched": true,
                    "id": preset.id,
                    "name": preset.name,
                    "color_scheme": preset.scheme,
                }));
            } else {
                println!(
                    "{}",
                    i18n::t_args(
                        "cli.theme.switched",
                        &[
                            ("id", preset.id),
                            ("name", preset.name),
                            ("scheme", preset.scheme),
                        ]
                    )
                );
            }
        }
    }
    Ok(())
}

fn theme_config_path() -> Result<std::path::PathBuf> {
    Ok(dirs_next::home_dir()
        .ok_or_else(|| anyhow!("could not resolve home directory"))?
        .join(".unterm")
        .join("theme.json"))
}

fn read_active_theme() -> Option<String> {
    let path = theme_config_path().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value
        .get("theme")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn write_theme(preset: &ThemePreset) -> Result<()> {
    let path = theme_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let value = json!({
        "theme": preset.id,
        "name": preset.name,
        "color_scheme": preset.scheme,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&value)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
