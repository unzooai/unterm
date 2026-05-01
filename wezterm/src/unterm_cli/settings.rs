//! `unterm settings open` — print the local Web Settings URL and open it in
//! the system browser.
//!
//! The bound port lives in `~/.unterm/server.json` (written by the GUI's
//! HTTP server when it binds). We don't require an MCP round trip; we just
//! read the file and shell out to the platform-native opener.

use super::client::ServerEndpoint;
use super::i18n;
use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Debug, Parser, Clone)]
pub struct SettingsCommand {
    #[command(subcommand)]
    pub sub: SettingsSubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum SettingsSubCommand {
    /// Print the Web Settings URL and open it in the system browser.
    Open {
        /// Just print the URL, do not launch a browser.
        #[arg(long)]
        print_only: bool,
    },
}

pub fn run(cmd: SettingsCommand) -> Result<()> {
    match cmd.sub {
        SettingsSubCommand::Open { print_only } => {
            let info = ServerEndpoint::resolve()?;
            if info.http_port == 0 {
                return Err(anyhow!("{}", i18n::t("cli.settings.no_port")));
            }
            let url = format!("http://127.0.0.1:{}", info.http_port);
            println!("{}", url);
            if !print_only {
                open_in_browser(&url)?;
            }
            Ok(())
        }
    }
}

fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(target_os = "windows")]
    let opener = "cmd";
    #[cfg(all(unix, not(target_os = "macos")))]
    let opener = "xdg-open";

    let mut cmd = std::process::Command::new(opener);
    #[cfg(target_os = "windows")]
    {
        cmd.args(["/C", "start", "", url]);
    }
    #[cfg(not(target_os = "windows"))]
    {
        cmd.arg(url);
    }
    cmd.spawn()
        .map_err(|e| anyhow!("failed to launch browser via {}: {}", opener, e))?;
    Ok(())
}
