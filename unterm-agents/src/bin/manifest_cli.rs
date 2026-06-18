//! manifest-cli — maintainer signing + publishing tool for the unterm.app
//! AI-agent manifest catalog.
//!
//! NOT for end users. Built only when the `manifest-cli` feature is on:
//!
//!     cargo run -p unterm-agents --features manifest-cli --bin manifest-cli -- keygen
//!
//! Workflow (one-time setup):
//!
//!   1. `manifest-cli keygen --out ~/.unterm-keys/`
//!      → writes `unterm-<DATE>.priv` (32 bytes) + `unterm-<DATE>.pub`
//!      → tells you the base64 pubkey to paste into trusted.json
//!
//!   2. Edit `unterm-agents/keys/trusted.json` to include the new pubkey,
//!      replacing the RFC 8032 placeholder.
//!
//!   3. Store the private-key file in your password manager (or macOS
//!      Keychain). Manifest-cli reads it back from disk each `sign` run.
//!
//! Workflow (per release of manifests):
//!
//!   1. Edit ./manifests/*.json (a directory of agent manifests).
//!   2. `manifest-cli sign --key ~/.unterm-keys/unterm-<DATE>.priv --manifests ./manifests --out envelope.json`
//!   3. `manifest-cli diff --against https://unterm.app/api/agents/manifests`
//!   4. `manifest-cli push --envelope envelope.json --kv-namespace UNTERM_MANIFESTS`
//!      (just runs `wrangler kv key put ... --binding=UNTERM_MANIFESTS current`)
//!
//! The signing key + manifests should live outside the public Unterm
//! repo — see `feedback_unterm_scope.md` and the design doc §3.

use anyhow::{bail, Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use unterm_agents::canonical::to_canonical_bytes_excluding;
use unterm_agents::manifest::{AgentManifest, Envelope, Signature};

#[derive(Parser)]
#[command(version, about = "Maintainer signing + publishing for Unterm AI agent manifests", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a new Ed25519 keypair.
    Keygen {
        #[arg(long, default_value = "unterm-key")]
        name: String,
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    /// Validate a directory of manifest JSON files against the schema.
    Validate {
        #[arg(long, default_value = "./manifests")]
        manifests: PathBuf,
    },
    /// Sign all manifests in a directory and emit a single envelope.json.
    Sign {
        /// Path to the private key file produced by `keygen`.
        #[arg(long)]
        key: PathBuf,
        /// Key id (must match a baked trusted_keys entry on the client).
        #[arg(long)]
        key_id: String,
        /// Directory of *.json manifest files.
        #[arg(long, default_value = "./manifests")]
        manifests: PathBuf,
        /// Output envelope path.
        #[arg(long, default_value = "./envelope.json")]
        out: PathBuf,
        /// How long until the envelope expires, in days. 14 = two weeks.
        #[arg(long, default_value = "14")]
        valid_days: i64,
        /// Minimum Unterm client version that can use this envelope.
        #[arg(long, default_value = "0.17.0")]
        min_unterm_version: String,
    },
    /// Diff a local envelope against the currently-published one.
    Diff {
        #[arg(long, default_value = "./envelope.json")]
        envelope: PathBuf,
        #[arg(long, default_value = "https://unterm.app/api/agents/manifests")]
        against: String,
    },
    /// Push an envelope to Cloudflare KV via wrangler. Wrapper around:
    ///   wrangler kv key put --binding=UNTERM_MANIFESTS current ./envelope.json
    Push {
        #[arg(long, default_value = "./envelope.json")]
        envelope: PathBuf,
        #[arg(long, default_value = "UNTERM_MANIFESTS")]
        kv_namespace_binding: String,
        #[arg(long, default_value = "current")]
        key: String,
        /// If set, also write a copy under archive:<unix-ts> for rollback.
        #[arg(long, default_value_t = true)]
        archive: bool,
    },
    /// Bake the current envelope into the unterm-agents crate as the
    /// fallback shipped with the next binary release.
    Bake {
        #[arg(long, default_value = "./envelope.json")]
        envelope: PathBuf,
        #[arg(long, default_value = "../unterm-agents/baked/manifests-fallback.json")]
        target: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Keygen { name, out } => keygen(&name, &out),
        Cmd::Validate { manifests } => validate(&manifests),
        Cmd::Sign {
            key,
            key_id,
            manifests,
            out,
            valid_days,
            min_unterm_version,
        } => sign(
            &key,
            &key_id,
            &manifests,
            &out,
            valid_days,
            &min_unterm_version,
        ),
        Cmd::Diff { envelope, against } => diff(&envelope, &against),
        Cmd::Push {
            envelope,
            kv_namespace_binding,
            key,
            archive,
        } => push(&envelope, &kv_namespace_binding, &key, archive),
        Cmd::Bake { envelope, target } => bake(&envelope, &target),
    }
}

fn keygen(name: &str, out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let pk = sk.verifying_key();

    let priv_path = out_dir.join(format!("{name}.priv"));
    let pub_path = out_dir.join(format!("{name}.pub"));

    std::fs::write(&priv_path, sk.to_bytes())?;
    let pubkey_b64 = base64::engine::general_purpose::STANDARD.encode(pk.to_bytes());
    std::fs::write(&pub_path, format!("{pubkey_b64}\n"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600))?;
    }

    println!("Generated keypair:");
    println!("  private: {} (chmod 0600)", priv_path.display());
    println!("  public:  {}", pub_path.display());
    println!();
    println!("Paste this into unterm-agents/keys/trusted.json:");
    println!();
    println!("  {{");
    println!("    \"key_id\":         \"{name}\",");
    println!("    \"public_key_b64\": \"{pubkey_b64}\",");
    println!("    \"expires_at\":     null,");
    println!("    \"note\":           \"Generated {}\"", chrono_now_iso());
    println!("  }}");
    println!();
    println!("Then store the .priv file in your password manager and");
    println!("optionally `srm` the on-disk copy.");
    Ok(())
}

fn validate(manifests_dir: &Path) -> Result<()> {
    let entries = read_manifest_files(manifests_dir)?;
    if entries.is_empty() {
        println!("(no manifests found in {})", manifests_dir.display());
        return Ok(());
    }
    for (path, m) in &entries {
        println!(
            "ok  {:<24}  v{}  {}",
            m.id,
            m.version,
            path.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    println!("\n{} manifest(s) validated.", entries.len());
    Ok(())
}

fn sign(
    key_path: &Path,
    key_id: &str,
    manifests_dir: &Path,
    out: &Path,
    valid_days: i64,
    min_unterm_version: &str,
) -> Result<()> {
    let key_bytes = std::fs::read(key_path)
        .with_context(|| format!("reading private key {}", key_path.display()))?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must be exactly 32 bytes"))?;
    let sk = SigningKey::from_bytes(&key_arr);

    let entries = read_manifest_files(manifests_dir)?;
    if entries.is_empty() {
        bail!("no manifests found in {}", manifests_dir.display());
    }
    let manifests: Vec<AgentManifest> = entries.into_iter().map(|(_, m)| m).collect();

    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::days(valid_days);
    let mut envelope_value = serde_json::json!({
        "envelope_version": 1,
        "issued_at":  now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "expires_at": expires.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "min_unterm_version": min_unterm_version,
        "manifests": manifests,
    });

    // Canonicalize and sign.
    let to_sign = to_canonical_bytes_excluding(&envelope_value, "signature");
    let sig = sk.sign(&to_sign);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_bytes());

    if let Value::Object(ref mut map) = envelope_value {
        map.insert(
            "signature".into(),
            serde_json::to_value(Signature {
                alg: "ed25519".into(),
                key_id: key_id.into(),
                sig: sig_b64,
            })?,
        );
    }

    // Round-trip into typed Envelope to ensure it parses, then re-serialize.
    let envelope: Envelope = serde_json::from_value(envelope_value)?;
    let pretty = serde_json::to_string_pretty(&envelope)? + "\n";
    std::fs::write(out, &pretty)?;
    println!(
        "Signed {} manifest(s) → {} ({} bytes, expires {})",
        envelope.manifests.len(),
        out.display(),
        pretty.len(),
        envelope.expires_at
    );
    Ok(())
}

fn diff(envelope_path: &Path, against_url: &str) -> Result<()> {
    let local_bytes = std::fs::read(envelope_path)?;
    let local: Envelope = serde_json::from_slice(&local_bytes)?;
    let remote_bytes = reqwest::blocking::get(against_url)
        .with_context(|| format!("fetching {against_url}"))?
        .bytes()
        .map(|b| b.to_vec())?;
    let remote: Envelope = match serde_json::from_slice::<Envelope>(&remote_bytes) {
        Ok(e) => e,
        Err(_) => {
            println!("(no current envelope at {against_url} — first publish)");
            return Ok(());
        }
    };

    let mut local_map = std::collections::HashMap::new();
    for m in &local.manifests {
        local_map.insert(&m.id, m);
    }
    let mut remote_map = std::collections::HashMap::new();
    for m in &remote.manifests {
        remote_map.insert(&m.id, m);
    }

    let mut all_ids: Vec<&String> = local_map.keys().chain(remote_map.keys()).copied().collect();
    all_ids.sort();
    all_ids.dedup();
    for id in all_ids {
        match (local_map.get(&id), remote_map.get(&id)) {
            (Some(l), Some(r)) if l.version != r.version => {
                println!("M  {id:<20} v{} → v{}", r.version, l.version);
            }
            (Some(_), Some(_)) => {}
            (Some(_), None) => println!("A  {id:<20} (new)"),
            (None, Some(_)) => println!("D  {id:<20} (removed)"),
            (None, None) => {}
        }
    }
    Ok(())
}

fn push(envelope: &Path, binding: &str, key: &str, archive: bool) -> Result<()> {
    if !envelope.exists() {
        bail!("envelope not found: {}", envelope.display());
    }
    println!(
        "wrangler kv key put --binding={binding} {key} {}",
        envelope.display()
    );
    let status = Command::new("wrangler")
        .args(["kv", "key", "put", &format!("--binding={binding}"), key])
        .arg(envelope)
        .status()
        .with_context(|| "wrangler not on PATH — install with `npm i -g wrangler`")?;
    if !status.success() {
        bail!("wrangler push failed: {status}");
    }
    if archive {
        let ts = chrono::Utc::now().timestamp();
        let archive_key = format!("archive:{ts}");
        println!("Archiving as {archive_key}");
        let status = Command::new("wrangler")
            .args([
                "kv",
                "key",
                "put",
                &format!("--binding={binding}"),
                &archive_key,
            ])
            .arg(envelope)
            .status()?;
        if !status.success() {
            bail!("wrangler archive push failed: {status}");
        }
    }
    Ok(())
}

fn bake(envelope: &Path, target: &Path) -> Result<()> {
    let bytes = std::fs::read(envelope)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(target, &bytes)?;
    println!("Baked {} → {}", envelope.display(), target.display());
    Ok(())
}

fn read_manifest_files(dir: &Path) -> Result<Vec<(PathBuf, AgentManifest)>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let manifest: AgentManifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", path.display()))?;
        out.push((path, manifest));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn chrono_now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Serialize, Deserialize)]
struct _Unused;
