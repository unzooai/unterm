//! `unterm-cli lang ...` — list, set, or print the active interface locale.

use super::i18n;
use super::output::print_json;
use anyhow::{anyhow, Result};
use clap::Parser;
use serde_json::json;

#[derive(Debug, Parser, Clone)]
pub struct LangCommand {
    #[command(subcommand)]
    pub sub: LangSubCommand,
}

#[derive(Debug, Parser, Clone)]
pub enum LangSubCommand {
    /// List every supported locale.
    List,
    /// Persist a new active locale to ~/.unterm/lang.json.
    Set {
        /// Locale code (e.g. en-US, zh-CN, ja-JP).
        code: String,
    },
    /// Print the currently active locale.
    Current,
}

pub fn run(cmd: LangCommand, json_out: bool) -> Result<()> {
    match cmd.sub {
        LangSubCommand::List => list(json_out),
        LangSubCommand::Set { code } => set(&code, json_out),
        LangSubCommand::Current => current(json_out),
    }
}

fn list(json_out: bool) -> Result<()> {
    let active = i18n::current_locale();
    if json_out {
        let arr: Vec<_> = i18n::available_locales()
            .iter()
            .map(|(code, name)| {
                json!({
                    "code": code,
                    "name": name,
                    "active": *code == active,
                })
            })
            .collect();
        print_json(&json!({"active": active, "locales": arr}));
    } else {
        let head_code = i18n::t("cli.lang.head.code");
        let head_name = i18n::t("cli.lang.head.name");
        let head_active = i18n::t("cli.lang.head.active");
        println!("{:<3} {:<8} {:<14} {}", "", head_code, head_active, head_name);
        for (code, name) in i18n::available_locales() {
            let marker = if *code == active { "*" } else { " " };
            let yes_no = if *code == active { "*" } else { "" };
            println!("{:<3} {:<8} {:<14} {}", marker, code, yes_no, name);
        }
    }
    Ok(())
}

fn set(code: &str, json_out: bool) -> Result<()> {
    if !i18n::set_locale_persistent(code) {
        return Err(anyhow!(
            "{}",
            i18n::t_args("cli.lang.unknown", &[("code", code)])
        ));
    }
    let canon = i18n::current_locale();
    let name = i18n::locale_name(canon).unwrap_or("");
    if json_out {
        print_json(&json!({"set": true, "code": canon, "name": name}));
    } else {
        println!(
            "{}",
            i18n::t_args("cli.lang.set", &[("code", canon), ("name", name)])
        );
    }
    Ok(())
}

fn current(json_out: bool) -> Result<()> {
    let code = i18n::current_locale();
    let name = i18n::locale_name(code).unwrap_or("");
    if json_out {
        print_json(&json!({"code": code, "name": name}));
    } else {
        println!(
            "{}",
            i18n::t_args("cli.lang.current", &[("code", code), ("name", name)])
        );
    }
    Ok(())
}
