//! The audit log's home on disk.
//!
//! The in-memory ring answers `session.audit_log` fast, and vanishes with the
//! process -- which for an *audit* trail is the one unacceptable property.
//! Every entry is appended here as one JSON line, already redacted by the
//! caller, in a per-day file under `~/.unterm/audit/`. A restart backfills
//! the ring from the most recent lines, so "what did the agent do yesterday"
//! survives the terminal being closed.
//!
//! Deliberately not a database: an append-only line per event needs no
//! schema, no migration, and stays greppable from any shell.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Days of history kept on disk before pruning.
const KEEP_DAYS: i64 = 30;

fn audit_dir() -> Option<PathBuf> {
    unterm_protocol::state_path("audit")
}

fn file_name(date: chrono::NaiveDate) -> String {
    format!("audit-{}.jsonl", date.format("%Y%m%d"))
}

/// Append one redacted entry to today's file.
///
/// Failures are logged and swallowed: a full disk must not take the terminal
/// down, and the in-memory ring still holds the entry for this session.
pub fn append(entry: &serde_json::Value) {
    let Some(dir) = audit_dir() else {
        return;
    };
    prune_once(&dir);
    if let Err(error) = append_in(&dir, chrono::Local::now().date_naive(), entry) {
        log::warn!("audit entry not persisted: {error:#}");
    }
}

fn append_in(dir: &Path, date: chrono::NaiveDate, entry: &serde_json::Value) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(file_name(date));
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    // The trail can carry command previews; on unix it is the user's alone,
    // like every other auth-adjacent file this product writes.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path)?;
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// The most recent `limit` persisted entries, oldest first.
///
/// Reads back over the daily files newest-first until the limit is met, so a
/// restart backfills from however many days that takes.
pub fn recent(limit: usize) -> Vec<serde_json::Value> {
    match audit_dir() {
        Some(dir) => recent_in(&dir, limit),
        None => Vec::new(),
    }
}

fn recent_in(dir: &Path, limit: usize) -> Vec<serde_json::Value> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_audit_file(path))
        .collect();
    files.sort();
    let mut out: Vec<serde_json::Value> = Vec::new();
    for path in files.iter().rev() {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut day: Vec<serde_json::Value> = text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        day.extend(out);
        out = day;
        if out.len() >= limit {
            let start = out.len() - limit;
            out.drain(..start);
            break;
        }
    }
    out
}

fn is_audit_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("audit-") && name.ends_with(".jsonl"))
}

/// Drop daily files older than the retention window, once per process.
fn prune_once(dir: &Path) {
    static PRUNED: std::sync::Once = std::sync::Once::new();
    PRUNED.call_once(|| {
        let cutoff = chrono::Local::now().date_naive() - chrono::Duration::days(KEEP_DAYS);
        let keep_from = file_name(cutoff);
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let old = is_audit_file(&path)
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name < keep_from.as_str());
            if old {
                let _ = std::fs::remove_file(&path);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn date(text: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn entries_round_trip_across_days_oldest_first() {
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"n": 1})).unwrap();
        append_in(dir.path(), date("2026-01-02"), &json!({"n": 2})).unwrap();
        append_in(dir.path(), date("2026-01-02"), &json!({"n": 3})).unwrap();

        let all = recent_in(dir.path(), 10);
        let ns: Vec<i64> = all.iter().map(|e| e["n"].as_i64().unwrap()).collect();
        assert_eq!(ns, vec![1, 2, 3]);
    }

    #[test]
    fn the_limit_keeps_the_newest() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..5 {
            append_in(dir.path(), date("2026-01-01"), &json!({"n": n})).unwrap();
        }
        let tail = recent_in(dir.path(), 2);
        let ns: Vec<i64> = tail.iter().map(|e| e["n"].as_i64().unwrap()).collect();
        assert_eq!(ns, vec![3, 4]);
    }

    #[test]
    fn a_corrupt_line_is_passed_over_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"n": 1})).unwrap();
        std::fs::write(
            dir.path().join(file_name(date("2026-01-02"))),
            "not json\n{\"n\":2}\n",
        )
        .unwrap();
        let all = recent_in(dir.path(), 10);
        assert_eq!(all.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn the_trail_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"n": 1})).unwrap();
        let path = dir.path().join(file_name(date("2026-01-01")));
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
