//! CLI parity with the Unterm MCP server.
//!
//! Every subcommand in this module talks to the running Unterm GUI's MCP
//! endpoint at `127.0.0.1:19876` (line-delimited JSON-RPC 2.0, authed via the
//! UUID at `~/.unterm/auth_token`). The intent is "anything you can do via
//! MCP, you can do from the shell".

mod client;
pub mod i18n;
mod lang;
mod output;
mod proxy;
mod screenshot;
mod session;
mod sessions;
mod settings;
mod theme;

pub use lang::LangCommand;
pub use proxy::ProxyCommand;
pub use session::SessionCommand;
pub use sessions::SessionsCommand;
pub use settings::SettingsCommand;
pub use theme::ThemeCommand;

use anyhow::Result;
use std::path::PathBuf;

pub fn run_proxy(cmd: ProxyCommand, json_out: bool) -> Result<()> {
    proxy::run(cmd, json_out)
}

pub fn run_theme(cmd: ThemeCommand, json_out: bool) -> Result<()> {
    theme::run(cmd, json_out)
}

pub fn run_session(cmd: SessionCommand, json_out: bool) -> Result<()> {
    session::run(cmd, json_out)
}

pub fn run_sessions(cmd: SessionsCommand, json_out: bool) -> Result<()> {
    sessions::run(cmd, json_out)
}

pub fn run_screenshot(
    include_window: bool,
    output: Option<PathBuf>,
    json_out: bool,
) -> Result<()> {
    screenshot::run(include_window, output, json_out)
}

pub fn run_settings(cmd: SettingsCommand) -> Result<()> {
    settings::run(cmd)
}

pub fn run_lang(cmd: LangCommand, json_out: bool) -> Result<()> {
    lang::run(cmd, json_out)
}

/// Apply the optional `--lang <code>` flag for the lifetime of this process.
pub fn apply_transient_lang(code: Option<&str>) {
    if let Some(c) = code {
        let _ = i18n::set_locale_transient(c);
    }
}
