//! Fleet — run one task across N agents in N isolated git worktrees.
//!
//! `launch` verifies the repo is clean, adds a worktree + branch per
//! member beside the repo (`../<repo>.fleet/<slug>-<n>/`), opens a tab
//! per member (its own tab so the tab badge shows that member's state),
//! and types the agent command into the fresh shell. Fleets persist in
//! `~/.unterm/fleets.json` so the Review page and `fleet.clean` survive
//! a restart; panes dying does NOT remove a fleet — the worktrees hold
//! the work product until every member is merged or discarded.

use anyhow::{anyhow, bail, Context, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Pending,
    Merged,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMember {
    pub agent: String,
    pub agent_cmd: String,
    pub worktree: PathBuf,
    pub branch: String,
    pub pane_id: Option<u64>,
    /// HEAD sha the worktree started from — the review baseline.
    pub checkpoint: String,
    pub review: ReviewState,
    /// Number of times this member has been launched. Legacy fleet records
    /// deserialize as their original first attempt.
    #[serde(default = "default_attempt")]
    pub attempt: u32,
    /// Timestamp of the latest launch/retry. Optional for legacy records.
    #[serde(default)]
    pub last_started_at: Option<String>,
    /// Most recent failure to create a pane, cleared after a successful retry.
    #[serde(default)]
    pub last_launch_error: Option<String>,
}

fn default_attempt() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    pub id: String,
    pub task: String,
    pub base_repo: PathBuf,
    pub base_branch: String,
    pub members: Vec<FleetMember>,
    pub created_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FleetFile {
    fleets: Vec<Fleet>,
}

fn fleets_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("UNTERM_FLEETS_PATH") {
        return Some(PathBuf::from(path));
    }
    dirs_next::home_dir().map(|h| h.join(".unterm").join("fleets.json"))
}

#[cfg(test)]
pub(crate) fn reset_store_for_tests() {
    store().lock().clear();
}

fn store() -> &'static Mutex<Vec<Fleet>> {
    static S: OnceLock<Mutex<Vec<Fleet>>> = OnceLock::new();
    S.get_or_init(|| {
        let fleets = fleets_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<FleetFile>(&s).ok())
            .map(|f| f.fleets)
            .unwrap_or_default();
        Mutex::new(fleets)
    })
}

fn save_locked(fleets: &[Fleet]) {
    let Some(path) = fleets_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = match serde_json::to_string_pretty(&FleetFile {
        fleets: fleets.to_vec(),
    }) {
        Ok(b) => b,
        Err(_) => return,
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// `git` with no console flash on Windows (same trick as git_panel).
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().with_context(|| format!("run git {args:?}"))?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Built-in launch command per agent. The task is quoted for the pane's
/// shell (POSIX single-quote everywhere except Windows, which gets
/// double-quote escaping for pwsh/cmd).
pub fn agent_command(agent: &str, task: &str) -> String {
    let quoted = quote_for_shell(task);
    match agent {
        "claude" => format!("claude {quoted}"),
        "codex" => format!("codex {quoted}"),
        "gemini" => format!("gemini -i {quoted}"),
        "aider" => format!("aider --message {quoted}"),
        other => format!("{other} {quoted}"),
    }
}

fn quote_for_shell(s: &str) -> String {
    #[cfg(windows)]
    {
        // pwsh + cmd both accept double quotes; escape embedded quotes.
        format!("\"{}\"", s.replace('"', "`\""))
    }
    #[cfg(not(windows))]
    {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn slugify(task: &str) -> String {
    let mut slug: String = task
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        slug = "task".to_string();
    }
    slug.truncate(32);
    slug
}

pub trait FleetPaneSpawner {
    fn spawn_member(&mut self, cwd: &Path, command: &str) -> Result<u64>;
}

pub trait FleetPaneRemover {
    fn remove_member(&mut self, pane_id: u64) -> Result<()>;
}

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

/// Fast pre-flight check (safe on the main thread): is `cwd` inside a
/// clean git repo? Returns the i18n key of the failure for palette UI.
pub fn precheck(cwd: &Path) -> std::result::Result<(), &'static str> {
    let Ok(root) = git(cwd, &["rev-parse", "--show-toplevel"]) else {
        return Err("cockpit.fleet_not_repo");
    };
    match git(Path::new(&root), &["status", "--porcelain"]) {
        Ok(s) if s.is_empty() => Ok(()),
        _ => Err("cockpit.fleet_not_clean"),
    }
}

/// Which of the built-in agents are actually on PATH.
pub fn installed_agents() -> Vec<&'static str> {
    ["claude", "codex", "gemini", "aider"]
        .iter()
        .copied()
        .filter(|bin| binary_on_path(bin))
        .collect()
}

fn binary_on_path(bin: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        let p = dir.join(bin);
        p.is_file() || {
            #[cfg(windows)]
            {
                dir.join(format!("{bin}.exe")).is_file() || dir.join(format!("{bin}.cmd")).is_file()
            }
            #[cfg(not(windows))]
            {
                false
            }
        }
    })
}

/// Launch a fleet. Blocking; call from a worker/MCP thread.
pub fn launch(cwd: &Path, task: &str, agents: &[String]) -> Result<Fleet> {
    let mut spawner = WezTermTabSpawner;
    launch_with_spawner(cwd, task, agents, &mut spawner)
}

pub fn launch_with_spawner(
    cwd: &Path,
    task: &str,
    agents: &[String],
    spawner: &mut dyn FleetPaneSpawner,
) -> Result<Fleet> {
    if agents.is_empty() {
        bail!("fleet needs at least one agent");
    }
    if agents.len() > 8 {
        bail!("fleet supports at most 8 members");
    }
    let repo_root = PathBuf::from(
        git(cwd, &["rev-parse", "--show-toplevel"]).map_err(|_| anyhow!("not a git repository"))?,
    );
    let base_branch = git(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let head = git(&repo_root, &["rev-parse", "HEAD"])?;
    let dirty = git(&repo_root, &["status", "--porcelain"])?;
    if !dirty.is_empty() {
        bail!("worktree not clean — commit or stash before launching a fleet");
    }

    let slug = slugify(task);
    // Disambiguate against previous fleets for the same slug.
    let existing: usize = {
        let s = store().lock();
        s.iter().filter(|f| f.id.contains(&slug)).count()
    };
    let fleet_id = if existing > 0 {
        format!("{slug}-{}", existing + 1)
    } else {
        slug.clone()
    };

    let repo_name = repo_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "repo".to_string());
    let fleet_dir = repo_root
        .parent()
        .unwrap_or(&repo_root)
        .join(format!("{repo_name}.fleet"));
    std::fs::create_dir_all(&fleet_dir).context("create fleet dir")?;

    let mut members = Vec::new();
    for (i, agent) in agents.iter().enumerate() {
        let n = i + 1;
        let branch = format!("fleet/{fleet_id}-{n}");
        let worktree = fleet_dir.join(format!("{fleet_id}-{n}"));
        git(
            &repo_root,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                &worktree.to_string_lossy(),
                "HEAD",
            ],
        )
        .with_context(|| format!("add worktree for member {n}"))?;
        let agent_cmd = agent_command(agent, task);
        let (pane_id, last_launch_error) = match spawner.spawn_member(&worktree, &agent_cmd) {
            Ok(id) => {
                super::status::set_fleet(id, Some(fleet_id.clone()));
                (Some(id), None)
            }
            Err(err) => {
                log::error!("fleet {fleet_id}: member {n} spawn failed: {err:#}");
                (None, Some(format!("{err:#}")))
            }
        };
        members.push(FleetMember {
            agent: agent.clone(),
            agent_cmd,
            worktree,
            branch,
            pane_id,
            checkpoint: head.clone(),
            review: ReviewState::Pending,
            attempt: 1,
            last_started_at: Some(chrono::Utc::now().to_rfc3339()),
            last_launch_error,
        });
    }

    let fleet = Fleet {
        id: fleet_id,
        task: task.to_string(),
        base_repo: repo_root,
        base_branch,
        members,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    {
        let mut s = store().lock();
        s.push(fleet.clone());
        save_locked(&s);
    }
    Ok(fleet)
}

pub fn list() -> Vec<Fleet> {
    store().lock().clone()
}

pub fn get(id: &str) -> Option<Fleet> {
    store().lock().iter().find(|f| f.id == id).cloned()
}

/// Look up which fleet (if any) a pane belongs to.
pub fn fleet_for_pane(pane_id: u64) -> Option<String> {
    store()
        .lock()
        .iter()
        .find(|f| f.members.iter().any(|m| m.pane_id == Some(pane_id)))
        .map(|f| f.id.clone())
}

/// Update a member's review state (member = 1-based index or branch name).
pub fn set_review_state(fleet_id: &str, member: &str, state: ReviewState) -> Result<()> {
    let mut s = store().lock();
    let fleet = s
        .iter_mut()
        .find(|f| f.id == fleet_id)
        .ok_or_else(|| anyhow!("no fleet {fleet_id:?}"))?;
    let m = resolve_member_mut(fleet, member)?;
    m.review = state;
    save_locked(&s);
    Ok(())
}

fn resolve_member_mut<'a>(fleet: &'a mut Fleet, member: &str) -> Result<&'a mut FleetMember> {
    if let Ok(n) = member.parse::<usize>() {
        let len = fleet.members.len();
        return fleet
            .members
            .get_mut(n.saturating_sub(1))
            .ok_or_else(|| anyhow!("member {n} out of range (fleet has {len})"));
    }
    fleet
        .members
        .iter_mut()
        .find(|m| m.branch == member || m.branch.ends_with(member))
        .ok_or_else(|| anyhow!("no member {member:?}"))
}

pub fn resolve_member(fleet: &Fleet, member: &str) -> Result<FleetMember> {
    if let Ok(n) = member.parse::<usize>() {
        return fleet
            .members
            .get(n.saturating_sub(1))
            .cloned()
            .ok_or_else(|| anyhow!("member {n} out of range"));
    }
    fleet
        .members
        .iter()
        .find(|m| m.branch == member || m.branch.ends_with(member))
        .cloned()
        .ok_or_else(|| anyhow!("no member {member:?}"))
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

pub fn retry_member_with_driver(
    fleet_id: &str,
    member: &str,
    spawner: &mut dyn FleetPaneSpawner,
    remover: &mut dyn FleetPaneRemover,
) -> Result<FleetMember> {
    let (old_pane_id, worktree, branch, agent_cmd) = {
        let s = store().lock();
        let fleet = s
            .iter()
            .find(|f| f.id == fleet_id)
            .ok_or_else(|| anyhow!("no fleet {fleet_id:?}"))?;
        let m = resolve_member(fleet, member)?;
        if m.review != ReviewState::Pending {
            bail!("cannot retry a {:?} fleet member", m.review);
        }
        validate_retry_worktree(&m)?;
        (m.pane_id, m.worktree, m.branch, m.agent_cmd)
    };

    // Persist the new attempt before touching panes. If spawning fails, Review
    // accurately shows that the member has no active pane and may be retried.
    {
        let mut s = store().lock();
        let fleet = s
            .iter_mut()
            .find(|f| f.id == fleet_id)
            .ok_or_else(|| anyhow!("no fleet {fleet_id:?}"))?;
        let m = resolve_member_mut(fleet, member)?;
        m.pane_id = None;
        m.attempt = m.attempt.saturating_add(1);
        m.last_started_at = Some(chrono::Utc::now().to_rfc3339());
        m.last_launch_error = None;
        save_locked(&s);
    }

    if let Some(pane_id) = old_pane_id {
        remover.remove_member(pane_id).ok();
    }

    let pane_id = match spawner.spawn_member(&worktree, &agent_cmd) {
        Ok(pane_id) => pane_id,
        Err(err) => {
            let message = format!("{err:#}");
            let mut s = store().lock();
            if let Some(fleet) = s.iter_mut().find(|f| f.id == fleet_id) {
                if let Ok(m) = resolve_member_mut(fleet, &branch) {
                    m.last_launch_error = Some(message.clone());
                }
                save_locked(&s);
            }
            return Err(err).context("retry fleet member");
        }
    };
    super::status::set_fleet(pane_id, Some(fleet_id.to_string()));

    let mut s = store().lock();
    let Some(fleet) = s.iter_mut().find(|f| f.id == fleet_id) else {
        remover.remove_member(pane_id).ok();
        bail!("fleet {fleet_id:?} was removed while retrying");
    };
    let m = resolve_member_mut(fleet, &branch)?;
    m.pane_id = Some(pane_id);
    m.last_launch_error = None;
    let result = m.clone();
    save_locked(&s);
    Ok(result)
}

fn validate_retry_worktree(member: &FleetMember) -> Result<()> {
    if !member.worktree.is_dir() {
        bail!("fleet worktree {:?} no longer exists", member.worktree);
    }
    let actual_branch = git(&member.worktree, &["rev-parse", "--abbrev-ref", "HEAD"])
        .context("validate fleet worktree")?;
    if actual_branch != member.branch {
        bail!(
            "fleet worktree is on branch {actual_branch:?}, expected {:?}",
            member.branch
        );
    }
    Ok(())
}

fn remove_member_pane(pane_id: u64) {
    super::status::set_fleet(pane_id, None);
    let (tx, rx) = std::sync::mpsc::channel();
    promise::spawn::spawn_into_main_thread(async move {
        mux::Mux::get().remove_pane(pane_id as mux::pane::PaneId);
        tx.send(()).ok();
    })
    .detach();
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
}

/// Remove a fleet: kill surviving panes, remove worktrees + branches,
/// drop the record. Refuses when members are still pending review unless
/// `force`; even with force, a worktree that still has uncommitted or
/// unmerged work aborts unless `force` (the caller confirmed).
pub fn clean(fleet_id: &str, force: bool) -> Result<()> {
    let fleet = get(fleet_id).ok_or_else(|| anyhow!("no fleet {fleet_id:?}"))?;
    if !force {
        let pending = fleet
            .members
            .iter()
            .filter(|m| m.review == ReviewState::Pending)
            .count();
        if pending > 0 {
            bail!("{pending} member(s) still pending review — merge/discard them or use force");
        }
    }
    for m in &fleet.members {
        if let Some(pane_id) = m.pane_id {
            remove_member_pane(pane_id);
        }
        if m.worktree.exists() {
            git(
                &fleet.base_repo,
                &[
                    "worktree",
                    "remove",
                    "--force",
                    &m.worktree.to_string_lossy(),
                ],
            )
            .with_context(|| format!("remove worktree {:?}", m.worktree))?;
        } else {
            let _ = git(&fleet.base_repo, &["worktree", "prune"]);
        }
        let _ = git(&fleet.base_repo, &["branch", "-D", &m.branch]);
    }
    // Remove the fleet dir if now empty.
    if let Some(parent) = fleet.members.first().and_then(|m| m.worktree.parent()) {
        let _ = std::fs::remove_dir(parent);
    }
    let mut s = store().lock();
    s.retain(|f| f.id != fleet_id);
    save_locked(&s);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(worktree: PathBuf, branch: &str) -> FleetMember {
        FleetMember {
            agent: "codex".to_string(),
            agent_cmd: "codex test".to_string(),
            worktree,
            branch: branch.to_string(),
            pane_id: Some(42),
            checkpoint: "abc123".to_string(),
            review: ReviewState::Pending,
            attempt: 1,
            last_started_at: None,
            last_launch_error: None,
        }
    }

    #[test]
    fn slug_basics() {
        assert_eq!(slugify("Fix the login bug now please"), "fix-the-login-bug");
        assert_eq!(slugify("修复登录"), "task");
        assert_eq!(slugify("a  b"), "a-b");
    }

    #[test]
    fn agent_commands_quote_tasks() {
        #[cfg(not(windows))]
        let cmd = agent_command("claude", "fix it's bug");
        #[cfg(not(windows))]
        assert_eq!(cmd, "claude 'fix it'\\''s bug'");
        assert!(agent_command("codex", "t").starts_with("codex "));
        assert!(agent_command("gemini", "t").starts_with("gemini -i "));
        assert!(agent_command("aider", "t").starts_with("aider --message "));
    }

    #[test]
    fn legacy_member_defaults_to_first_attempt() {
        let json = r#"{
            "agent":"codex",
            "agent_cmd":"codex test",
            "worktree":"repo.fleet/task-1",
            "branch":"fleet/task-1",
            "pane_id":null,
            "checkpoint":"abc123",
            "review":"pending"
        }"#;
        let member: FleetMember = serde_json::from_str(json).unwrap();
        assert_eq!(member.attempt, 1);
        assert_eq!(member.last_started_at, None);
        assert_eq!(member.last_launch_error, None);
    }

    #[test]
    fn retry_validation_keeps_dirty_worktree_and_checks_branch() {
        let temp = tempfile::tempdir().unwrap();
        git(temp.path(), &["init", "-b", "fleet/test-1"]).unwrap();
        git(
            temp.path(),
            &["config", "user.email", "test@unterm.invalid"],
        )
        .unwrap();
        git(temp.path(), &["config", "user.name", "Unterm Test"]).unwrap();
        std::fs::write(temp.path().join("result.txt"), "first\n").unwrap();
        git(temp.path(), &["add", "result.txt"]).unwrap();
        git(temp.path(), &["commit", "-m", "initial"]).unwrap();

        // A failed agent may leave valuable uncommitted and untracked work.
        // Validation must accept it and must not modify it.
        std::fs::write(temp.path().join("result.txt"), "first\nsecond\n").unwrap();
        std::fs::write(temp.path().join("untracked.txt"), "keep me\n").unwrap();
        let member = test_member(temp.path().to_path_buf(), "fleet/test-1");
        validate_retry_worktree(&member).unwrap();
        let status = git(temp.path(), &["status", "--porcelain"]).unwrap();
        assert!(status.contains("result.txt"));
        assert!(status.contains("untracked.txt"));

        let wrong_branch = test_member(temp.path().to_path_buf(), "fleet/other");
        assert!(validate_retry_worktree(&wrong_branch)
            .unwrap_err()
            .to_string()
            .contains("expected"));
    }
}
