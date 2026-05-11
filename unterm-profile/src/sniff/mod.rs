//! Auto-discovery of credentials already on disk.
//!
//! All seven sniffers in this module are **read-only**: they parse
//! existing config files (`~/.aws/credentials`, `~/.npmrc`, etc.) and
//! emit [`Candidate`]s for the onboarding wizard to display. Nothing
//! here writes to the user's tools — that's reserved for the
//! wizard's explicit "Create profile" step, which copies values into
//! the OS keychain under our own `unterm` service and never touches
//! the source files.
//!
//! Design note: a candidate represents one *credential entry*, not
//! one *profile*. A real-world `~/.aws/credentials` typically has
//! several `[profile_name]` sections; each becomes one Candidate.
//! The wizard then groups Candidates into Unterm profiles via a
//! drag-and-drop UI (or by running suggested groupings — e.g.
//! "alex-acme on github.com" + "acme-work on AWS" auto-grouped into
//! a "Work — Acme" profile when display names suggest a shared
//! identity).
//!
//! Why per-sniffer modules: each source format is its own parsing
//! problem (INI vs YAML vs JSON vs custom line-format). Keeping them
//! isolated means a typo in one sniffer can't break the others, and
//! adding an 8th source (Stripe / Vercel CLI / Cloudflare wrangler /
//! ...) is a single-file PR.

pub mod aws;
pub mod docker;
pub mod gcloud;
pub mod gh;
pub mod netrc;
pub mod npm;
pub mod ssh;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which on-disk source a credential was discovered in. Surfaced in
/// the wizard so a user picking between two GitHub PATs can tell
/// "the one from `gh` CLI" apart from "the one from `.npmrc`".
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSource {
    Gh,
    Aws,
    Npm,
    Ssh,
    Docker,
    Gcloud,
    Netrc,
}

impl CandidateSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CandidateSource::Gh => "gh",
            CandidateSource::Aws => "aws",
            CandidateSource::Npm => "npm",
            CandidateSource::Ssh => "ssh",
            CandidateSource::Docker => "docker",
            CandidateSource::Gcloud => "gcloud",
            CandidateSource::Netrc => "netrc",
        }
    }
}

/// One discovered credential. The wizard renders one row per
/// Candidate, with the user checking which to import.
///
/// `suggested_value` is `None` when the source identifies an account
/// but doesn't expose the secret in a way we can read directly — the
/// canonical example is `gh` CLI which stores the OAuth token in the
/// OS keychain itself, so we know "alex-acme exists on github.com"
/// but the wizard has to prompt the user to paste the token.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub source: CandidateSource,
    /// Short human-readable label rendered in the wizard list.
    pub label: String,
    /// Suggested env-var name to expose this credential as inside
    /// spawned shells. E.g. `GITHUB_TOKEN` for gh, `NPM_TOKEN` for
    /// npm. Not guaranteed unique across candidates — the wizard's
    /// grouping step handles collisions.
    pub suggested_env_name: String,
    /// The actual secret bytes, if the source exposed them in
    /// plaintext. `None` means "ask the user".
    pub suggested_value: Option<String>,
    /// Host or service the credential authenticates against, if
    /// relevant. Used by the wizard's suggested-grouping heuristic.
    pub host: Option<String>,
    /// Account / login / username, if the source distinguishes one.
    pub user: Option<String>,
    /// Origin file path. Surfaced in the wizard tooltip so users can
    /// trace any candidate back to where we read it from.
    pub origin: PathBuf,
}

/// Run every sniffer in turn and collect all candidates. Failures
/// from a single sniffer are logged and skipped — one corrupt
/// `~/.npmrc` shouldn't prevent the wizard from offering AWS
/// candidates. Returns an empty Vec when nothing is found, which
/// is the expected outcome on a fresh machine.
pub fn scan_all() -> Vec<Candidate> {
    let mut out = Vec::new();
    // Each sniffer is run independently so one broken file doesn't
    // skip the others. Errors degrade to a debug log — "no creds
    // found" and "file unreadable" are both legitimate outcomes for
    // a vibe coder who simply hasn't used that tool.
    let sniffers: [(&str, fn() -> Result<Vec<Candidate>>); 7] = [
        ("gh", gh::sniff),
        ("aws", aws::sniff),
        ("npm", npm::sniff),
        ("ssh", ssh::sniff),
        ("docker", docker::sniff),
        ("gcloud", gcloud::sniff),
        ("netrc", netrc::sniff),
    ];
    for (name, sniffer) in sniffers {
        match sniffer() {
            Ok(mut v) => out.append(&mut v),
            Err(e) => log::debug!("sniff::{name}: skipped ({e:#})"),
        }
    }
    out
}
