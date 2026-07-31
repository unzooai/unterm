//! Saved workspaces: named snapshots of where every tab was.
//!
//! The same files the MCP surface's `workspace.save` / `workspace.list` /
//! `workspace.restore` tools read and write -- `~/.unterm/workspaces/*.json`
//! -- so a workspace saved from either side shows up on both. 0.57.4's mux
//! had live workspaces to switch between; next-core keeps them as snapshots,
//! and "switching" is reopening one, a tab per saved directory.

use serde_json::{json, Value};

/// One saved workspace, as much of it as the palette needs.
pub struct Saved {
    pub name: String,
    /// One per saved session that still knew its directory, in saved order.
    pub cwds: Vec<String>,
}

fn directory() -> Option<std::path::PathBuf> {
    dirs_next::home_dir().map(|home| home.join(".unterm").join("workspaces"))
}

/// Every saved workspace, by name.
pub fn list() -> Vec<Saved> {
    directory().map(|dir| list_in(&dir)).unwrap_or_default()
}

fn list_in(dir: &std::path::Path) -> Vec<Saved> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut workspaces: Vec<Saved> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "json"))
        .filter_map(|path| read(&path))
        .collect();
    workspaces.sort_by(|a, b| a.name.cmp(&b.name));
    workspaces
}

/// One workspace's directories, or nothing if it cannot be read.
pub fn cwds(name: &str) -> Vec<String> {
    directory()
        .map(|dir| dir.join(format!("{name}.json")))
        .and_then(|path| read(&path))
        .map(|saved| saved.cwds)
        .unwrap_or_default()
}

fn read(path: &std::path::Path) -> Option<Saved> {
    let name = path.file_stem()?.to_string_lossy().to_string();
    let text = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let cwds = value
        .get("sessions")?
        .as_array()?
        .iter()
        .filter_map(|session| session.get("cwd").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    Some(Saved { name, cwds })
}

/// Save tabs as `(title, cwd)` pairs under a name. Returns how many.
pub fn save(name: &str, tabs: &[(String, String)]) -> anyhow::Result<usize> {
    let dir = directory().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
    save_in(&dir, name, tabs)
}

fn save_in(dir: &std::path::Path, name: &str, tabs: &[(String, String)]) -> anyhow::Result<usize> {
    // The name becomes a file name, and a typed line is not a place to
    // accept path traversal from.
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        anyhow::bail!("`{name}` is not a usable workspace name");
    }
    std::fs::create_dir_all(dir)?;
    // The shape `workspace.save` writes, so `workspace.restore` and
    // `workspace.list` can read it back without knowing who saved it.
    let sessions: Vec<Value> = tabs
        .iter()
        .enumerate()
        .map(|(index, (title, cwd))| {
            json!({
                "id": index,
                "title": title,
                "cwd": cwd,
            })
        })
        .collect();
    let workspace = json!({
        "name": name,
        "sessions": sessions,
        "saved_at": chrono::Local::now().to_rfc3339(),
    });
    std::fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&workspace)?,
    )?;
    Ok(tabs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(title: &str, cwd: &str) -> (String, String) {
        (title.to_string(), cwd.to_string())
    }

    #[test]
    fn a_round_trip_keeps_the_directories_in_order() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        save_in(dir.path(), "review", &[tab("api", "/code/api"), tab("web", "/code/web")])
            .expect("a saved workspace");

        let saved = list_in(dir.path());
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "review");
        assert_eq!(saved[0].cwds, ["/code/api", "/code/web"]);
    }

    /// The MCP surface writes richer records -- profiles, launch contexts,
    /// null cwds for panes that never reported one. The palette reads what it
    /// understands and skips what it does not, rather than refusing the file.
    #[test]
    fn a_workspace_the_mcp_surface_saved_is_readable() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let record = json!({
            "name": "agents",
            "sessions": [
                {"id": 3, "title": "claude", "cwd": "/work/agents", "profile": "dev",
                 "launch": {"values_redacted": true}},
                {"id": 4, "title": "idle", "cwd": null},
            ],
            "saved_at": "2026-07-30T12:00:00+08:00",
        });
        std::fs::write(
            dir.path().join("agents.json"),
            serde_json::to_string_pretty(&record).unwrap(),
        )
        .unwrap();

        let saved = list_in(dir.path());
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].cwds, ["/work/agents"]);
    }

    #[test]
    fn workspaces_come_back_sorted_by_name() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        save_in(dir.path(), "zulu", &[tab("z", "/z")]).unwrap();
        save_in(dir.path(), "alpha", &[tab("a", "/a")]).unwrap();
        let names: Vec<String> = list_in(dir.path()).into_iter().map(|ws| ws.name).collect();
        assert_eq!(names, ["alpha", "zulu"]);
    }

    /// A typed name becomes a file name, and must not name a file anywhere
    /// else.
    #[test]
    fn a_name_that_walks_the_filesystem_is_refused() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for name in ["", "..", "a/b", r"a\b"] {
            assert!(
                save_in(dir.path(), name, &[tab("t", "/t")]).is_err(),
                "{name:?} should not be a workspace name"
            );
        }
    }

    /// A file that is not a workspace is not a workspace, rather than a
    /// panic or a phantom entry.
    #[test]
    fn junk_files_are_passed_over() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("junk.json"), "not json").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "{}").unwrap();
        assert!(list_in(dir.path()).is_empty());
        assert!(super::read(&dir.path().join("missing.json")).is_none());
    }
}
