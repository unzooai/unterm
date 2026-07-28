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
pub mod server_info;
pub mod system_proxy;
