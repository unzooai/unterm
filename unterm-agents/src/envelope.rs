//! Envelope verification: parse + verify Ed25519 sig + freshness checks.
//!
//! Every byte from the network goes through here before any agent-related
//! command runs. If verification fails for any reason — bad signature,
//! unknown key id, expired envelope, min-unterm-version too high — we
//! return an error and the caller falls back to the on-disk cache or
//! finally to the baked-in fallback (see `fetch.rs`).
//!
//! `TRUSTED_KEYS` is baked into the binary at build time from
//! `unterm-agents/keys/trusted.json`. To rotate keys you bake a new
//! binary; old binaries keep working as long as their baked key set
//! contains the current signing key.

use crate::canonical::to_canonical_bytes_excluding;
use crate::errors::AgentError;
use crate::manifest::Envelope;
use base64::Engine;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use std::collections::HashMap;

/// JSON manifest of trusted signing keys, baked at build time.
const TRUSTED_KEYS_JSON: &str = include_str!("../keys/trusted.json");

#[derive(Debug, Deserialize)]
struct TrustedKeyEntry {
    key_id: String,
    /// base64 (standard, padded) — exactly 32 bytes when decoded.
    public_key_b64: String,
    #[serde(default)]
    expires_at: Option<String>,
    /// Free-text annotation kept around so old key entries are easier to
    /// audit ("rotated out 2027-03-14, hardware-key v2"). The struct field
    /// is intentionally unread by the runtime.
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TrustedKeysFile {
    keys: Vec<TrustedKeyEntry>,
}

fn load_trusted_keys() -> Result<HashMap<String, VerifyingKey>, AgentError> {
    let parsed: TrustedKeysFile = serde_json::from_str(TRUSTED_KEYS_JSON)
        .map_err(|e| AgentError::TrustedKeysCorrupt(e.to_string()))?;
    let mut out = HashMap::new();
    for entry in parsed.keys {
        if let Some(exp) = &entry.expires_at {
            if let Ok(t) = DateTime::parse_from_rfc3339(exp) {
                if t < Utc::now() {
                    log::warn!("trusted key {} expired at {}; skipping", entry.key_id, exp);
                    continue;
                }
            }
        }
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&entry.public_key_b64)
            .map_err(|e| AgentError::TrustedKeysCorrupt(format!("base64: {e}")))?;
        let bytes: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| AgentError::TrustedKeysCorrupt("public key must be 32 bytes".into()))?;
        let vk = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| AgentError::TrustedKeysCorrupt(e.to_string()))?;
        out.insert(entry.key_id, vk);
    }
    if out.is_empty() {
        return Err(AgentError::TrustedKeysCorrupt(
            "no usable trusted keys baked in".into(),
        ));
    }
    Ok(out)
}

/// Parse + verify a signed envelope. The raw bytes are what came off the
/// wire; we re-encode canonically before verifying so the verifier matches
/// the signer regardless of how the network response was whitespaced.
pub fn verify_envelope(raw: &[u8]) -> Result<Envelope, AgentError> {
    // Step 1: parse JSON into a Value first — we need both the typed
    // Envelope (to validate semantics) and the raw Value (to compute the
    // canonical bytes-to-verify with the `signature` field stripped).
    let value: serde_json::Value =
        serde_json::from_slice(raw).map_err(|e| AgentError::ParseFailed(e.to_string()))?;
    let envelope: Envelope = serde_json::from_value(value.clone())
        .map_err(|e| AgentError::ParseFailed(e.to_string()))?;

    // Step 2: signature alg + key id.
    if envelope.signature.alg != "ed25519" {
        return Err(AgentError::UnsupportedSigAlg(
            envelope.signature.alg.clone(),
        ));
    }
    let keys = load_trusted_keys()?;
    let vk = keys
        .get(&envelope.signature.key_id)
        .ok_or_else(|| AgentError::UnknownKeyId(envelope.signature.key_id.clone()))?;

    // Step 3: canonical bytes (envelope minus the `signature` field).
    let to_verify = to_canonical_bytes_excluding(&value, "signature");

    // Step 4: decode + verify.
    let sig_raw = base64::engine::general_purpose::STANDARD
        .decode(&envelope.signature.sig)
        .map_err(|e| AgentError::BadSignature(format!("base64: {e}")))?;
    let sig_bytes: [u8; 64] = sig_raw
        .as_slice()
        .try_into()
        .map_err(|_| AgentError::BadSignature("signature must be 64 bytes".into()))?;
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify_strict(&to_verify, &sig)
        .map_err(|e| AgentError::BadSignature(e.to_string()))?;

    // Step 5: freshness.
    let expires = DateTime::parse_from_rfc3339(&envelope.expires_at)
        .map_err(|e| AgentError::ParseFailed(format!("expires_at: {e}")))?;
    if expires < Utc::now() {
        return Err(AgentError::Expired(envelope.expires_at.clone()));
    }

    // Step 6: min unterm version. We accept any version >= min.
    if compare_versions(env!("CARGO_PKG_VERSION"), &envelope.min_unterm_version) < 0 {
        return Err(AgentError::ClientTooOld {
            have: env!("CARGO_PKG_VERSION").into(),
            need: envelope.min_unterm_version.clone(),
        });
    }

    Ok(envelope)
}

/// Naive dotted semver compare. -1 / 0 / 1. Pre-release tags are stripped.
pub fn compare_versions(a: &str, b: &str) -> i32 {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| !c.is_ascii_digit() && c != '.')
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let av = parse(a);
    let bv = parse(b);
    for i in 0..av.len().max(bv.len()) {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        if x < y {
            return -1;
        }
        if x > y {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert_eq!(compare_versions("0.17.0", "0.17.0"), 0);
        assert_eq!(compare_versions("0.17.0", "0.18.0"), -1);
        assert_eq!(compare_versions("0.18.0", "0.17.0"), 1);
        assert_eq!(compare_versions("1.0.0", "0.99.99"), 1);
        // Trailing labels stripped.
        assert_eq!(compare_versions("0.17.0-rc1", "0.17.0"), 0);
    }
}
