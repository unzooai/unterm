//! A terminal that does not use WezTerm.
//!
//! next-core runs the shell and owns the screen; unterm-render draws it; winit
//! provides the window. This binary exists alongside `unterm` rather than
//! replacing it, so the working terminal keeps working while this one grows.

mod args;
mod background;
mod charselect;
mod chrome;
mod chrome_font;
mod clipboard;
mod cockpit;
mod composer;
mod confirm;
mod copy_mode;
mod dir_jump;
mod directory;
mod fleet;
mod fonts;
mod ghost;
mod git;
mod ime;
mod keys;
mod links;
mod mcp_host;
mod mouse;
mod palette;
mod panes;
mod paneselect;
mod scroll;
mod scrollbar;
mod search;
mod session_restore;
mod select;
mod shape;
mod sidebar;
mod statsbar;
mod statusbar;
mod terminal;
mod theme;
mod topbar;
mod tree;
mod ui_tokens;
mod window;
mod window_buttons;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().filter_or("UNTERM_LOG", "info"))
        .init();

    let args = args::parse(std::env::args().skip(1));
    for argument in &args.unrecognised {
        log::warn!("ignoring unrecognised argument {argument:?}");
    }

    // A Lua config from an older build, converted once so the settings the
    // user wrote are not silently dropped on upgrade.
    if let Err(err) = migrate_old_config() {
        log::warn!("could not convert the previous config: {err:#}");
    }

    // The same declarative config the rest of the product reads. Unreadable or
    // absent, we start on defaults rather than refusing to open: a terminal you
    // cannot open is no way to fix your config.
    let (config, errors) = unterm_services::settings::load(args.config.clone());
    for error in &errors {
        log::warn!("config line {}: {}", error.line, error.message);
    }
    apply_path_append(&config);
    if let Ok(Some(milliseconds)) = config.int_of("stats.refresh_ms") {
        if let Ok(milliseconds) = u64::try_from(milliseconds) {
            statsbar::set_refresh_ms(milliseconds);
        }
    }
    unterm_services::settings::set_current(&config);
    unterm_engine::next_core::NextCoreEngine::set_new_session_scrollback_lines(
        unterm_services::settings::scrollback_lines(&config),
    );
    if let Some(profile) = args.profile.as_deref() {
        std::env::set_var("UNTERM_STARTUP_PROFILE", profile);
    }

    // The agent-facing API. next-core answers everything about sessions and
    // screens; this app answers the two things that need a font stack and a
    // key table. Installed before the window opens so an agent connecting
    // early finds a working surface rather than a half-built one.
    unterm_engine::install_next_core_provider();
    mcp_host::install();
    let (port, token) = unterm_mcp::start_mcp_server_with_version(env!("CARGO_PKG_VERSION"));
    log::info!("MCP server listening on 127.0.0.1:{port}");

    // The settings UI, on the same token. `unterm-cli settings` opens it.
    let settings_port = unterm_settings::start_web_settings_server(token);
    log::info!("settings UI listening on 127.0.0.1:{settings_port}");

    // The update check, which existed but was never started: without this
    // call the Updates page always said "never checked" and nobody was told
    // a release had shipped.
    unterm_settings::update_check::start_background_poller();

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = window::App::new(&config)?;
    // A plain launch reopens where the last one closed; naming a directory
    // or a command on the line asks for something specific instead.
    if args.cwd.is_none() && args.command.is_empty() {
        if let Some(saved) = session_restore::load() {
            app.set_restore(saved);
        }
    }
    app.set_start_directory(args.cwd);
    app.set_start_command(args.command);
    let run_result = event_loop.run_app(&mut app);
    let shutdown = unterm_services::server_info::unregister_current_instance();
    if !shutdown.errors.is_empty() {
        log::warn!(
            "could not fully unregister instance during shutdown: {}",
            shutdown.errors.join("; ")
        );
    }
    run_result?;
    Ok(())
}

/// Extend the environment inherited by every new pane.
///
/// The 0.57 Windows config did this in Lua. Keeping it at process startup
/// makes shell discovery, agent discovery and the shells themselves see one
/// identical PATH.
fn apply_path_append(config: &unterm_engine::next_core::config::Config) {
    let mut additions: Vec<std::path::PathBuf> = config
        .list_of("path_append")
        .ok()
        .flatten()
        .into_iter()
        .flatten()
        .filter_map(|value| match value {
            unterm_engine::next_core::config::Value::Str(path) => {
                Some(std::path::PathBuf::from(path))
            }
            _ => None,
        })
        .collect();

    #[cfg(windows)]
    {
        let mut standard = vec![
            std::path::PathBuf::from(r"C:\Program Files\nodejs"),
            std::path::PathBuf::from(r"C:\Strawberry\perl\bin"),
        ];
        if let Some(appdata) = std::env::var_os("APPDATA") {
            standard.push(std::path::PathBuf::from(appdata).join("npm"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            standard.push(std::path::PathBuf::from(home).join(".bun").join("bin"));
        }
        additions.extend(standard.into_iter().filter(|path| path.is_dir()));
    }

    let current = std::env::var_os("PATH").unwrap_or_default();
    let paths = merged_path(std::env::split_paths(&current), additions);
    match std::env::join_paths(paths) {
        Ok(path) => std::env::set_var("PATH", path),
        Err(error) => log::warn!("could not extend PATH: {error}"),
    }
}

fn merged_path(
    current: impl IntoIterator<Item = std::path::PathBuf>,
    additions: impl IntoIterator<Item = std::path::PathBuf>,
) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = current.into_iter().collect();
    for addition in additions {
        let duplicate = paths.iter().any(|known| {
            if cfg!(windows) {
                known
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&addition.to_string_lossy())
            } else {
                known == &addition
            }
        });
        if !duplicate {
            paths.push(addition);
        }
    }
    paths
}

/// Convert a Lua config from an older build, once.
///
/// The previous terminal read `unterm.lua`; this one reads `unterm.conf`. A
/// user upgrading has settings written in the old format and no reason to
/// know the format changed, so the first run converts what it can and leaves
/// the original alone -- a conversion that eats the file it read is one
/// nobody can check.
///
/// Only when there is no new-format config yet: converting over one the user
/// has since written would undo their work.
fn migrate_old_config() -> anyhow::Result<()> {
    use unterm_engine::next_core::config_migrate;

    let Some(home) = dirs_next::home_dir() else {
        return Ok(());
    };
    let new_path = home.join(".unterm").join("unterm.conf");
    if new_path.exists() {
        return Ok(());
    }

    let candidates = [
        // This was the normal Windows/Linux config location used by the
        // WezTerm-based releases. Missing it made an upgrade silently fall
        // back to the new bundled font and spacing even though the user's
        // v0.57.4 configuration was still present.
        home.join(".config").join("unterm").join("unterm.lua"),
        home.join(".unterm").join("unterm.lua"),
        home.join(".unterm.lua"),
        home.join(".wezterm.lua"),
    ];
    let Some(old_path) = candidates.into_iter().find(|path| path.exists()) else {
        return Ok(());
    };

    let source = std::fs::read_to_string(&old_path)?;
    let migration = config_migrate::migrate_lua(&source);
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&new_path, &migration.text)?;

    log::info!("converted {} to {}", old_path.display(), new_path.display());
    for left in &migration.unconverted {
        log::info!("  not converted: {left:?}");
    }
    Ok(())
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn path_extensions_keep_order_and_do_not_duplicate_entries() {
        let current = [
            std::path::PathBuf::from("first"),
            std::path::PathBuf::from("second"),
        ];
        let merged = merged_path(
            current,
            [
                std::path::PathBuf::from("second"),
                std::path::PathBuf::from("third"),
            ],
        );

        assert_eq!(
            merged,
            [
                std::path::PathBuf::from("first"),
                std::path::PathBuf::from("second"),
                std::path::PathBuf::from("third"),
            ]
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_path_deduplication_ignores_case() {
        let merged = merged_path(
            [std::path::PathBuf::from(r"C:\Tools")],
            [std::path::PathBuf::from(r"c:\tools")],
        );
        assert_eq!(merged.len(), 1);
    }
}
