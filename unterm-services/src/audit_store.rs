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
//!
//! **The chain.** Each entry carries `seq`, the sha256 of the entry before
//! it, and its own. An append-only file is only append-only by convention —
//! it is a file, and anyone who can read it can edit it — so the chain is
//! what makes an edit *visible*: changing a line changes its hash, and every
//! line after it says what the hash was supposed to be. That does not prevent
//! tampering; it prevents tampering going unnoticed, which is the honest
//! property a local trail can offer. Somebody who rewrites the whole file
//! from the edit onwards leaves a chain that verifies, and the only defence
//! against that is a copy somewhere else.
//!
//! **Correlation.** An entry that says "a command ran" and cannot say which
//! task, run, step, lease or call it belongs to is a line in a log rather
//! than a record. The ids are stamped here, from whatever the caller knew.

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

/// What an entry belongs to.
///
/// Every field is optional because a caller usually knows some of them. What
/// matters is that the ones it does know are written down: reconstructing
/// "which task was that" afterwards from timestamps is guesswork.
#[derive(Clone, Debug, Default)]
pub struct Correlation {
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub lease_id: Option<String>,
    pub call_id: Option<String>,
    pub approval_id: Option<String>,
    pub grant_id: Option<String>,
    pub workspace_id: Option<String>,
    /// The session the action happened in.
    pub session_id: Option<String>,
    /// The path as actually resolved — not what was typed.
    ///
    /// The resolved path is the one the decision was made about; a record
    /// holding only the caller's string cannot answer "what did it actually
    /// touch" after a symlink moved.
    pub resolved_path: Option<String>,
    /// allowed | denied | needs_approval | failed — the outcome, which is the
    /// field somebody scanning a trail after an incident actually reads.
    pub state: Option<String>,
}

impl Correlation {
    fn stamp(&self, entry: &mut serde_json::Value) {
        let Some(map) = entry.as_object_mut() else {
            return;
        };
        for (key, value) in [
            ("task_id", &self.task_id),
            ("run_id", &self.run_id),
            ("step_id", &self.step_id),
            ("lease_id", &self.lease_id),
            ("call_id", &self.call_id),
            ("approval_id", &self.approval_id),
            ("grant_id", &self.grant_id),
            ("workspace_id", &self.workspace_id),
            ("session_id", &self.session_id),
            ("resolved_path", &self.resolved_path),
            ("state", &self.state),
        ] {
            if let Some(value) = value {
                map.insert(key.to_string(), serde_json::Value::String(value.clone()));
            }
        }
    }
}

/// Append one redacted entry to today's file.
///
/// Failures are logged and swallowed: a full disk must not take the terminal
/// down, and the in-memory ring still holds the entry for this session.
pub fn append(entry: &serde_json::Value) {
    append_correlated(entry, &Correlation::default())
}

/// Append with the ids that say what this entry belongs to.
pub fn append_correlated(entry: &serde_json::Value, correlation: &Correlation) {
    let Some(dir) = audit_dir() else {
        return;
    };
    prune_once(&dir);
    let mut entry = entry.clone();
    correlation.stamp(&mut entry);
    if let Err(error) = append_in(&dir, chrono::Local::now().date_naive(), &entry) {
        log::warn!("audit entry not persisted: {error:#}");
    }
}

/// The hash of one entry, over its content without the chain fields.
///
/// `sha256` is excluded because it is the output, and `prev_sha256` and `seq`
/// are included because a reordering that kept every entry intact would
/// otherwise verify.
fn entry_hash(entry: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let mut copy = entry.clone();
    if let Some(map) = copy.as_object_mut() {
        map.remove("sha256");
    }
    let mut hasher = Sha256::new();
    hasher.update(canonical(&copy).as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Serialise with keys in a fixed order, so the same entry always hashes the
/// same whichever order a map happened to iterate in.
fn canonical(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .into_iter()
                .map(|key| format!("{key:?}:{}", canonical(&map[key])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => format!(
            "[{}]",
            items.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        other => other.to_string(),
    }
}

/// What verifying a trail found.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct ChainReport {
    pub entries: usize,
    pub intact: bool,
    /// The first entry whose hash or link does not match, if any. Only the
    /// first: everything after a break is unverifiable rather than wrong, and
    /// reporting it all as broken buries the one line that matters.
    pub broken_at: Option<usize>,
    pub detail: Option<String>,
}

/// Walk the persisted trail and check that it has not been edited.
pub fn verify_chain() -> ChainReport {
    match audit_dir() {
        Some(dir) => verify_chain_in(&dir),
        None => ChainReport {
            intact: true,
            ..ChainReport::default()
        },
    }
}

fn verify_chain_in(dir: &Path) -> ChainReport {
    let entries = recent_in(dir, usize::MAX);
    let mut previous: Option<String> = None;
    // `None` means "no expectation yet" — before the first chained entry, and
    // again after any gap. A number here would have to be either right or a
    // false alarm, and on a trail that predates the chain it is a false alarm
    // on the user's own history.
    let mut expected_seq: Option<i64> = None;
    for (index, entry) in entries.iter().enumerate() {
        let recorded = entry.get("sha256").and_then(|value| value.as_str());
        let Some(recorded) = recorded else {
            // Entries written before the chain existed have no hashes. They
            // are reported rather than failed: an older trail is not a
            // tampered one, and treating it as one would make the check cry
            // wolf on every upgrade.
            previous = None;
            expected_seq = None;
            continue;
        };
        if entry_hash(entry) != recorded {
            return ChainReport {
                entries: entries.len(),
                intact: false,
                broken_at: Some(index),
                detail: Some("an entry no longer matches its own hash".into()),
            };
        }
        let linked = entry.get("prev_sha256").and_then(|value| value.as_str());
        if let (Some(previous), Some(linked)) = (previous.as_deref(), linked) {
            if previous != linked {
                return ChainReport {
                    entries: entries.len(),
                    intact: false,
                    broken_at: Some(index),
                    detail: Some("an entry is missing or was reordered".into()),
                };
            }
        }
        let seq = entry.get("seq").and_then(|value| value.as_i64()).unwrap_or(-1);
        if let (Some(expected), true) = (expected_seq, seq >= 0) {
            if seq != expected {
                return ChainReport {
                    entries: entries.len(),
                    intact: false,
                    broken_at: Some(index),
                    detail: Some(format!("expected entry {expected}, found {seq}")),
                };
            }
        }
        expected_seq = (seq >= 0).then_some(seq + 1);
        previous = Some(recorded.to_string());
    }
    ChainReport {
        entries: entries.len(),
        intact: true,
        broken_at: None,
        detail: None,
    }
}

/// The last entry's sequence number and hash, so the next one can link to it.
///
/// Read from the file rather than kept in memory: two processes appending —
/// the Core and a GUI that has not migrated — must not both think they are
/// entry seventeen.
///
/// **Only the last line.** The first version of this called `recent_in`,
/// which reads every audit file for the day and JSON-parses every line, to
/// then use exactly one of them. That is linear work per append and
/// quadratic over a session: 0.36 ms at five hundred entries, 3.75 ms at four
/// thousand, still climbing. An agent session writes an entry per event, so
/// a couple of busy panes walked the terminal into a stall it never came out
/// of. Seek to the end and read backwards instead.
fn tip(dir: &Path) -> (i64, Option<String>) {
    let Some(entry) = last_entry(dir) else {
        return (0, None);
    };
    (
        entry.get("seq").and_then(|value| value.as_i64()).unwrap_or(-1) + 1,
        entry
            .get("sha256")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    )
}

/// The newest entry, without reading what came before it.
fn last_entry(dir: &Path) -> Option<serde_json::Value> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_audit_file(path))
        .collect();
    files.sort();
    // Newest first, and keep going: the newest file can be empty — a fresh
    // day whose first entry is the one being written — and the chain
    // continues across days.
    for path in files.iter().rev() {
        if let Some(entry) = last_line_of(path).and_then(|line| serde_json::from_str(&line).ok()) {
            return Some(entry);
        }
    }
    None
}

/// The last non-empty line of a file, read from the end.
fn last_line_of(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    if length == 0 {
        return None;
    }
    // Enough for one entry in a single read; the loop handles the rest, and
    // an entry longer than this is a command preview, not the common case.
    const CHUNK: u64 = 8 * 1024;
    let mut end = length;
    let mut tail: Vec<u8> = Vec::new();
    while end > 0 {
        let start = end.saturating_sub(CHUNK);
        let size = (end - start) as usize;
        let mut buffer = vec![0u8; size];
        file.seek(SeekFrom::Start(start)).ok()?;
        file.read_exact(&mut buffer).ok()?;
        buffer.extend_from_slice(&tail);
        tail = buffer;
        // A trailing newline belongs to the last entry, not to an empty one
        // after it.
        let trimmed = tail
            .iter()
            .rposition(|byte| *byte != b'\n')
            .map(|last| &tail[..=last])?;
        if let Some(position) = trimmed.iter().rposition(|byte| *byte == b'\n') {
            return String::from_utf8(trimmed[position + 1..].to_vec()).ok();
        }
        if start == 0 {
            // The whole file is one line.
            return String::from_utf8(trimmed.to_vec()).ok();
        }
        end = start;
    }
    None
}

fn append_in(dir: &Path, date: chrono::NaiveDate, entry: &serde_json::Value) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let (seq, previous) = tip(dir);
    let mut entry = entry.clone();
    if let Some(map) = entry.as_object_mut() {
        map.insert("seq".into(), serde_json::Value::from(seq));
        match previous {
            Some(previous) => {
                map.insert("prev_sha256".into(), serde_json::Value::String(previous));
            }
            None => {
                map.remove("prev_sha256");
            }
        }
    }
    let hash = entry_hash(&entry);
    if let Some(map) = entry.as_object_mut() {
        map.insert("sha256".into(), serde_json::Value::String(hash));
    }
    let entry = &entry;
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


    #[test]
    fn every_entry_links_to_the_one_before_it() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..4 {
            append_in(dir.path(), date("2026-01-01"), &json!({"n": n})).unwrap();
        }
        let all = recent_in(dir.path(), 10);
        assert_eq!(
            all.iter()
                .map(|e| e["seq"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert!(all[0].get("prev_sha256").is_none(), "the first links to nothing");
        for pair in all.windows(2) {
            assert_eq!(
                pair[1]["prev_sha256"], pair[0]["sha256"],
                "the chain is not linked"
            );
        }
        let report = verify_chain_in(dir.path());
        assert!(report.intact, "{report:?}");
        assert_eq!(report.entries, 4);
    }

    #[test]
    fn editing_an_entry_is_visible() {
        // The property the chain is for. Not prevention — it is a file, and
        // whoever can read it can edit it — but an edit that cannot be
        // noticed is the same as no trail at all.
        let dir = tempfile::tempdir().unwrap();
        for n in 0..3 {
            append_in(dir.path(), date("2026-01-01"), &json!({"command": format!("cmd {n}")}))
                .unwrap();
        }
        let path = dir.path().join(file_name(date("2026-01-01")));
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("cmd 1", "something else")).unwrap();

        let report = verify_chain_in(dir.path());
        assert!(!report.intact, "an edited entry verified");
        assert_eq!(report.broken_at, Some(1));
        assert!(report.detail.unwrap().contains("its own hash"));
    }

    #[test]
    fn removing_an_entry_is_visible() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..4 {
            append_in(dir.path(), date("2026-01-01"), &json!({"n": n})).unwrap();
        }
        let path = dir.path().join(file_name(date("2026-01-01")));
        let text = std::fs::read_to_string(&path).unwrap();
        let kept: Vec<&str> = text
            .lines()
            .enumerate()
            .filter(|(index, _)| *index != 1)
            .map(|(_, line)| line)
            .collect();
        std::fs::write(&path, kept.join("\n") + "\n").unwrap();

        let report = verify_chain_in(dir.path());
        assert!(!report.intact, "a deleted entry left no trace");
        assert_eq!(report.broken_at, Some(1));
    }

    #[test]
    fn a_trail_written_before_the_chain_existed_is_not_called_tampered() {
        // Otherwise the first run after an upgrade reports the user's own
        // history as evidence of an attack.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(
            dir.path().join(file_name(date("2026-01-01"))),
            "{\"n\":1}\n{\"n\":2}\n",
        )
        .unwrap();
        append_in(dir.path(), date("2026-01-02"), &json!({"n": 3})).unwrap();

        let report = verify_chain_in(dir.path());
        assert!(report.intact, "{report:?}");
        assert_eq!(report.entries, 3);
    }

    #[test]
    fn the_chain_continues_across_days() {
        // The files are per-day; the chain is not.
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"n": 1})).unwrap();
        append_in(dir.path(), date("2026-01-02"), &json!({"n": 2})).unwrap();
        let all = recent_in(dir.path(), 10);
        assert_eq!(all[1]["prev_sha256"], all[0]["sha256"]);
        assert!(verify_chain_in(dir.path()).intact);
    }

    #[test]
    fn an_entry_says_what_it_belongs_to() {
        let dir = tempfile::tempdir().unwrap();
        let mut entry = json!({"method": "exec.run"});
        Correlation {
            task_id: Some("tsk_1".into()),
            lease_id: Some("lse_1".into()),
            state: Some("allowed".into()),
            ..Correlation::default()
        }
        .stamp(&mut entry);
        append_in(dir.path(), date("2026-01-01"), &entry).unwrap();

        let stored = recent_in(dir.path(), 1).remove(0);
        assert_eq!(stored["task_id"], "tsk_1");
        assert_eq!(stored["lease_id"], "lse_1");
        assert_eq!(stored["state"], "allowed");
        // And an id nobody supplied is absent rather than empty: a field that
        // says "" is a claim that there was no task.
        assert!(stored.get("run_id").is_none());
    }

    #[test]
    fn the_hash_does_not_depend_on_key_order() {
        // Two entries with the same content must hash alike, or verification
        // depends on how a map happened to iterate.
        let one: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":{"c":2,"d":3}}"#).unwrap();
        let two: serde_json::Value = serde_json::from_str(r#"{"b":{"d":3,"c":2},"a":1}"#).unwrap();
        assert_eq!(entry_hash(&one), entry_hash(&two));
    }


    #[test]
    fn appending_does_not_get_slower_as_the_log_grows() {
        // The regression this exists for: `tip` used to read every entry of
        // the day and JSON-parse it, to use one. Linear per append, quadratic
        // over a session — and an agent session writes an entry per event, so
        // a couple of busy panes walked the terminal into a stall.
        //
        // Timing in a test is usually a bad idea. Here the thing being
        // asserted *is* a complexity class, and the ratio between two sizes
        // is what shows it; the margin is wide enough that a loaded machine
        // does not decide the outcome.
        let dir = tempfile::tempdir().unwrap();
        let day = date("2026-01-01");
        let entry = json!({"method": "session.input", "actor": "codex"});

        let time_for = |count: usize| {
            let start = std::time::Instant::now();
            for _ in 0..count {
                append_in(dir.path(), day, &entry).unwrap();
            }
            start.elapsed().as_secs_f64() / count as f64
        };

        let early = time_for(200);
        // Get well past the point where re-reading everything would hurt.
        time_for(3_000);
        let late = time_for(200);

        assert!(
            late < early * 5.0,
            "appending got {:.1}x slower as the log grew — the tip is reading \
             more than the last line again ({early:.6}s then {late:.6}s)",
            late / early
        );
    }

    #[test]
    fn the_last_entry_is_found_across_an_empty_newer_file() {
        // Midnight: today's file exists and is empty because its first entry
        // is the one being written. The chain has to continue from
        // yesterday's rather than restart at zero.
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"n": 1})).unwrap();
        std::fs::write(dir.path().join(file_name(date("2026-01-02"))), "").unwrap();

        append_in(dir.path(), date("2026-01-02"), &json!({"n": 2})).unwrap();
        let all = recent_in(dir.path(), 10);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1]["seq"], 1, "the sequence restarted across a day");
        assert_eq!(all[1]["prev_sha256"], all[0]["sha256"]);
        assert!(verify_chain_in(dir.path()).intact);
    }

    #[test]
    fn a_single_line_file_has_a_last_line() {
        // The read-backwards loop has to terminate when there is no newline
        // before the entry — the first append of a day.
        let dir = tempfile::tempdir().unwrap();
        append_in(dir.path(), date("2026-01-01"), &json!({"only": true})).unwrap();
        let (seq, previous) = tip(dir.path());
        assert_eq!(seq, 1);
        assert!(previous.is_some());
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
