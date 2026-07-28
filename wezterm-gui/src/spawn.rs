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
    unterm_services::launch_env::apply_unterm_proxy_env(&mut cmd_builder);
    unterm_services::launch_env::apply_unterm_windows_utf8(&mut cmd_builder);
    unterm_services::launch_env::apply_unterm_profile_env(&mut cmd_builder);

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

/// Inject env vars from this window's bound identity profile, if any.
///
/// Reads the per-instance JSON (see `server_info::read_current`) for
/// the profile binding, loads the profile registry, resolves secrets
/// from the OS keychain, and overlays the resulting env onto the
/// `CommandBuilder`. Profile env wins over any vars that were set by
/// the spawn caller — when a user picks "Work — Acme", we want
/// Work's GITHUB_TOKEN, not whatever leaked in from process env.
///
/// Every failure mode is recoverable: registry unparseable, keychain
/// locked, individual secret missing — we log a warning and continue
/// with whatever did resolve. The mental model is "a shell with
/// partial profile env is still more useful than a failed spawn".
/// The `profile.audit` MCP method surfaces unresolved secrets so the
/// user notices via the chip rather than via a broken `git push`.
///
/// No-op when:
///   - no instance ID (we're not yet registered in `~/.unterm/instances/`)
///   - the instance file's `profile` field is None or empty
///   - the platform has no compiled-in SecretStore backend (e.g.
///     Linux without secret-service)
#[cfg(test)]
mod proxy_guard_tests {
    use unterm_services::launch_env::proxy_endpoint_reachable;
    use std::net::TcpListener;

    #[test]
    fn reachable_when_something_is_listening() {
        // Bind an ephemeral port so the probe has a live endpoint to find.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(proxy_endpoint_reachable(&format!(
            "http://127.0.0.1:{port}"
        )));
        assert!(proxy_endpoint_reachable(&format!(
            "socks5://127.0.0.1:{port}"
        )));
    }

    #[test]
    fn unreachable_when_nothing_is_listening() {
        // Bind then drop, so the port is almost certainly free again — a dead
        // proxy must NOT be injected (this is the "don't break the terminal" guard).
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        assert!(!proxy_endpoint_reachable(&format!(
            "http://127.0.0.1:{port}"
        )));
    }

    #[test]
    fn does_not_block_when_url_has_no_port() {
        // No port to probe → assume reachable rather than silently dropping it.
        assert!(proxy_endpoint_reachable("http://proxy.example.com"));
    }
}
