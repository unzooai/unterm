//! Where work is allowed to happen, and what it produced.
//!
//! Two records that have nothing in common except that they both landed in
//! v5 and both answer questions about a task after it is over.
//!
//! A **workspace** is a named root. The interesting property — that two of
//! them cannot see each other — is a property of the *set*, not of any one
//! root: no check of one path against one root can establish it. So they are
//! kept together and read whole.
//!
//! An **artifact** is content addressed by its hash, with the bytes on disk
//! and only the index here. A task can produce a screen recording or a
//! download, and a database holding those is one nobody can copy, back up or
//! open.

use serde::{Deserialize, Serialize};

/// A named root that work is confined to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    /// Absolute and canonical, resolved when the workspace was created.
    ///
    /// Storing what the user typed would mean resolving it on every check —
    /// and a symlink resolved at check time is a root that can move out from
    /// under a scope between two checks.
    pub root: String,
    pub created_at: String,
    pub archived_at: Option<String>,
}

impl Workspace {
    pub fn is_live(&self) -> bool {
        self.archived_at.is_none()
    }
}

/// One thing a task produced.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    /// The address of the bytes. Two tasks producing identical content share
    /// one file; the rows stay separate, because provenance is not content.
    pub sha256: String,
    pub bytes: i64,
    pub media_type: Option<String>,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    pub call_id: Option<String>,
    /// Who produced it: `provider.call`, `brain`, `recording`, `upload`.
    pub origin: String,
    /// A human-facing name. Not a path: an artifact is not a file the user
    /// chose the location of.
    pub name: Option<String>,
    pub created_at: String,
}

/// What to record about a new artifact.
#[derive(Clone, Debug, Default)]
pub struct NewArtifact {
    pub sha256: String,
    pub bytes: i64,
    pub media_type: Option<String>,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    pub call_id: Option<String>,
    pub origin: String,
    pub name: Option<String>,
}

/// A hosted agent session, as it survives the process that ran it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub adapter: String,
    pub command: String,
    pub cwd: Option<String>,
    /// The caller's own identifiers, carried through untouched.
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub step_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub lease_id: Option<String>,
    pub state: String,
    pub exit_code: Option<i64>,
    pub signal: Option<String>,
    pub reason: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}
