use anyhow::{anyhow, bail, Context};
use config::keyassignment::SpawnCommand;
use config::TermConfig;
use mux::activity::Activity;
use mux::domain::SplitSource;
use mux::tab::SplitRequest;
use mux::window::WindowId as MuxWindowId;
use mux::Mux;
use portable_pty::CommandBuilder;
use std::sync::Arc;
use wezterm_term::TerminalSize;

#[derive(Copy, Debug, Clone, Eq, PartialEq)]
pub enum SpawnWhere {
    NewWindow,
    NewTab,
    SplitPane(SplitRequest),
}

pub fn spawn_command_impl(
    spawn: &SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    src_window_id: Option<MuxWindowId>,
    term_config: Arc<TermConfig>,
) {
    let spawn = spawn.clone();

    promise::spawn::spawn(async move {
        if let Err(err) =
            spawn_command_internal(spawn, spawn_where, size, src_window_id, term_config).await
        {
            log::error!("Failed to spawn: {:#}", err);
        }
    })
    .detach();
}

pub async fn spawn_command_internal(
    spawn: SpawnCommand,
    spawn_where: SpawnWhere,
    size: TerminalSize,
    src_window_id: Option<MuxWindowId>,
    term_config: Arc<TermConfig>,
) -> anyhow::Result<()> {
    let mux = Mux::get();
    let activity = Activity::new();

    let current_pane_id = match src_window_id {
        Some(window_id) => {
            if let Some(tab) = mux.get_active_tab_for_window(window_id) {
                tab.get_active_pane().map(|p| p.pane_id())
            } else {
                None
            }
        }
        None => None,
    };

    let cwd = if let Some(cwd) = spawn.cwd.as_ref() {
        Some(cwd.to_str().map(|s| s.to_owned()).ok_or_else(|| {
            anyhow!(
                "Domain::spawn requires that the cwd be unicode in {:?}",
                cwd
            )
        })?)
    } else {
        None
    };

    let mut cmd_builder = match (
        spawn.args.as_ref(),
        spawn.cwd.as_ref(),
        spawn.set_environment_variables.is_empty(),
    ) {
        (None, None, true) => None,
        _ => {
            let mut builder = spawn
                .args
                .as_ref()
                .map(|args| CommandBuilder::from_argv(args.iter().map(Into::into).collect()))
                .unwrap_or_else(CommandBuilder::new_default_prog);
            for (k, v) in spawn.set_environment_variables.iter() {
                builder.env(k, v);
            }
            if let Some(cwd) = &spawn.cwd {
                builder.cwd(cwd);
            }
            Some(builder)
        }
    };
    apply_unterm_proxy_env(&mut cmd_builder);

    let workspace = mux.active_workspace().clone();

    match spawn_where {
        SpawnWhere::SplitPane(direction) => {
            let src_window_id = match src_window_id {
                Some(id) => id,
                None => anyhow::bail!("no src window when splitting a pane?"),
            };
            if let Some(tab) = mux.get_active_tab_for_window(src_window_id) {
                let pane = tab
                    .get_active_pane()
                    .ok_or_else(|| anyhow!("tab to have a pane"))?;

                log::trace!("doing split_pane");
                let (pane, _size) = mux
                    .split_pane(
                        // tab.tab_id(),
                        pane.pane_id(),
                        direction,
                        SplitSource::Spawn {
                            command: cmd_builder,
                            command_dir: cwd,
                        },
                        spawn.domain,
                    )
                    .await
                    .context("split_pane")?;
                pane.set_config(term_config);
            } else {
                bail!("there is no active tab while splitting pane!?");
            }
        }
        _ => {
            let (_tab, pane, window_id) = mux
                .spawn_tab_or_window(
                    match spawn_where {
                        SpawnWhere::NewWindow => None,
                        _ => src_window_id,
                    },
                    spawn.domain,
                    cmd_builder,
                    cwd,
                    size,
                    current_pane_id,
                    workspace,
                    spawn.position,
                )
                .await
                .context("spawn_tab_or_window")?;

            // If it was created in this window, it copies our handlers.
            // Otherwise, we'll pick them up when we later respond to
            // the new window being created.
            if Some(window_id) == src_window_id {
                pane.set_config(term_config);
            }
        }
    };

    drop(activity);

    Ok(())
}

fn apply_unterm_proxy_env(cmd_builder: &mut Option<CommandBuilder>) {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    let builder = cmd_builder.get_or_insert_with(CommandBuilder::new_default_prog);
    for (key, value) in proxy {
        builder.env(key, value);
    }
}

pub(crate) fn apply_unterm_proxy_to_spawn(spawn: &mut SpawnCommand) {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    for (key, value) in proxy {
        spawn.set_environment_variables.insert(key, value);
    }
}

pub(crate) fn apply_unterm_proxy_to_process_env() {
    let Some(proxy) = read_unterm_proxy_env() else {
        return;
    };
    for (key, value) in proxy {
        std::env::set_var(key, value);
    }
}

fn read_unterm_proxy_env() -> Option<Vec<(String, String)>> {
    // Schema is now just `{ "enabled": true|false }`. URLs are auto-detected
    // from the OS at spawn time so the user doesn't have to copy host:port
    // out of their proxy GUI. Existing proxy.json files with explicit
    // `http_proxy` / `socks_proxy` keys still work as a manual override.
    let path = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".unterm")
        .join("proxy.json");
    let value: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({"enabled": false}));
    if !value
        .get("enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    // Prefer explicit override URLs in proxy.json; otherwise auto-detect.
    let manual_http = value
        .get("http_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let manual_socks = value
        .get("socks_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let manual_no = value
        .get("no_proxy")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let detected = if manual_http.is_none() && manual_socks.is_none() {
        crate::system_proxy::detect()
    } else {
        None
    };

    let http = manual_http.or_else(|| {
        detected
            .as_ref()
            .and_then(|d| d.primary_http().map(str::to_string))
    });
    let socks = manual_socks.or_else(|| detected.as_ref().and_then(|d| d.socks.clone()));
    let no_proxy = manual_no
        .or_else(|| detected.as_ref().and_then(|d| d.no_proxy.clone()))
        .unwrap_or_else(|| "localhost,127.0.0.1,::1".to_string());

    let mut env = Vec::new();
    if let Some(http) = &http {
        env.push(("HTTP_PROXY".to_string(), http.clone()));
        env.push(("HTTPS_PROXY".to_string(), http.clone()));
        env.push(("http_proxy".to_string(), http.clone()));
        env.push(("https_proxy".to_string(), http.clone()));
    }
    if let Some(socks) = &socks {
        env.push(("ALL_PROXY".to_string(), socks.clone()));
        env.push(("all_proxy".to_string(), socks.clone()));
    }
    if !no_proxy.is_empty() {
        env.push(("NO_PROXY".to_string(), no_proxy.clone()));
        env.push(("no_proxy".to_string(), no_proxy));
    }

    if env.is_empty() {
        log::warn!(
            "Unterm proxy is enabled but no proxy could be detected from the OS or scan; \
             set explicit URLs in ~/.unterm/proxy.json if needed"
        );
        None
    } else {
        Some(env)
    }
}
