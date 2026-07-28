//! Spawning and removing fleet panes through the mux.
//!
//! The fleet logic itself moved to `unterm-services`, which knows nothing
//! about a mux or a window. These are the implementations of its two traits
//! for this front end; a different front end supplies its own, which is what
//! the traits are for.

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use unterm_services::cockpit::fleet::{
    clean_with_remover, launch_with_spawner, retry_member_with_driver, Fleet, FleetMember,
    FleetPaneRemover,
    FleetPaneSpawner,
};

struct WezTermTabSpawner;
struct WezTermPaneRemover;

impl FleetPaneSpawner for WezTermTabSpawner {
    fn spawn_member(&mut self, cwd: &Path, command: &str) -> Result<u64> {
        spawn_member_tab(cwd, command)
    }
}

impl FleetPaneRemover for WezTermPaneRemover {
    fn remove_member(&mut self, pane_id: u64) -> Result<()> {
        remove_member_pane(pane_id);
        Ok(())
    }
}

/// Spawn a tab whose shell starts in `cwd`, then type `command` into it.
/// Blocking (rx.recv): callable from MCP/worker threads, NOT from the
/// main thread (the spawn resolves there).
fn spawn_member_tab(cwd: &Path, command: &str) -> Result<u64> {
    use config::keyassignment::SpawnTabDomain;
    use mux::Mux;

    let command_dir = Some(cwd.to_string_lossy().to_string());
    let size = wezterm_term::TerminalSize {
        rows: 32,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
        dpi: 0,
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let dir = command_dir.clone();
    promise::spawn::spawn_into_main_thread(async move {
        promise::spawn::spawn(async move {
            let result = async {
                let mux = Mux::get();
                let window_id = mux
                    .iter_windows()
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("No windows available"))?;
                let (_tab, pane, _wid) = mux
                    .spawn_tab_or_window(
                        Some(window_id),
                        SpawnTabDomain::DefaultDomain,
                        None,
                        dir,
                        size,
                        None,
                        String::new(),
                        None,
                    )
                    .await
                    .context("spawn_tab_or_window")?;
                Ok::<u64, anyhow::Error>(pane.pane_id() as u64)
            }
            .await;
            tx.send(result).ok();
        })
        .detach();
    })
    .detach();
    let pane_id = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .context("fleet member spawn timed out")??;

    // Give the shell a moment to initialize, then type the command.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let text = format!("{command}\r");
    let (tx, rx) = std::sync::mpsc::channel();
    promise::spawn::spawn_into_main_thread(async move {
        let result = (|| {
            let mux = Mux::get();
            let pane = mux
                .get_pane(pane_id as mux::pane::PaneId)
                .ok_or_else(|| anyhow!("pane {pane_id} vanished"))?;
            pane.writer().write_all(text.as_bytes())?;
            Ok::<(), anyhow::Error>(())
        })();
        tx.send(result).ok();
    })
    .detach();
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .context("fleet member command write timed out")??;
    Ok(pane_id)
}

fn remove_member_pane(pane_id: u64) {
    unterm_services::cockpit::status::set_fleet(pane_id, None);
    let (tx, rx) = std::sync::mpsc::channel();
    promise::spawn::spawn_into_main_thread(async move {
        mux::Mux::get().remove_pane(pane_id as mux::pane::PaneId);
        tx.send(()).ok();
    })
    .detach();
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
}

/// Launch a fleet. Blocking; call from a worker/MCP thread.
pub fn launch(cwd: &Path, task: &str, agents: &[String]) -> Result<Fleet> {
    let mut spawner = WezTermTabSpawner;
    launch_with_spawner(cwd, task, agents, &mut spawner)
}

/// Relaunch a pending fleet member in its existing worktree.
///
/// The branch, checkpoint and every committed/uncommitted change are retained.
/// Only the pane association is replaced, so a retry never deletes or resets
/// work. The previous pane is closed before the new agent starts to prevent two
/// processes from concurrently editing the same worktree.
pub fn retry_member(fleet_id: &str, member: &str) -> Result<FleetMember> {
    let mut spawner = WezTermTabSpawner;
    let mut remover = WezTermPaneRemover;
    retry_member_with_driver(fleet_id, member, &mut spawner, &mut remover)
}

/// Remove a fleet: kill surviving panes, remove worktrees + branches,
/// drop the record. Refuses when members are still pending review unless
/// `force`; even with force, a worktree that still has uncommitted or
/// unmerged work aborts unless `force` (the caller confirmed).
pub fn clean(fleet_id: &str, force: bool) -> Result<()> {
    let mut remover = WezTermPaneRemover;
    clean_with_remover(fleet_id, force, &mut remover)
}
