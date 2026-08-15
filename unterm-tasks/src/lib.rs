//! The durable task engine.
//!
//! Everything the Agent Cockpit knows used to live in process memory and a
//! whole-file JSON rewrite, which meant two things: it disappeared when the
//! process did, and two writers could not both be right. This is the other
//! answer — one SQLite file, one state machine, one monotonically numbered
//! event stream that every reader follows by cursor.
//!
//! Three properties the rest of the system is allowed to rely on:
//!
//! * **A step with an idempotency key runs once.** Asking again returns the
//!   step that already exists, so a retried message cannot repeat a side
//!   effect.
//! * **A step is claimed by exactly one worker.** The claim is a conditional
//!   update, so concurrency is settled by the database rather than by whoever
//!   read first.
//! * **Nothing is lost to a crash.** A worker that dies stops renewing its
//!   lease; reconciliation turns the lapsed claim into `Interrupted`, which
//!   is a verdict a reader can act on rather than a row stuck at `Running`
//!   forever.

pub mod model;
mod schema;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub use model::{Event, Run, RunId, State, Step, StepId, Task, TaskId};

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// What claiming a step gave you.
#[derive(Clone, Debug, PartialEq)]
pub enum Claim {
    /// The step is yours until the lease expires.
    Granted(Step),
    /// Somebody else holds it, or it is no longer pending.
    Denied { held_by: Option<String>, state: State },
}

/// What asking for a step with an idempotency key gave you.
#[derive(Clone, Debug, PartialEq)]
pub enum StepRequest {
    /// There was no such key; this step is new.
    Created(Step),
    /// The key was already used. This is that step, untouched — the caller's
    /// side effect has already happened (or is happening) and must not be
    /// repeated.
    AlreadyExists(Step),
}

impl StepRequest {
    pub fn step(&self) -> &Step {
        match self {
            StepRequest::Created(step) | StepRequest::AlreadyExists(step) => step,
        }
    }

    pub fn is_new(&self) -> bool {
        matches!(self, StepRequest::Created(_))
    }
}

/// The task store.
///
/// One connection behind a mutex rather than a pool: the writer is the Core,
/// the volumes are human-scale, and a single connection makes "did my write
/// land before your read" a question with an answer.
pub struct TaskStore {
    connection: Mutex<Connection>,
    path: Option<PathBuf>,
}

impl TaskStore {
    /// Open (creating if needed) the store at `path`, migrating it forward.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let mut connection = Connection::open(&path)
            .with_context(|| format!("open task store at {}", path.display()))?;
        schema::apply_pragmas(&connection)?;
        if let Some(backup) = schema::migrate(&mut connection, Some(&path))? {
            log_line(&format!("task store backed up to {}", backup.display()));
        }
        Ok(Self {
            connection: Mutex::new(connection),
            path: Some(path),
        })
    }

    /// An in-memory store, for tests and for a Core told to keep nothing.
    pub fn in_memory() -> Result<Self> {
        let mut connection = Connection::open_in_memory()?;
        schema::apply_pragmas(&connection)?;
        schema::migrate(&mut connection, None)?;
        Ok(Self {
            connection: Mutex::new(connection),
            path: None,
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    fn with<T>(&self, f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("the task store lock was poisoned by an earlier panic"))?;
        f(&mut guard)
    }

    // ---- tasks -------------------------------------------------------

    pub fn create_task(&self, kind: &str, title: &str) -> Result<Task> {
        let task = Task {
            id: TaskId::new(),
            kind: kind.to_string(),
            title: title.to_string(),
            state: State::Pending,
            version: 1,
            created_at: now(),
            updated_at: now(),
        };
        self.with(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO tasks (id, kind, title, state, version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    task.id.as_str(),
                    task.kind,
                    task.title,
                    task.state.as_str(),
                    task.version,
                    task.created_at,
                    task.updated_at
                ],
            )?;
            append_event(
                &transaction,
                Some(task.id.as_str()),
                None,
                None,
                "task.created",
                &serde_json::json!({"kind": task.kind, "title": task.title}),
            )?;
            transaction.commit()?;
            Ok(())
        })?;
        Ok(task)
    }

    pub fn task(&self, id: &TaskId) -> Result<Option<Task>> {
        self.with(|connection| {
            connection
                .query_row(
                    "SELECT id, kind, title, state, version, created_at, updated_at
                     FROM tasks WHERE id = ?1",
                    params![id.as_str()],
                    row_to_task,
                )
                .optional()
                .map_err(Into::into)
        })
    }

    pub fn tasks(&self) -> Result<Vec<Task>> {
        self.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, kind, title, state, version, created_at, updated_at
                 FROM tasks ORDER BY created_at, id",
            )?;
            let rows = statement.query_map([], row_to_task)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// Move a task to `next`, refusing an illegal edge and refusing a write
    /// based on a version the caller no longer has.
    pub fn set_task_state(&self, id: &TaskId, next: State, expected_version: i64) -> Result<Task> {
        self.transition("tasks", id.as_str(), next, expected_version, |connection| {
            connection.query_row(
                "SELECT id, kind, title, state, version, created_at, updated_at
                 FROM tasks WHERE id = ?1",
                params![id.as_str()],
                row_to_task,
            )
        })
    }

    // ---- runs --------------------------------------------------------

    /// Start another attempt at a task. Attempts number from 1 and never
    /// reuse a number, so "the third try" always means the same run.
    pub fn start_run(&self, task_id: &TaskId) -> Result<Run> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let attempt: i64 = transaction.query_row(
                "SELECT COALESCE(MAX(attempt), 0) + 1 FROM runs WHERE task_id = ?1",
                params![task_id.as_str()],
                |row| row.get(0),
            )?;
            let run = Run {
                id: RunId::new(),
                task_id: task_id.clone(),
                attempt,
                state: State::Running,
                version: 1,
                started_at: now(),
                updated_at: now(),
                ended_at: None,
            };
            transaction.execute(
                "INSERT INTO runs (id, task_id, attempt, state, version, started_at, updated_at, ended_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
                params![
                    run.id.as_str(),
                    run.task_id.as_str(),
                    run.attempt,
                    run.state.as_str(),
                    run.version,
                    run.started_at,
                    run.updated_at
                ],
            )?;
            append_event(
                &transaction,
                Some(task_id.as_str()),
                Some(run.id.as_str()),
                None,
                "run.started",
                &serde_json::json!({"attempt": attempt}),
            )?;
            transaction.commit()?;
            Ok(run)
        })
    }

    pub fn runs(&self, task_id: &TaskId) -> Result<Vec<Run>> {
        self.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, task_id, attempt, state, version, started_at, updated_at, ended_at
                 FROM runs WHERE task_id = ?1 ORDER BY attempt",
            )?;
            let rows = statement.query_map(params![task_id.as_str()], row_to_run)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    pub fn finish_run(&self, id: &RunId, next: State, expected_version: i64) -> Result<Run> {
        let run = self.transition("runs", id.as_str(), next, expected_version, |connection| {
            connection.query_row(
                "SELECT id, task_id, attempt, state, version, started_at, updated_at, ended_at
                 FROM runs WHERE id = ?1",
                params![id.as_str()],
                row_to_run,
            )
        })?;
        if next.is_terminal() {
            self.with(|connection| {
                connection.execute(
                    "UPDATE runs SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL",
                    params![id.as_str(), now()],
                )?;
                Ok(())
            })?;
        }
        Ok(run)
    }

    // ---- steps -------------------------------------------------------

    /// Ask for a step. With an idempotency key, asking twice gets the same
    /// step back rather than a second one.
    pub fn request_step(
        &self,
        run_id: &RunId,
        kind: &str,
        idempotency_key: Option<&str>,
    ) -> Result<StepRequest> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            if let Some(key) = idempotency_key {
                let existing = transaction
                    .query_row(
                        "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                                claimed_by, lease_expires_at, created_at, updated_at
                         FROM steps WHERE idempotency_key = ?1",
                        params![key],
                        row_to_step,
                    )
                    .optional()?;
                if let Some(step) = existing {
                    // Deliberately not touching it: the caller is repeating
                    // itself, and the record of what actually happened
                    // belongs to the first request.
                    transaction.commit()?;
                    return Ok(StepRequest::AlreadyExists(step));
                }
            }
            let ordinal: i64 = transaction.query_row(
                "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM steps WHERE run_id = ?1",
                params![run_id.as_str()],
                |row| row.get(0),
            )?;
            let step = Step {
                id: StepId::new(),
                run_id: run_id.clone(),
                ordinal,
                kind: kind.to_string(),
                state: State::Pending,
                version: 1,
                idempotency_key: idempotency_key.map(str::to_string),
                claimed_by: None,
                lease_expires_at: None,
                created_at: now(),
                updated_at: now(),
            };
            transaction.execute(
                "INSERT INTO steps (id, run_id, ordinal, kind, state, version, idempotency_key,
                                    claimed_by, lease_expires_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, ?8, ?9)",
                params![
                    step.id.as_str(),
                    step.run_id.as_str(),
                    step.ordinal,
                    step.kind,
                    step.state.as_str(),
                    step.version,
                    step.idempotency_key,
                    step.created_at,
                    step.updated_at
                ],
            )?;
            append_event(
                &transaction,
                None,
                Some(run_id.as_str()),
                Some(step.id.as_str()),
                "step.requested",
                &serde_json::json!({"kind": kind, "ordinal": ordinal}),
            )?;
            transaction.commit()?;
            Ok(StepRequest::Created(step))
        })
    }

    /// Take a pending step, for `lease` seconds.
    ///
    /// The whole decision is one conditional UPDATE: whoever the database
    /// applies first wins, and the loser is told who holds it. Reading then
    /// writing would let two workers both see `pending` and both proceed.
    pub fn claim_step(&self, id: &StepId, worker: &str, lease_seconds: i64) -> Result<Claim> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let expires = (chrono::Utc::now() + chrono::Duration::seconds(lease_seconds))
                .to_rfc3339();
            let changed = transaction.execute(
                "UPDATE steps
                    SET state = 'running', claimed_by = ?2, lease_expires_at = ?3,
                        version = version + 1, updated_at = ?4
                  WHERE id = ?1 AND state = 'pending'",
                params![id.as_str(), worker, expires, now()],
            )?;
            let step = transaction.query_row(
                "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                        claimed_by, lease_expires_at, created_at, updated_at
                 FROM steps WHERE id = ?1",
                params![id.as_str()],
                row_to_step,
            )?;
            if changed == 0 {
                transaction.commit()?;
                return Ok(Claim::Denied {
                    held_by: step.claimed_by.clone(),
                    state: step.state,
                });
            }
            append_event(
                &transaction,
                None,
                Some(step.run_id.as_str()),
                Some(step.id.as_str()),
                "step.claimed",
                &serde_json::json!({"worker": worker, "lease_expires_at": expires}),
            )?;
            transaction.commit()?;
            Ok(Claim::Granted(step))
        })
    }

    /// Push the lease out. A worker that is alive says so by doing this; one
    /// that stops is what reconciliation notices.
    pub fn heartbeat_step(&self, id: &StepId, worker: &str, lease_seconds: i64) -> Result<bool> {
        self.with(|connection| {
            let expires = (chrono::Utc::now() + chrono::Duration::seconds(lease_seconds))
                .to_rfc3339();
            let changed = connection.execute(
                "UPDATE steps SET lease_expires_at = ?3, updated_at = ?4
                  WHERE id = ?1 AND claimed_by = ?2 AND state = 'running'",
                params![id.as_str(), worker, expires, now()],
            )?;
            // Deliberately not bumping `version`: a heartbeat is not a change
            // to what the step *is*, and bumping it would make every reader's
            // compare-and-swap lose a race against a liveness ping.
            Ok(changed == 1)
        })
    }

    pub fn step(&self, id: &StepId) -> Result<Option<Step>> {
        self.with(|connection| {
            connection
                .query_row(
                    "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                            claimed_by, lease_expires_at, created_at, updated_at
                     FROM steps WHERE id = ?1",
                    params![id.as_str()],
                    row_to_step,
                )
                .optional()
                .map_err(Into::into)
        })
    }

    pub fn steps(&self, run_id: &RunId) -> Result<Vec<Step>> {
        self.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                        claimed_by, lease_expires_at, created_at, updated_at
                 FROM steps WHERE run_id = ?1 ORDER BY ordinal",
            )?;
            let rows = statement.query_map(params![run_id.as_str()], row_to_step)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    pub fn finish_step(&self, id: &StepId, next: State, expected_version: i64) -> Result<Step> {
        let step = self.transition("steps", id.as_str(), next, expected_version, |connection| {
            connection.query_row(
                "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                        claimed_by, lease_expires_at, created_at, updated_at
                 FROM steps WHERE id = ?1",
                params![id.as_str()],
                row_to_step,
            )
        })?;
        if next.is_terminal() {
            // The claim is over; leaving it set would make a finished step
            // look held, and reconciliation would keep considering it.
            self.with(|connection| {
                connection.execute(
                    "UPDATE steps SET claimed_by = NULL, lease_expires_at = NULL WHERE id = ?1",
                    params![id.as_str()],
                )?;
                Ok(())
            })?;
        }
        Ok(step)
    }

    // ---- recovery ----------------------------------------------------

    /// Turn claims whose lease has lapsed into `Interrupted`.
    ///
    /// This is what makes a hard kill survivable. The worker that held the
    /// step is gone, so nobody will ever report its verdict; leaving the row
    /// at `Running` means a Cockpit that shows work in progress forever.
    /// `Interrupted` says what is actually known — it started, and nobody can
    /// say how it ended — and a retry starts a new run rather than pretending
    /// this one continued.
    ///
    /// Returns the steps it reclaimed.
    pub fn reconcile(&self) -> Result<Vec<Step>> {
        self.with(|connection| {
            let cutoff = now();
            let transaction = connection.transaction()?;
            let lapsed: Vec<String> = {
                let mut statement = transaction.prepare(
                    "SELECT id FROM steps
                      WHERE state = 'running'
                        AND lease_expires_at IS NOT NULL
                        AND lease_expires_at < ?1",
                )?;
                let rows = statement.query_map(params![cutoff], |row| row.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            let mut reclaimed = Vec::new();
            for id in lapsed {
                transaction.execute(
                    "UPDATE steps
                        SET state = 'interrupted', version = version + 1, updated_at = ?2,
                            claimed_by = NULL, lease_expires_at = NULL
                      WHERE id = ?1",
                    params![id, now()],
                )?;
                let step = transaction.query_row(
                    "SELECT id, run_id, ordinal, kind, state, version, idempotency_key,
                            claimed_by, lease_expires_at, created_at, updated_at
                     FROM steps WHERE id = ?1",
                    params![id],
                    row_to_step,
                )?;
                append_event(
                    &transaction,
                    None,
                    Some(step.run_id.as_str()),
                    Some(step.id.as_str()),
                    "step.interrupted",
                    &serde_json::json!({"reason": "lease expired"}),
                )?;
                reclaimed.push(step);
            }
            transaction.commit()?;
            Ok(reclaimed)
        })
    }

    // ---- events ------------------------------------------------------

    /// Everything after `cursor`, oldest first. A subscriber remembers the
    /// `seq` of the last event it handled and passes it back.
    pub fn events_since(&self, cursor: i64, limit: i64) -> Result<Vec<Event>> {
        self.with(|connection| {
            let mut statement = connection.prepare(
                "SELECT seq, at, task_id, run_id, step_id, kind, payload
                 FROM events WHERE seq > ?1 ORDER BY seq LIMIT ?2",
            )?;
            let rows = statement.query_map(params![cursor, limit], row_to_event)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })
    }

    /// The newest cursor, for a reader that wants only what happens next.
    pub fn latest_cursor(&self) -> Result<i64> {
        self.with(|connection| {
            Ok(connection.query_row("SELECT COALESCE(MAX(seq), 0) FROM events", [], |row| {
                row.get(0)
            })?)
        })
    }

    // ---- shared transition ------------------------------------------

    /// The one place a state changes.
    ///
    /// Both guards matter and they catch different mistakes: the edge check
    /// stops a caller inventing a transition the machine does not have, and
    /// the version check stops a caller acting on a row that moved under it.
    fn transition<T>(
        &self,
        table: &'static str,
        id: &str,
        next: State,
        expected_version: i64,
        load: impl Fn(&Connection) -> rusqlite::Result<T>,
    ) -> Result<T> {
        self.with(|connection| {
            let transaction = connection.transaction()?;
            let (current, version): (String, i64) = transaction
                .query_row(
                    &format!("SELECT state, version FROM {table} WHERE id = ?1"),
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| anyhow::anyhow!("no {table} row with id {id}"))?;
            let current = State::parse(&current)?;
            if version != expected_version {
                anyhow::bail!(
                    "{table} {id} is at version {version}, not {expected_version}; \
                     something changed it since you read it"
                );
            }
            if !next.may_follow(current) {
                anyhow::bail!(
                    "{table} {id} cannot go from {} to {}",
                    current.as_str(),
                    next.as_str()
                );
            }
            let changed = transaction.execute(
                &format!(
                    "UPDATE {table} SET state = ?2, version = version + 1, updated_at = ?3
                      WHERE id = ?1 AND version = ?4"
                ),
                params![id, next.as_str(), now(), expected_version],
            )?;
            if changed != 1 {
                anyhow::bail!("{table} {id} was changed by someone else mid-transition");
            }
            append_event(
                &transaction,
                (table == "tasks").then_some(id),
                (table == "runs").then_some(id),
                (table == "steps").then_some(id),
                &format!("{}.{}", table.trim_end_matches('s'), next.as_str()),
                &serde_json::json!({"from": current.as_str(), "to": next.as_str()}),
            )?;
            let loaded = load(&transaction)?;
            transaction.commit()?;
            Ok(loaded)
        })
    }
}

fn append_event(
    connection: &Connection,
    task_id: Option<&str>,
    run_id: Option<&str>,
    step_id: Option<&str>,
    kind: &str,
    payload: &serde_json::Value,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO events (at, task_id, run_id, step_id, kind, payload)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![now(), task_id, run_id, step_id, kind, payload.to_string()],
    )?;
    Ok(())
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: TaskId::parse(&row.get::<_, String>(0)?).map_err(to_sqlite_error)?,
        kind: row.get(1)?,
        title: row.get(2)?,
        state: State::parse(&row.get::<_, String>(3)?).map_err(to_sqlite_error)?,
        version: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    Ok(Run {
        id: RunId::parse(&row.get::<_, String>(0)?).map_err(to_sqlite_error)?,
        task_id: TaskId::parse(&row.get::<_, String>(1)?).map_err(to_sqlite_error)?,
        attempt: row.get(2)?,
        state: State::parse(&row.get::<_, String>(3)?).map_err(to_sqlite_error)?,
        version: row.get(4)?,
        started_at: row.get(5)?,
        updated_at: row.get(6)?,
        ended_at: row.get(7)?,
    })
}

fn row_to_step(row: &rusqlite::Row<'_>) -> rusqlite::Result<Step> {
    Ok(Step {
        id: StepId::parse(&row.get::<_, String>(0)?).map_err(to_sqlite_error)?,
        run_id: RunId::parse(&row.get::<_, String>(1)?).map_err(to_sqlite_error)?,
        ordinal: row.get(2)?,
        kind: row.get(3)?,
        state: State::parse(&row.get::<_, String>(4)?).map_err(to_sqlite_error)?,
        version: row.get(5)?,
        idempotency_key: row.get(6)?,
        claimed_by: row.get(7)?,
        lease_expires_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let payload: String = row.get(6)?;
    Ok(Event {
        seq: row.get(0)?,
        at: row.get(1)?,
        task_id: row
            .get::<_, Option<String>>(2)?
            .and_then(|raw| TaskId::parse(&raw).ok()),
        run_id: row
            .get::<_, Option<String>>(3)?
            .and_then(|raw| RunId::parse(&raw).ok()),
        step_id: row
            .get::<_, Option<String>>(4)?
            .and_then(|raw| StepId::parse(&raw).ok()),
        kind: row.get(5)?,
        payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
    })
}

fn to_sqlite_error(error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, error.into())
}

/// The store has no logger of its own; the Core prints what it says.
fn log_line(message: &str) {
    eprintln!("unterm-tasks: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> TaskStore {
        TaskStore::in_memory().expect("in-memory store")
    }

    /// A task, one attempt at it, one step inside that — the shape everything
    /// else is built from.
    fn scaffold(store: &TaskStore) -> (Task, Run) {
        let task = store.create_task("agent", "write the tests").unwrap();
        let run = store.start_run(&task.id).unwrap();
        (task, run)
    }

    #[test]
    fn a_task_runs_and_the_event_stream_says_so_in_order() {
        let store = store();
        let (task, run) = scaffold(&store);
        let step = store
            .request_step(&run.id, "tool.call", None)
            .unwrap()
            .step()
            .clone();

        assert_eq!(task.state, State::Pending);
        assert_eq!(run.attempt, 1);
        assert_eq!(step.ordinal, 1);

        let events = store.events_since(0, 100).unwrap();
        let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["task.created", "run.started", "step.requested"]);
        // The cursor is what every reader follows; it must only go up.
        let seqs: Vec<i64> = events.iter().map(|e| e.seq).collect();
        let mut sorted = seqs.clone();
        sorted.sort_unstable();
        assert_eq!(seqs, sorted, "the event cursor went backwards");
        assert_eq!(store.latest_cursor().unwrap(), *seqs.last().unwrap());

        // Reading from a cursor returns only what came after it.
        let tail = store.events_since(seqs[0], 100).unwrap();
        assert_eq!(tail.len(), 2);
        assert!(tail.iter().all(|e| e.seq > seqs[0]));
    }

    #[test]
    fn the_same_idempotency_key_never_makes_a_second_step() {
        // The gate: "相同幂等键不重复副作用". A retried message must not run
        // the tool twice, and the store — not the caller — is what enforces
        // it, because the caller is the thing that just retried.
        let store = store();
        let (_task, run) = scaffold(&store);

        let first = store
            .request_step(&run.id, "shell.write", Some("call-42"))
            .unwrap();
        assert!(first.is_new(), "the first request creates the step");

        let second = store
            .request_step(&run.id, "shell.write", Some("call-42"))
            .unwrap();
        assert!(
            !second.is_new(),
            "the second request must not create a second step"
        );
        assert_eq!(
            first.step().id,
            second.step().id,
            "the repeat must name the step that already exists"
        );
        assert_eq!(store.steps(&run.id).unwrap().len(), 1);

        // A different key is a different request, and no key at all opts out
        // of the whole mechanism rather than colliding with the others.
        assert!(store
            .request_step(&run.id, "shell.write", Some("call-43"))
            .unwrap()
            .is_new());
        assert!(store.request_step(&run.id, "shell.write", None).unwrap().is_new());
        assert!(store.request_step(&run.id, "shell.write", None).unwrap().is_new());
        assert_eq!(store.steps(&run.id).unwrap().len(), 4);
    }

    #[test]
    fn a_repeated_request_does_not_disturb_the_step_already_running() {
        let store = store();
        let (_task, run) = scaffold(&store);
        let step = store
            .request_step(&run.id, "tool", Some("once"))
            .unwrap()
            .step()
            .clone();
        assert!(matches!(
            store.claim_step(&step.id, "worker-a", 60).unwrap(),
            Claim::Granted(_)
        ));

        let again = store.request_step(&run.id, "tool", Some("once")).unwrap();
        let seen = again.step();
        assert!(!again.is_new());
        // The retry must not reset it to pending, or a second worker could
        // claim work that is already in flight.
        assert_eq!(seen.state, State::Running);
        assert_eq!(seen.claimed_by.as_deref(), Some("worker-a"));
    }

    #[test]
    fn only_one_worker_can_claim_a_step() {
        let store = store();
        let (_task, run) = scaffold(&store);
        let step = store.request_step(&run.id, "tool", None).unwrap().step().clone();

        let first = store.claim_step(&step.id, "worker-a", 60).unwrap();
        let second = store.claim_step(&step.id, "worker-b", 60).unwrap();

        assert!(matches!(first, Claim::Granted(_)));
        match second {
            Claim::Denied { held_by, state } => {
                assert_eq!(held_by.as_deref(), Some("worker-a"));
                assert_eq!(state, State::Running);
            }
            Claim::Granted(_) => panic!("two workers were both told they own the step"),
        }
    }

    #[test]
    fn concurrent_workers_across_connections_still_produce_one_winner() {
        // The gate: "并发 Worker 不重复执行 Step". The single-connection test
        // above cannot see the real race — two processes, two connections,
        // one row. This one opens the same file twice, which is how the Core
        // and a worker actually meet.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let store = TaskStore::open(&path).unwrap();
        let (_task, run) = scaffold(&store);
        let step = store.request_step(&run.id, "tool", None).unwrap().step().clone();

        let mut granted = 0;
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|worker| {
                    let path = path.clone();
                    let step = step.id.clone();
                    scope.spawn(move || {
                        let store = TaskStore::open(&path).expect("second connection");
                        matches!(
                            store.claim_step(&step, &format!("worker-{worker}"), 60),
                            Ok(Claim::Granted(_))
                        )
                    })
                })
                .collect();
            for handle in handles {
                if handle.join().unwrap() {
                    granted += 1;
                }
            }
        });

        assert_eq!(granted, 1, "{granted} workers were each told the step was theirs");
        let step = store.step(&step.id).unwrap().unwrap();
        assert_eq!(step.state, State::Running);
        assert!(step.claimed_by.is_some());
    }

    #[test]
    fn a_stale_version_loses_instead_of_overwriting() {
        let store = store();
        let task = store.create_task("agent", "t").unwrap();
        let running = store
            .set_task_state(&task.id, State::Running, task.version)
            .unwrap();
        assert_eq!(running.version, task.version + 1);

        // Someone still holding the old version tries to act on it. Letting
        // this through is how a cancel silently undoes a completion.
        let error = store
            .set_task_state(&task.id, State::Succeeded, task.version)
            .unwrap_err()
            .to_string();
        assert!(error.contains("version"), "got: {error}");
        assert_eq!(store.task(&task.id).unwrap().unwrap().state, State::Running);
    }

    #[test]
    fn the_store_refuses_a_transition_the_machine_does_not_have() {
        let store = store();
        let task = store.create_task("agent", "t").unwrap();
        // Pending -> Succeeded skips ever having run.
        let error = store
            .set_task_state(&task.id, State::Succeeded, task.version)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cannot go from"), "got: {error}");

        let running = store
            .set_task_state(&task.id, State::Running, task.version)
            .unwrap();
        let done = store
            .set_task_state(&task.id, State::Succeeded, running.version)
            .unwrap();
        // And terminal is terminal.
        let error = store
            .set_task_state(&task.id, State::Failed, done.version)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cannot go from"), "got: {error}");
    }

    #[test]
    fn a_lapsed_lease_becomes_interrupted_rather_than_running_forever() {
        let store = store();
        let (_task, run) = scaffold(&store);
        let alive = store.request_step(&run.id, "tool", None).unwrap().step().clone();
        let doomed = store.request_step(&run.id, "tool", None).unwrap().step().clone();

        store.claim_step(&alive.id, "worker-a", 600).unwrap();
        // A lease that expired a second ago: the worker holding it is gone.
        store.claim_step(&doomed.id, "worker-b", -1).unwrap();

        let reclaimed = store.reconcile().unwrap();
        assert_eq!(reclaimed.len(), 1, "reconcile took the wrong number of steps");
        assert_eq!(reclaimed[0].id, doomed.id);
        assert_eq!(reclaimed[0].state, State::Interrupted);
        assert!(
            reclaimed[0].claimed_by.is_none(),
            "an interrupted step must not still look held"
        );
        // The live worker is untouched: reconciliation that stops healthy work
        // is worse than the stall it is fixing.
        assert_eq!(store.step(&alive.id).unwrap().unwrap().state, State::Running);

        // And it is said out loud, so a Cockpit following the cursor learns.
        let kinds: Vec<String> = store
            .events_since(0, 100)
            .unwrap()
            .into_iter()
            .map(|e| e.kind)
            .collect();
        assert!(kinds.iter().any(|k| k == "step.interrupted"), "{kinds:?}");
    }

    #[test]
    fn a_heartbeat_keeps_a_claim_alive_without_disturbing_its_version() {
        let store = store();
        let (_task, run) = scaffold(&store);
        let step = store.request_step(&run.id, "tool", None).unwrap().step().clone();
        // Claim with an already-expired lease, then prove a heartbeat rescues
        // it before reconciliation can take it.
        store.claim_step(&step.id, "worker-a", -1).unwrap();
        let claimed = store.step(&step.id).unwrap().unwrap();

        assert!(store.heartbeat_step(&step.id, "worker-a", 600).unwrap());
        assert!(store.reconcile().unwrap().is_empty(), "a beating worker was reaped");

        let after = store.step(&step.id).unwrap().unwrap();
        assert_eq!(
            after.version, claimed.version,
            "a liveness ping is not a change to what the step is, and bumping \
             the version would make every reader's compare-and-swap lose to it"
        );
        // Somebody else's heartbeat is not accepted.
        assert!(!store.heartbeat_step(&step.id, "worker-b", 600).unwrap());
    }

    #[test]
    fn every_state_survives_the_process_dying_underneath_it() {
        // The gate: "逐状态强杀恢复". Dropping the store is what a hard kill
        // leaves behind — no flush, no cleanup, no chance to write a verdict.
        // Reopening must find exactly what was committed, and reconciliation
        // must turn the one ambiguous case into a verdict.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");

        let mut expected = Vec::new();
        {
            let store = TaskStore::open(&path).unwrap();
            for state in [
                State::Pending,
                State::Running,
                State::Succeeded,
                State::Failed,
                State::Cancelled,
            ] {
                let task = store.create_task("agent", "survivor").unwrap();
                let mut current = task.clone();
                if state != State::Pending {
                    current = store
                        .set_task_state(&task.id, State::Running, current.version)
                        .unwrap();
                }
                if state != State::Pending && state != State::Running {
                    current = store
                        .set_task_state(&task.id, state, current.version)
                        .unwrap();
                }
                expected.push((current.id.clone(), state));
            }
            // Plus a step left mid-flight by a worker that never came back.
            let (_t, run) = scaffold(&store);
            let step = store.request_step(&run.id, "tool", None).unwrap().step().clone();
            store.claim_step(&step.id, "doomed-worker", -1).unwrap();
            // No drop ceremony: this is the kill.
        }

        let store = TaskStore::open(&path).unwrap();
        for (id, state) in expected {
            let task = store.task(&id).unwrap().expect("task survived the kill");
            assert_eq!(task.state, state, "task {id} came back in the wrong state");
        }
        let reclaimed = store.reconcile().unwrap();
        assert_eq!(
            reclaimed.len(),
            1,
            "the step whose worker died must come back as a verdict, not a stall"
        );
        assert_eq!(reclaimed[0].state, State::Interrupted);
    }

    #[test]
    fn retrying_adds_an_attempt_instead_of_rewriting_the_failed_one() {
        let store = store();
        let task = store.create_task("agent", "flaky").unwrap();
        let first = store.start_run(&task.id).unwrap();
        let finished = store
            .finish_run(&first.id, State::Failed, first.version)
            .unwrap();
        assert_eq!(finished.state, State::Failed);

        let second = store.start_run(&task.id).unwrap();
        assert_eq!(second.attempt, 2);

        let runs = store.runs(&task.id).unwrap();
        assert_eq!(runs.len(), 2);
        // The record of the failure is exactly as it was left: a retry that
        // edits the previous attempt destroys the evidence of what went wrong.
        assert_eq!(runs[0].state, State::Failed);
        assert!(runs[0].ended_at.is_some(), "a finished run records when");
        assert_eq!(runs[1].state, State::Running);
        assert!(runs[1].ended_at.is_none());
    }

    #[test]
    fn a_finished_step_stops_looking_claimed() {
        let store = store();
        let (_task, run) = scaffold(&store);
        let step = store.request_step(&run.id, "tool", None).unwrap().step().clone();
        let Claim::Granted(claimed) = store.claim_step(&step.id, "worker-a", 600).unwrap() else {
            panic!("the claim was refused");
        };
        let done = store
            .finish_step(&claimed.id, State::Succeeded, claimed.version)
            .unwrap();
        assert_eq!(done.state, State::Succeeded);

        let after = store.step(&step.id).unwrap().unwrap();
        assert!(after.claimed_by.is_none(), "a finished step still looks held");
        assert!(after.lease_expires_at.is_none());
        // And reconciliation has no interest in it any more.
        assert!(store.reconcile().unwrap().is_empty());
    }

    #[test]
    fn the_file_keeps_everything_across_a_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let (task_id, cursor) = {
            let store = TaskStore::open(&path).unwrap();
            let (task, run) = scaffold(&store);
            store.request_step(&run.id, "tool", Some("k")).unwrap();
            (task.id, store.latest_cursor().unwrap())
        };
        let store = TaskStore::open(&path).unwrap();
        assert!(store.task(&task_id).unwrap().is_some());
        assert_eq!(store.latest_cursor().unwrap(), cursor, "the cursor restarted");
        // The idempotency key outlives the process too, which is the only way
        // it can protect against a retry that follows a crash.
        let run = store.runs(&task_id).unwrap().remove(0);
        assert!(!store.request_step(&run.id, "tool", Some("k")).unwrap().is_new());
    }
}
