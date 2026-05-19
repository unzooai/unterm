//! Canonical-JSON encoder used for signing + verifying the manifest envelope.
//!
//! We can't just `serde_json::to_string(&envelope)` because the byte sequence
//! that gets signed must be bit-identical to what the verifier produces, and
//! serde_json's default emits map keys in insertion order — Ed25519 signature
//! checks fail at the first differing byte.
//!
//! The format is a strict subset of RFC 8785 (JSON Canonicalization Scheme):
//!   - object keys are sorted lexicographically by their UTF-8 bytes
//!   - no insignificant whitespace
//!   - numbers as serde_json renders them (we never emit floats in the
//!     envelope, so the IEEE-754 corner cases in JCS don't apply)
//!   - strings as serde_json renders them (UTF-8 with \uXXXX for control
//!     chars, which matches JCS)
//!
//! The signing tool (`manifest-cli`) uses an identical implementation; if
//! these ever drift, signed envelopes break. Keep them in sync.

use serde_json::{Map, Value};

/// Re-serialize a Value with object keys sorted, no whitespace.
pub fn to_canonical_bytes(value: &Value) -> Vec<u8> {
    let normalized = normalize(value);
    serde_json::to_vec(&normalized).expect("canonical JSON serialization")
}

/// Same as [`to_canonical_bytes`] but skips one top-level key — used by the
/// envelope so we sign everything *except* the `signature` field itself.
pub fn to_canonical_bytes_excluding(value: &Value, exclude_top_level_key: &str) -> Vec<u8> {
    let mut v = value.clone();
    if let Value::Object(map) = &mut v {
        map.remove(exclude_top_level_key);
    }
    to_canonical_bytes(&v)
}

fn normalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> =
                map.iter().map(|(k, v)| (k.clone(), normalize(v))).collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            let mut sorted = Map::with_capacity(entries.len());
            for (k, v) in entries {
                sorted.insert(k, v);
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(normalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_sorted_lexicographically() {
        let v = json!({ "b": 1, "a": 2, "c": 3 });
        let bytes = to_canonical_bytes(&v);
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), r#"{"a":2,"b":1,"c":3}"#);
    }

    #[test]
    fn nested_objects_normalized() {
        let v = json!({ "outer": { "z": 1, "a": 2 } });
        let bytes = to_canonical_bytes(&v);
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            r#"{"outer":{"a":2,"z":1}}"#
        );
    }

    #[test]
    fn arrays_preserve_order() {
        // Arrays are ordered — only object keys are sorted.
        let v = json!([3, 1, 2]);
        let bytes = to_canonical_bytes(&v);
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "[3,1,2]");
    }

    #[test]
    fn exclude_top_level_key_removes_only_that_field() {
        let v = json!({ "a": 1, "signature": "abc", "b": 2 });
        let bytes = to_canonical_bytes_excluding(&v, "signature");
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), r#"{"a":1,"b":2}"#);
    }
}
