//! Cockpit hook wiring — the third setup-ai channel.
//!
//! The cockpit's OSC layer already tracks agents zero-config; official
//! hooks are the highest-precision layer on top. This module writes the
//! per-agent hook configuration that reports lifecycle events back via
//! `unterm-cli agent signal` (which reads $WEZTERM_PANE from the hook's
//! inherited environment and no-ops when no Unterm is running):
//!
//!   * Claude Code — ~/.claude/settings.json `hooks`:
//!       UserPromptSubmit → working, Notification → waiting, Stop → done.
//!   * Codex — ~/.codex/config.toml `notify` (turn-complete → done) and
//!       `[tui] notifications = true` (approval OSC 9 for the OSC layer).
//!   * Aider — ~/.aider.conf.yml `notifications` + `notifications-command`
//!       (waiting-for-input → waiting). Aider has no other signal source.
//!   * Gemini — intentionally untouched: its dynamic window title already
//!       encodes all four states with zero configuration.
//!
//! Merge-only, never clobber: an existing user value wins and we report
//! "skipped". First write of each file leaves a `<file>.unterm-bak` copy.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct HookReport {
    pub client: &'static str,
    pub path: String,
    pub action: String, // written | unchanged | skipped(<why>) | error(<why>)
}

fn signal_cmd(cli: &str, agent: &str, event: &str) -> String {
    format!("{cli} agent signal --agent {agent} --event {event}")
}

fn backup_once(path: &Path) {
    let bak = PathBuf::from(format!("{}.unterm-bak", path.display()));
    if !bak.exists() {
        let _ = std::fs::copy(path, bak);
    }
}

pub fn apply_all(cli: &str, remove: bool, dry_run: bool) -> Vec<HookReport> {
    let Some(home) = dirs_next::home_dir() else {
        return vec![];
    };
    let mut reports = Vec::new();
    if home.join(".claude").exists() || home.join(".claude.json").exists() {
        reports.push(apply_claude(&home, cli, remove, dry_run));
    }
    if home.join(".codex").exists() {
        reports.push(apply_codex(&home, cli, remove, dry_run));
    }
    if home.join(".aider.conf.yml").exists() || binary_on_path("aider") {
        reports.push(apply_aider(&home, cli, remove, dry_run));
    }
    reports
}

fn binary_on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let p = dir.join(bin);
                p.is_file() || cfg!(windows) && dir.join(format!("{bin}.exe")).is_file()
            })
        })
        .unwrap_or(false)
}

// --- Claude Code -------------------------------------------------------

fn apply_claude(home: &Path, cli: &str, remove: bool, dry_run: bool) -> HookReport {
    let path = home.join(".claude").join("settings.json");
    let report = |action: String| HookReport {
        client: "claude-code",
        path: path.display().to_string(),
        action,
    };
    let mut root: Value = match std::fs::read_to_string(&path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(e) => return report(format!("skipped(unparseable json: {e})")),
        },
        Err(_) => json!({}),
    };
    if !root.is_object() {
        return report("skipped(settings.json is not an object)".into());
    }

    let events: &[(&str, &str)] = &[
        ("UserPromptSubmit", "working"),
        ("Notification", "waiting"),
        ("Stop", "done"),
    ];
    let hooks = root
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| json!({}));
    if !hooks.is_object() {
        return report("skipped(hooks is not an object)".into());
    }
    let mut changed = false;
    for (event, signal_event) in events {
        let cmd = signal_cmd(cli, "claude", signal_event);
        let arr = hooks
            .as_object_mut()
            .unwrap()
            .entry(*event)
            .or_insert_with(|| json!([]));
        let Some(list) = arr.as_array_mut() else {
            continue;
        };
        let is_ours = |v: &Value| {
            v.pointer("/hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| c.contains("agent signal"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        };
        if remove {
            let before = list.len();
            list.retain(|v| !is_ours(v));
            changed |= list.len() != before;
        } else if !list.iter().any(is_ours) {
            list.push(json!({
                "hooks": [ { "type": "command", "command": cmd, "timeout": 5 } ]
            }));
            changed = true;
        }
    }
    if remove {
        // Drop now-empty arrays / the hooks object itself if we emptied it.
        if let Some(obj) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) {
            obj.retain(|_, v| v.as_array().map(|a| !a.is_empty()).unwrap_or(true));
        }
    }
    if !changed {
        return report("unchanged".into());
    }
    if dry_run {
        return report("would write".into());
    }
    if path.exists() {
        backup_once(&path);
    } else if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(&root)
        .map_err(|e| e.to_string())
        .and_then(|s| std::fs::write(&path, s + "\n").map_err(|e| e.to_string()))
    {
        Ok(()) => report("written".into()),
        Err(e) => report(format!("error({e})")),
    }
}

// --- Codex -------------------------------------------------------------

fn apply_codex(home: &Path, cli: &str, remove: bool, dry_run: bool) -> HookReport {
    let path = home.join(".codex").join("config.toml");
    let report = |action: String| HookReport {
        client: "codex",
        path: path.display().to_string(),
        action,
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: toml::Value = match toml::from_str(&text) {
        Ok(v) => v,
        Err(e) => return report(format!("skipped(unparseable toml: {e})")),
    };
    let Some(table) = doc.as_table_mut() else {
        return report("skipped(not a toml table)".into());
    };
    let ours = |v: &toml::Value| {
        v.as_array()
            .and_then(|a| a.get(1).zip(a.get(2)))
            .map(|(x, y)| x.as_str() == Some("agent") && y.as_str() == Some("signal"))
            .unwrap_or(false)
    };
    let mut changed = false;
    if remove {
        if table.get("notify").map(&ours).unwrap_or(false) {
            table.remove("notify");
            changed = true;
        }
    } else {
        match table.get("notify") {
            Some(v) if ours(v) => {}
            Some(_) => return report("skipped(user already has a notify command)".into()),
            None => {
                table.insert(
                    "notify".into(),
                    toml::Value::Array(vec![
                        toml::Value::String(cli.to_string()),
                        toml::Value::String("agent".into()),
                        toml::Value::String("signal".into()),
                        toml::Value::String("--agent".into()),
                        toml::Value::String("codex".into()),
                        toml::Value::String("--event".into()),
                        toml::Value::String("done".into()),
                    ]),
                );
                changed = true;
            }
        }
        // Turn on TUI notifications so approval requests reach the OSC
        // layer (waiting detection) even without a hook for it.
        let tui = table
            .entry("tui")
            .or_insert_with(|| toml::Value::Table(Default::default()));
        if let Some(tui) = tui.as_table_mut() {
            if tui.get("notifications").is_none() {
                tui.insert("notifications".into(), toml::Value::Boolean(true));
                changed = true;
            }
        }
    }
    if !changed {
        return report("unchanged".into());
    }
    if dry_run {
        return report("would write".into());
    }
    if path.exists() {
        backup_once(&path);
    } else if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match toml::to_string_pretty(&doc)
        .map_err(|e| e.to_string())
        .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()))
    {
        Ok(()) => report("written".into()),
        Err(e) => report(format!("error({e})")),
    }
}

// --- Aider -------------------------------------------------------------

const AIDER_MARK: &str = "# managed by `unterm-cli agent enable-hooks`";

fn apply_aider(home: &Path, cli: &str, remove: bool, dry_run: bool) -> HookReport {
    let path = home.join(".aider.conf.yml");
    let report = |action: String| HookReport {
        client: "aider",
        path: path.display().to_string(),
        action,
    };
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let has_ours = text.contains(AIDER_MARK);
    if remove {
        if !has_ours {
            return report("unchanged".into());
        }
        let stripped: Vec<&str> = text
            .lines()
            .filter(|l| {
                !(l.contains(AIDER_MARK)
                    || l.starts_with("notifications:") && text.contains(AIDER_MARK)
                    || l.starts_with("notifications-command:") && l.contains("agent signal"))
            })
            .collect();
        if dry_run {
            return report("would write".into());
        }
        backup_once(&path);
        return match std::fs::write(&path, stripped.join("\n") + "\n") {
            Ok(()) => report("written".into()),
            Err(e) => report(format!("error({e})")),
        };
    }
    if has_ours {
        return report("unchanged".into());
    }
    if text.contains("notifications-command:") {
        return report("skipped(user already has notifications-command)".into());
    }
    let block = format!(
        "\n{AIDER_MARK}\nnotifications: true\nnotifications-command: {}\n",
        signal_cmd(cli, "aider", "waiting")
    );
    if dry_run {
        return report("would write".into());
    }
    if path.exists() {
        backup_once(&path);
    }
    match std::fs::write(&path, format!("{text}{block}")) {
        Ok(()) => report("written".into()),
        Err(e) => report(format!("error({e})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("unterm-hooks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn claude_hooks_merge_and_remove() {
        let home = tmp_home("claude");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::write(
            home.join(".claude/settings.json"),
            r#"{"model":"opus","hooks":{"Stop":[{"hooks":[{"type":"command","command":"say done"}]}]}}"#,
        )
        .unwrap();

        let r = apply_claude(&home, "unterm-cli", false, false);
        assert_eq!(r.action, "written");
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude/settings.json")).unwrap())
                .unwrap();
        // User's model + existing Stop hook survive; ours appended.
        assert_eq!(v["model"], "opus");
        assert_eq!(v["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert!(v["hooks"]["Notification"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("--event waiting"));

        // Idempotent.
        let r = apply_claude(&home, "unterm-cli", false, false);
        assert_eq!(r.action, "unchanged");

        // Remove strips only ours.
        let r = apply_claude(&home, "unterm-cli", true, false);
        assert_eq!(r.action, "written");
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(home.join(".claude/settings.json")).unwrap())
                .unwrap();
        assert_eq!(v["hooks"]["Stop"].as_array().unwrap().len(), 1);
        assert!(v["hooks"].get("Notification").is_none());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn codex_respects_existing_notify() {
        let home = tmp_home("codex");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::write(
            home.join(".codex/config.toml"),
            "notify = [\"my-notifier\"]\n",
        )
        .unwrap();
        let r = apply_codex(&home, "unterm-cli", false, false);
        assert!(r.action.starts_with("skipped"), "{}", r.action);

        std::fs::write(home.join(".codex/config.toml"), "model = \"gpt-5\"\n").unwrap();
        let r = apply_codex(&home, "unterm-cli", false, false);
        assert_eq!(r.action, "written");
        let text = std::fs::read_to_string(home.join(".codex/config.toml")).unwrap();
        assert!(text.contains("notify"));
        assert!(text.contains("notifications = true"));
        assert!(text.contains("model = \"gpt-5\""));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn aider_appends_and_strips() {
        let home = tmp_home("aider");
        std::fs::write(home.join(".aider.conf.yml"), "dark-mode: true\n").unwrap();
        let r = apply_aider(&home, "unterm-cli", false, false);
        assert_eq!(r.action, "written");
        let text = std::fs::read_to_string(home.join(".aider.conf.yml")).unwrap();
        assert!(text.contains("dark-mode: true"));
        assert!(text.contains("notifications-command: unterm-cli agent signal"));

        let r = apply_aider(&home, "unterm-cli", true, false);
        assert_eq!(r.action, "written");
        let text = std::fs::read_to_string(home.join(".aider.conf.yml")).unwrap();
        assert!(text.contains("dark-mode: true"));
        assert!(!text.contains("notifications-command"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
