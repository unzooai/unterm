//! A terminal that does not use WezTerm.
//!
//! next-core runs the shell and owns the screen; unterm-render draws it; winit
//! provides the window. This binary exists alongside `unterm` rather than
//! replacing it, so the working terminal keeps working while this one grows.

mod fonts;
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

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = window::App::new(&config)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
