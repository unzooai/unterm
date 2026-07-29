//! A terminal that does not use WezTerm.
//!
//! next-core runs the shell and owns the screen; unterm-render draws it; winit
//! provides the window. This binary exists alongside `unterm` rather than
//! replacing it, so the working terminal keeps working while this one grows.

mod args;
mod cockpit;
mod confirm;
mod copy_mode;
mod directory;
mod fonts;
mod ime;
mod keys;
mod links;
mod mcp_host;
mod mouse;
mod palette;
mod panes;
mod scroll;
mod scrollbar;
mod search;
mod shape;
mod statusbar;
mod tabbar;
mod select;
mod terminal;
mod window;

use unterm_engine::next_core::config;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().filter_or("UNTERM_LOG", "info"),
    )
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
    unterm_services::settings::set_current(&config);

    // The agent-facing API. next-core answers everything about sessions and
    // screens; this app answers the two things that need a font stack and a
    // key table. Installed before the window opens so an agent connecting
    // early finds a working surface rather than a half-built one.
    unterm_engine::install_next_core_provider();
    mcp_host::install();
    let (port, token) = unterm_mcp::start_mcp_server();
    log::info!("MCP server listening on 127.0.0.1:{port}");

    // The settings UI, on the same token. `unterm-cli settings` opens it.
    let settings_port = unterm_settings::start_web_settings_server(token);
    log::info!("settings UI listening on 127.0.0.1:{settings_port}");

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = window::App::new(&config)?;
    app.set_start_directory(args.cwd);
    event_loop.run_app(&mut app)?;
    Ok(())
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

    log::info!(
        "converted {} to {}",
        old_path.display(),
        new_path.display()
    );
    for left in &migration.unconverted {
        log::info!("  not converted: {left:?}");
    }
    Ok(())
}
