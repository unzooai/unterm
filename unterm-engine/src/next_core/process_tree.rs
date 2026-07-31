use super::ProcessTreeSnapshot;
use procinfo::LocalProcessInfo;
use std::path::Path;

#[derive(Clone, Debug)]
pub(super) struct ProcessNodeSummary {
    pub(super) pid: u32,
    pub(super) name: String,
    pub(super) cwd: Option<String>,
    pub(super) argv: Vec<String>,
    start_time: u64,
    child_count: usize,
    pub(super) detected_agent: Option<String>,
}

pub(super) fn snapshot(
    root_pid: Option<u32>,
    fallback_process: &str,
) -> Option<ProcessTreeSnapshot> {
    let Some(pid) = root_pid else {
        return Some(fallback(None, fallback_process));
    };
    let Some(root) = LocalProcessInfo::with_root_pid(pid) else {
        return Some(fallback(Some(pid), fallback_process));
    };
    Some(snapshot_from_root(&root))
}

fn fallback(root_pid: Option<u32>, fallback_process: &str) -> ProcessTreeSnapshot {
    ProcessTreeSnapshot {
        root_pid,
        root_process: fallback_process.to_string(),
        root_cwd: None,
        foreground_pid: root_pid,
        foreground_process: fallback_process.to_string(),
        foreground_cwd: None,
        foreground_argv: Vec::new(),
        child_count: 0,
        detected_agent: detect_known_agent_name(fallback_process).map(str::to_string),
    }
}

pub(super) fn snapshot_from_root(root: &LocalProcessInfo) -> ProcessTreeSnapshot {
    let foreground = foreground_process_summary(root);
    ProcessTreeSnapshot {
        root_pid: Some(root.pid),
        root_process: root.name.clone(),
        root_cwd: path_to_non_empty_string(&root.cwd),
        foreground_pid: Some(foreground.pid),
        foreground_process: foreground.name,
        foreground_cwd: foreground.cwd,
        foreground_argv: foreground.argv,
        child_count: count_descendants(root),
        detected_agent: detect_agent_in_process_tree(root),
    }
}

pub(super) fn foreground_process_summary(root: &LocalProcessInfo) -> ProcessNodeSummary {
    let mut best = ProcessNodeSummary {
        pid: root.pid,
        name: root.name.clone(),
        cwd: path_to_non_empty_string(&root.cwd),
        argv: root.argv.clone(),
        start_time: root.start_time,
        child_count: count_descendants(root),
        detected_agent: detect_known_agent_name(&root.name)
            .or_else(|| {
                root.argv
                    .first()
                    .and_then(|arg| detect_known_agent_name(arg))
            })
            .map(str::to_string),
    };
    for child in root.children.values() {
        let candidate = foreground_process_summary(child);
        let candidate_score = (
            candidate.detected_agent.is_some(),
            candidate.start_time,
            candidate.child_count,
        );
        let best_score = (
            best.detected_agent.is_some(),
            best.start_time,
            best.child_count,
        );
        if candidate_score > best_score {
            best = candidate;
        }
    }
    best
}

fn path_to_non_empty_string(path: &Path) -> Option<String> {
    let text = path.to_string_lossy();
    (!text.is_empty()).then(|| text.into_owned())
}

fn count_descendants(root: &LocalProcessInfo) -> usize {
    root.children
        .values()
        .map(|child| 1 + count_descendants(child))
        .sum()
}

pub(super) fn detect_agent_in_process_tree(root: &LocalProcessInfo) -> Option<String> {
    detect_known_agent_name(&root.name)
        .or_else(|| {
            root.argv
                .first()
                .and_then(|arg| detect_known_agent_name(arg))
        })
        .map(str::to_string)
        .or_else(|| {
            root.children
                .values()
                .find_map(detect_agent_in_process_tree)
        })
}

fn detect_known_agent_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let bare = std::path::Path::new(&lower)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&lower);
    match bare {
        "claude" => Some("claude"),
        "codex" => Some("codex"),
        "gemini" => Some("gemini"),
        "kimi" | "kimi-code" => Some("kimi"),
        "aider" => Some("aider"),
        "opencode" => Some("opencode"),
        "trae" | "trae-cli" | "trae_agent" | "trae-agent" => Some("trae"),
        "zcode" | "z-code" | "z code" => Some("zcode"),
        "cursor-agent" | "cursoragent" => Some("cursor-agent"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn process_info_for_test(
        pid: u32,
        ppid: u32,
        name: &str,
        argv: Vec<String>,
        start_time: u64,
        children: Vec<LocalProcessInfo>,
    ) -> LocalProcessInfo {
        LocalProcessInfo {
            pid,
            ppid,
            name: name.to_string(),
            executable: PathBuf::from(name),
            argv,
            cwd: PathBuf::from(format!("C:\\work\\{name}-{pid}")),
            status: procinfo::LocalProcessStatus::Run,
            start_time,
            #[cfg(windows)]
            console: 0,
            children: children
                .into_iter()
                .map(|child| (child.pid, child))
                .collect(),
        }
    }

    #[test]
    fn summary_prefers_known_agent_descendant() {
        // The fixture is a Windows process tree -- powershell.exe hosting
        // node.exe -- and its path arithmetic is Windows path arithmetic.
        if cfg!(not(windows)) {
            return;
        }

        let helper =
            process_info_for_test(30, 20, "node.exe", vec!["node".to_string()], 30, Vec::new());
        let codex = process_info_for_test(
            40,
            20,
            "node.exe",
            vec![
                "C:\\Users\\me\\AppData\\Roaming\\npm\\codex.cmd".to_string(),
                "--ask-for-approval".to_string(),
            ],
            20,
            Vec::new(),
        );
        let root = process_info_for_test(
            10,
            0,
            "powershell.exe",
            vec!["powershell.exe".to_string()],
            10,
            vec![helper, codex],
        );

        let foreground = foreground_process_summary(&root);
        assert_eq!(foreground.pid, 40);
        assert_eq!(foreground.detected_agent.as_deref(), Some("codex"));
        assert_eq!(foreground.cwd.as_deref(), Some("C:\\work\\node.exe-40"));
        let snapshot = snapshot_from_root(&root);
        assert_eq!(
            snapshot.root_cwd.as_deref(),
            Some("C:\\work\\powershell.exe-10")
        );
        assert_eq!(
            snapshot.foreground_cwd.as_deref(),
            Some("C:\\work\\node.exe-40")
        );
        assert_eq!(
            detect_agent_in_process_tree(&root).as_deref(),
            Some("codex")
        );
    }

    #[test]
    fn summary_uses_newest_child_without_agent_match() {
        let older = process_info_for_test(30, 10, "git.exe", Vec::new(), 30, Vec::new());
        let newer = process_info_for_test(40, 10, "cargo.exe", Vec::new(), 40, Vec::new());
        let root = process_info_for_test(10, 0, "cmd.exe", Vec::new(), 10, vec![older, newer]);

        let foreground = foreground_process_summary(&root);
        assert_eq!(foreground.pid, 40);
        assert_eq!(foreground.name, "cargo.exe");
        assert_eq!(foreground.detected_agent, None);
    }

    #[test]
    fn fallback_reports_known_root_agent_without_process_tree() {
        let snapshot = snapshot(None, "codex").expect("fallback snapshot");
        assert_eq!(snapshot.foreground_process, "codex");
        assert_eq!(snapshot.detected_agent.as_deref(), Some("codex"));
        assert_eq!(snapshot.child_count, 0);
    }
}
