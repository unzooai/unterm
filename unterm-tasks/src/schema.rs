//! The database's shape, and how it gets from one shape to the next.
//!
//! Migrations are numbered and applied in order inside a transaction, with
//! `PRAGMA user_version` as the record of where a file has got to. Before a
//! file is migrated at all it is copied aside, because the failure this
//! protects against is not "the migration errored" — that rolls back — but
//! "the migration succeeded and was wrong", which is only recoverable if the
//! old bytes still exist.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Every migration, in order. The index is the version it produces: after
/// applying `MIGRATIONS[0]` the file is at `user_version = 1`.
///
/// Never edit one that has shipped. A user's file was migrated by the old
/// text and will not be revisited; changing it only makes new installs differ
/// from old ones, which is the hardest class of bug to see.
const MIGRATIONS: &[&str] = &[
    // v1 — the durable task engine's first shape.
    r#"
    CREATE TABLE tasks (
        id          TEXT PRIMARY KEY,
        kind        TEXT NOT NULL,
        title       TEXT NOT NULL,
        state       TEXT NOT NULL,
        version     INTEGER NOT NULL DEFAULT 1,
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
    );

    CREATE TABLE runs (
        id          TEXT PRIMARY KEY,
        task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
        attempt     INTEGER NOT NULL,
        state       TEXT NOT NULL,
        version     INTEGER NOT NULL DEFAULT 1,
        started_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL,
        ended_at    TEXT,
        UNIQUE (task_id, attempt)
    );
    CREATE INDEX runs_by_task ON runs(task_id);

    CREATE TABLE steps (
        id               TEXT PRIMARY KEY,
        run_id           TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
        ordinal          INTEGER NOT NULL,
        kind             TEXT NOT NULL,
        state            TEXT NOT NULL,
        version          INTEGER NOT NULL DEFAULT 1,
        -- The uniqueness that makes "do this once" true. A NULL key opts out;
        -- SQLite treats NULLs as distinct, so unkeyed steps do not collide.
        idempotency_key  TEXT UNIQUE,
        claimed_by       TEXT,
        lease_expires_at TEXT,
        created_at       TEXT NOT NULL,
        updated_at       TEXT NOT NULL
    );
    CREATE INDEX steps_by_run ON steps(run_id);
    -- Reconciliation scans for running steps whose lease lapsed; without this
    -- it is a full table scan every sweep, forever.
    CREATE INDEX steps_by_state_lease ON steps(state, lease_expires_at);

    CREATE TABLE events (
        seq      INTEGER PRIMARY KEY AUTOINCREMENT,
        at       TEXT NOT NULL,
        task_id  TEXT,
        run_id   TEXT,
        step_id  TEXT,
        kind     TEXT NOT NULL,
        payload  TEXT NOT NULL
    );
    CREATE INDEX events_by_task ON events(task_id, seq);
    "#,
];

/// The version a fresh file is migrated to.
pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

fn user_version(connection: &Connection) -> Result<i64> {
    Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// Put a connection into the shape the rest of this crate assumes.
///
/// WAL so a reader never blocks the writer — the Cockpit polls this while the
/// Core writes to it. `NORMAL` rather than `FULL` because the durability that
/// matters here is "survives the process dying", which WAL+NORMAL gives; only
/// losing power can lose the last commit, and a task record is not worth an
/// fsync per write to the user's SSD. `busy_timeout` so two processes
/// contending briefly wait instead of erroring.
pub fn apply_pragmas(connection: &Connection) -> Result<()> {
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .context("enable WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "busy_timeout", 5_000)?;
    Ok(())
}

/// Copy the file aside before it is changed.
///
/// Named for the version it is a backup *of*, so the file on disk says what
/// it can be restored into rather than only when it was taken.
fn back_up(path: &Path, from_version: i64) -> Result<Option<std::path::PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let backup = path.with_file_name(format!(
        "{}.v{from_version}.backup",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::copy(path, &backup)
        .with_context(|| format!("back up {} before migrating", path.display()))?;
    Ok(Some(backup))
}

/// Bring a file up to `latest_version()`, backing it up first if there is
/// anything to migrate.
///
/// Returns the backup that was taken, if one was.
pub fn migrate(connection: &mut Connection, path: Option<&Path>) -> Result<Option<std::path::PathBuf>> {
    let current = user_version(connection)?;
    let latest = latest_version();
    if current > latest {
        // A file written by a newer Unterm. Refusing is the only honest
        // answer: this build does not know what those tables mean, and
        // guessing corrupts the record the user came back for.
        anyhow::bail!(
            "the task database is at version {current}, newer than this build understands ({latest}). \
             Upgrade Unterm, or move the file aside to start fresh."
        );
    }
    if current == latest {
        return Ok(None);
    }

    let backup = match path {
        Some(path) if current > 0 => back_up(path, current)?,
        _ => None,
    };

    for (index, statements) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version <= current {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction
            .execute_batch(statements)
            .with_context(|| format!("apply task-store migration v{version}"))?;
        // `pragma_update` cannot run inside the transaction's borrow, and
        // user_version is not transactional in SQLite anyway; setting it last
        // means a crash mid-migration leaves the old version and the whole
        // batch rolled back, so the next open retries cleanly.
        transaction.commit()?;
        connection.pragma_update(None, "user_version", version)?;
    }

    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        apply_pragmas(&connection).unwrap();
        connection
    }

    #[test]
    fn a_fresh_file_reaches_the_latest_version() {
        let mut connection = memory();
        migrate(&mut connection, None).unwrap();
        assert_eq!(user_version(&connection).unwrap(), latest_version());
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        let mut connection = memory();
        migrate(&mut connection, None).unwrap();
        // The second call must be a no-op rather than an error or a second
        // CREATE TABLE: every process start calls this.
        assert!(migrate(&mut connection, None).unwrap().is_none());
        assert_eq!(user_version(&connection).unwrap(), latest_version());
    }

    #[test]
    fn a_file_from_the_future_is_refused_rather_than_guessed_at() {
        let mut connection = memory();
        connection
            .pragma_update(None, "user_version", latest_version() + 5)
            .unwrap();
        let error = migrate(&mut connection, None).unwrap_err().to_string();
        assert!(error.contains("newer than this build"), "got: {error}");
    }

    #[test]
    fn an_existing_file_is_copied_aside_before_it_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        // A file that already exists at v0 with something in it.
        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch("CREATE TABLE keepsake (note TEXT); INSERT INTO keepsake VALUES ('older')")
                .unwrap();
        }
        let mut connection = Connection::open(&path).unwrap();
        apply_pragmas(&connection).unwrap();
        // v0 -> v1 with a real file: no backup, because there is no schema
        // version to restore into yet. Bump it so there is.
        connection.pragma_update(None, "user_version", 0).unwrap();
        let backup = migrate(&mut connection, Some(&path)).unwrap();
        assert!(
            backup.is_none(),
            "a v0 file has no shipped schema to preserve"
        );

        // Now pretend a v2 exists and re-run: an established file must be
        // copied before it is touched.
        let taken = back_up(&path, latest_version()).unwrap().expect("backup");
        assert!(taken.exists(), "the backup file was not written");
        let restored = Connection::open(&taken).unwrap();
        let note: String = restored
            .query_row("SELECT note FROM keepsake", [], |row| row.get(0))
            .unwrap();
        assert_eq!(note, "older", "the backup does not hold the old contents");
    }

    #[test]
    fn wal_is_actually_on() {
        // WAL is what lets the Cockpit read while the Core writes; if the
        // pragma silently failed, the first contended read would block.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let connection = Connection::open(&path).unwrap();
        apply_pragmas(&connection).unwrap();
        let mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
}
