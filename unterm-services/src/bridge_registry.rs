//! Persistent lifecycle records for long-lived `unterm-cli mcp-stdio` bridges.
//!
//! MCP clients own and respawn bridge processes, so Unterm cannot replace one
//! in-place. It can, however, request a graceful drain. The bridge rejects its
//! next request with a stable compatibility error and exits; the owner then
//! starts the configured (newly installed) binary.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use unterm_protocol::BuildHandshake;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BridgeRecord {
    build: BuildHandshake,
    state: String,
    #[serde(default)]
    drain_reason: Option<String>,
    updated_at: String,
}

pub struct BridgeRegistration {
    path: PathBuf,
}

impl BridgeRegistration {
    pub fn drain_reason(&self) -> Option<String> {
        let record: BridgeRecord = serde_json::from_slice(&fs::read(&self.path).ok()?).ok()?;
        (record.state == "draining")
            .then_some(record.drain_reason)
            .flatten()
    }
}

impl Drop for BridgeRegistration {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn register(build: BuildHandshake) -> Result<BridgeRegistration> {
    register_in(&bridges_dir()?, build)
}

pub fn request_incompatible_drains() -> Result<usize> {
    request_incompatible_drains_in(&bridges_dir()?)
}

fn bridges_dir() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("UNTERM_STATE_DIR") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path).join("bridges"));
        }
    }
    Ok(dirs_next::home_dir()
        .context("could not resolve home directory")?
        .join(".unterm")
        .join("bridges"))
}

fn register_in(dir: &Path, build: BuildHandshake) -> Result<BridgeRegistration> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", build.pid));
    let record = BridgeRecord {
        build,
        state: "active".into(),
        drain_reason: None,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    write_atomic(&path, &record)?;
    Ok(BridgeRegistration { path })
}

fn request_incompatible_drains_in(dir: &Path) -> Result<usize> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let mut requested = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(mut record) = serde_json::from_slice::<BridgeRecord>(&bytes) else {
            continue;
        };
        if !crate::server_info::pid_alive(record.build.pid) {
            let _ = fs::remove_file(&path);
            continue;
        }
        let compatibility = record.build.compatibility();
        if compatibility.is_usable() || record.state == "draining" {
            continue;
        }
        let code = compatibility
            .error_code()
            .unwrap_or("protocol_incompatible");
        record.state = "draining".into();
        record.drain_reason = Some(format!(
            "{code}: bridge {} is incompatible with installed Unterm {}; restart from the configured unterm-cli path",
            record.build.product_version,
            unterm_protocol::PRODUCT_VERSION,
        ));
        record.updated_at = chrono::Utc::now().to_rfc3339();
        write_atomic(&path, &record)?;
        requested += 1;
    }
    Ok(requested)
}

fn write_atomic(path: &Path, value: &BridgeRecord) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unterm_protocol::ProcessRole;

    #[test]
    fn incompatible_live_bridge_is_marked_for_drain() {
        let root = tempfile::tempdir().unwrap();
        let mut build = BuildHandshake::current(ProcessRole::McpBridge, std::process::id(), "now");
        build.product_version = "0.57.4".into();
        let registration = register_in(root.path(), build).unwrap();
        assert_eq!(request_incompatible_drains_in(root.path()).unwrap(), 1);
        let reason = registration.drain_reason().unwrap();
        assert!(reason.starts_with("product_version_mismatch:"), "{reason}");
    }

    #[test]
    fn compatible_bridge_stays_active_and_drop_unregisters_it() {
        let root = tempfile::tempdir().unwrap();
        let build = BuildHandshake::current(ProcessRole::McpBridge, std::process::id(), "now");
        let registration = register_in(root.path(), build).unwrap();
        let path = registration.path.clone();
        assert_eq!(request_incompatible_drains_in(root.path()).unwrap(), 0);
        assert!(registration.drain_reason().is_none());
        drop(registration);
        assert!(!path.exists());
    }
}
