//! CLI parity with the Unterm MCP server.
//!
//! Every subcommand in this module talks to the running Unterm GUI's MCP
//! endpoint at `127.0.0.1:19876` (line-delimited JSON-RPC 2.0, authed via the
//! UUID at `~/.unterm/auth_token`). The intent is "anything you can do via
//! MCP, you can do from the shell".

mod agent;
mod client;
pub mod i18n;
mod lang;
mod mcp_stdio;
mod output;
mod profile;
mod proxy;
mod reference;
mod screenshot;
mod scrollback;
mod session;
mod setup_ai;
mod sessions;
mod settings;
mod theme;
mod upload;

pub use agent::AgentCommand;
pub use lang::LangCommand;
pub use profile::ProfileCommand;
pub use proxy::ProxyCommand;
pub use reference::ReferenceCommand;
pub use scrollback::ScrollbackCommand;
pub use session::SessionCommand;
pub use sessions::SessionsCommand;
pub use setup_ai::SetupAiCommand;
pub use settings::SettingsCommand;
pub use theme::ThemeCommand;
pub use upload::UploadCommand;

use anyhow::Result;
use std::path::PathBuf;

pub fn run_proxy(cmd: ProxyCommand, json_out: bool) -> Result<()> {
    proxy::run(cmd, json_out)
}

pub fn run_profile(cmd: ProfileCommand, json_out: bool) -> Result<()> {
    profile::run(cmd, json_out)
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

pub fn run_agent(cmd: AgentCommand, json_out: bool) -> Result<()> {
    agent::run(cmd, json_out)
}

pub fn run_upload(cmd: UploadCommand, json_out: bool) -> Result<()> {
    upload::run(cmd, json_out)
}

pub fn run_scrollback(cmd: ScrollbackCommand, json_out: bool) -> Result<()> {
    scrollback::run(cmd, json_out)
}

pub fn run_reference(cmd: ReferenceCommand, json_out: bool) -> Result<()> {
    reference::run(cmd, json_out)
}

pub fn run_setup_ai(cmd: SetupAiCommand, json_out: bool) -> Result<()> {
    setup_ai::run(cmd, json_out)
}

pub fn run_mcp_stdio() -> Result<()> {
    mcp_stdio::run()
}

/// Apply the optional `--lang <code>` flag for the lifetime of this process.
pub fn apply_transient_lang(code: Option<&str>) {
    if let Some(c) = code {
        let _ = i18n::set_locale_transient(c);
    }
}
