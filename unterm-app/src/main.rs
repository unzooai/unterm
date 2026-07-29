//! A terminal that does not use WezTerm.
//!
//! next-core runs the shell and owns the screen; unterm-render draws it; winit
//! provides the window. This binary exists alongside `unterm` rather than
//! replacing it, so the working terminal keeps working while this one grows.

mod confirm;
mod fonts;
mod ime;
mod keys;
mod links;
mod mcp_host;
mod mouse;
mod panes;
mod scroll;
mod scrollbar;
mod search;
mod shape;
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

    // The same declarative config the rest of the product reads. Unreadable or
    // absent, we start on defaults rather than refusing to open: a terminal you
    // cannot open is no way to fix your config.
    let config = std::env::args()
        .nth(1)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|source| config::parse(&source).ok())
        .unwrap_or_default();

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
    event_loop.run_app(&mut app)?;
    Ok(())
}
