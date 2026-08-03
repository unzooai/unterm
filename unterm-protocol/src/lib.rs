//! Shared wire types and version identity for every Unterm process.
//!
//! This crate deliberately has no dependency on the GUI, engine, MCP server,
//! or storage. It is the compatibility boundary used before those components
//! are split into independent processes.

use serde::{Deserialize, Serialize};

/// Installed product version. All shipped binaries obtain it from this crate.
pub const PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Source revision embedded at build time by `build.rs`.
pub const BUILD_COMMIT: &str = env!("UNTERM_BUILD_COMMIT");
/// Unterm local control-plane protocol. Bump the major version for breaking changes.
pub const PROTOCOL_VERSION: &str = "1.0.0";
/// Durable data schema. M0 has no task database yet, so this is the registry schema.
pub const DATA_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRole {
    #[default]
    Gui,
    Cli,
    McpBridge,
    Core,
    BrainAdapter,
    Provider,
}

/// Minimum identity returned by every process handshake.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildHandshake {
    pub product_version: String,
    pub build_commit: String,
    pub protocol_version: String,
    pub data_schema_version: u32,
    pub process_role: ProcessRole,
    pub pid: u32,
    pub started_at: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    /// A pre-M0 peer with the same product version but no protocol metadata.
    Legacy,
    ProductVersionMismatch,
    ProtocolIncompatible,
    DataSchemaIncompatible,
}

impl Compatibility {
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Compatible | Self::Legacy)
    }

    pub fn error_code(self) -> Option<&'static str> {
        match self {
            Self::Compatible | Self::Legacy => None,
            Self::ProductVersionMismatch => Some("product_version_mismatch"),
            Self::ProtocolIncompatible => Some("protocol_incompatible"),
            Self::DataSchemaIncompatible => Some("data_schema_incompatible"),
        }
    }
}

impl BuildHandshake {
    pub fn current(process_role: ProcessRole, pid: u32, started_at: impl Into<String>) -> Self {
        Self {
            product_version: PRODUCT_VERSION.to_string(),
            build_commit: BUILD_COMMIT.to_string(),
            protocol_version: PROTOCOL_VERSION.to_string(),
            data_schema_version: DATA_SCHEMA_VERSION,
            process_role,
            pid,
            started_at: started_at.into(),
        }
    }

    pub fn is_protocol_compatible(&self) -> bool {
        protocol_major(&self.protocol_version) == protocol_major(PROTOCOL_VERSION)
    }

    /// Decide whether this binary may safely talk to `peer`.
    ///
    /// Product versions are intentionally exact during M0 so an upgraded GUI
    /// cannot silently retain an older long-lived bridge. Pre-M0 peers are
    /// accepted only when their legacy `version` still equals this product.
    pub fn compatibility(&self) -> Compatibility {
        if self.product_version != PRODUCT_VERSION {
            return Compatibility::ProductVersionMismatch;
        }
        if self.protocol_version.is_empty() || self.protocol_version == "legacy" {
            return Compatibility::Legacy;
        }
        if !self.is_protocol_compatible() {
            return Compatibility::ProtocolIncompatible;
        }
        if self.data_schema_version > DATA_SCHEMA_VERSION {
            return Compatibility::DataSchemaIncompatible;
        }
        Compatibility::Compatible
    }
}

fn protocol_major(version: &str) -> Option<&str> {
    version.split('.').next().filter(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_contains_every_required_identity_field() {
        let value = serde_json::to_value(BuildHandshake::current(
            ProcessRole::Core,
            42,
            "2026-08-03T00:00:00Z",
        ))
        .unwrap();
        for field in [
            "product_version",
            "build_commit",
            "protocol_version",
            "data_schema_version",
            "process_role",
            "pid",
            "started_at",
        ] {
            assert!(
                value.get(field).is_some(),
                "missing handshake field {field}"
            );
        }
    }

    #[test]
    fn compatibility_is_decided_by_protocol_major() {
        let mut handshake = BuildHandshake::current(ProcessRole::Provider, 7, "now");
        handshake.protocol_version = "1.99.0".into();
        assert!(handshake.is_protocol_compatible());
        handshake.protocol_version = "2.0.0".into();
        assert!(!handshake.is_protocol_compatible());
    }

    #[test]
    fn exact_product_version_prevents_a_stale_bridge() {
        let mut handshake = BuildHandshake::current(ProcessRole::Gui, 7, "now");
        handshake.product_version = "0.57.4".into();
        assert_eq!(
            handshake.compatibility(),
            Compatibility::ProductVersionMismatch
        );
        assert_eq!(
            handshake.compatibility().error_code(),
            Some("product_version_mismatch")
        );
    }

    #[test]
    fn same_version_legacy_peer_remains_usable_during_migration() {
        let mut handshake = BuildHandshake::current(ProcessRole::Gui, 7, "now");
        handshake.protocol_version = "legacy".into();
        handshake.data_schema_version = 0;
        assert_eq!(handshake.compatibility(), Compatibility::Legacy);
        assert!(handshake.compatibility().is_usable());
    }
}
