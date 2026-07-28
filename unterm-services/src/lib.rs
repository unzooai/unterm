//! Unterm's product services, independent of any front end.
//!
//! These were inside the GUI crate for no reason other than history: the
//! instance registry, the agent cockpit, proxy control, command suggestion.
//! None of them draw anything or know what a window is.

pub mod clash_api;
pub mod cockpit;
pub mod ghost_text;
pub mod i18n;
pub mod launch_env;
pub mod recording;
pub mod scrollback_options;
pub mod server_info;
pub mod env_names;
pub mod settings;

/// What this build calls itself, stamped into recordings and reported to
/// agents. Read from the crate rather than from a config, which is where it
/// used to live for no reason other than history.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod system_proxy;
