//! Read / write each AI agent's native config file via a uniform DOM.
//!
//! Every supported format (json, toml, yaml, env, ini, raw_text) parses
//! into a [`serde_json::Value`] before Unterm touches it, and serializes
//! back to the right format on write. Storing in JSON-shape internally
//! means the jq-style `key_map` paths in a manifest's
//! [`StorageFile.key_map`](crate::manifest::StorageFile) work the same
//! way regardless of which file format the agent uses on disk.
//!
//! Write semantics:
//!   * `preserve_unknown_keys` (default): we set only the keys we own and
//!     leave every other field of the file alone. Important because agents
//!     ship new config fields between versions and we don't want to clobber
//!     them.
//!   * `overwrite`: we serialize the full set of known keys + drop unknown
//!     fields. Rare; used when the agent's config file is *only* meant to
//!     be managed by Unterm.
//!   * `atomic_write`: tmp-file in same dir, then rename(2). Crash-safe.

use crate::errors::{AgentError, Result};
use crate::manifest::StorageFile;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Apply a set of (setting_key → value) updates to one file.
///
/// `path` is the resolved on-disk path (templates like `{{HOME}}` already
/// expanded by the caller).
pub fn apply_settings_to_file(
    file_spec: &StorageFile,
    path: &Path,
    updates: &BTreeMap<String, Value>,
) -> Result<()> {
    // ---- raw_text: the whole file IS the value of one setting ----
    if file_spec.format == "raw_text" {
        let single = file_spec
            .single_key
            .as_ref()
            .ok_or_else(|| AgentError::UnsupportedFormat("raw_text needs single_key".into()))?;
        if let Some(new_value) = updates.get(single) {
            let s = new_value.as_str().unwrap_or("");
            write_bytes(path, s.as_bytes(), file_spec.atomic_write)?;
        }
        return Ok(());
    }

    // ---- structured formats: read → patch → write ----
    let mut dom = if path.exists() {
        let bytes = std::fs::read(path)?;
        parse_to_value(&file_spec.format, &bytes)?
    } else {
        Value::Object(Map::new())
    };

    for (setting_key, new_value) in updates {
        let Some(jq_path) = file_spec.key_map.get(setting_key) else {
            continue; // schema-driven; setting may live in a different file
        };
        set_by_jq_path(&mut dom, jq_path, new_value.clone())?;
    }

    let bytes = serialize_from_value(&file_spec.format, &dom)?;
    write_bytes(path, &bytes, file_spec.atomic_write)?;
    Ok(())
}

/// Read settings back out of an agent's existing config file. Used by the
/// "import existing settings" flow on first run.
pub fn read_settings_from_file(
    file_spec: &StorageFile,
    path: &Path,
) -> Result<BTreeMap<String, Value>> {
    let mut out = BTreeMap::new();

    if file_spec.format == "raw_text" {
        if let Some(single) = &file_spec.single_key {
            if path.exists() {
                let s = std::fs::read_to_string(path)?;
                out.insert(single.clone(), Value::String(s));
            }
        }
        return Ok(out);
    }

    if !path.exists() {
        return Ok(out);
    }
    let bytes = std::fs::read(path)?;
    let dom = parse_to_value(&file_spec.format, &bytes)?;

    for (setting_key, jq_path) in &file_spec.key_map {
        if let Some(v) = get_by_jq_path(&dom, jq_path) {
            out.insert(setting_key.clone(), v.clone());
        }
    }
    Ok(out)
}

// ---------- DOM parse / serialize per format ----------

fn parse_to_value(format: &str, bytes: &[u8]) -> Result<Value> {
    match format {
        "json" => serde_json::from_slice(bytes).map_err(AgentError::from),
        "toml" => {
            let s =
                std::str::from_utf8(bytes).map_err(|e| AgentError::ParseFailed(e.to_string()))?;
            let t: toml::Value =
                toml::from_str(s).map_err(|e| AgentError::ParseFailed(e.to_string()))?;
            Ok(toml_value_to_json(t))
        }
        "yaml" => {
            let v: serde_yaml::Value = serde_yaml::from_slice(bytes)
                .map_err(|e| AgentError::ParseFailed(e.to_string()))?;
            Ok(yaml_value_to_json(v))
        }
        "env" => Ok(env_text_to_value(
            std::str::from_utf8(bytes).map_err(|e| AgentError::ParseFailed(e.to_string()))?,
        )),
        "ini" => Ok(ini_text_to_value(
            std::str::from_utf8(bytes).map_err(|e| AgentError::ParseFailed(e.to_string()))?,
        )),
        other => Err(AgentError::UnsupportedFormat(other.into())),
    }
}

fn serialize_from_value(format: &str, value: &Value) -> Result<Vec<u8>> {
    match format {
        "json" => {
            let mut bytes = serde_json::to_vec_pretty(value)?;
            bytes.push(b'\n');
            Ok(bytes)
        }
        "toml" => {
            let t = json_to_toml_value(value)?;
            let s = toml::to_string_pretty(&t)
                .map_err(|e| AgentError::ParseFailed(format!("toml serialize: {e}")))?;
            Ok(s.into_bytes())
        }
        "yaml" => {
            let s = serde_yaml::to_string(value)
                .map_err(|e| AgentError::ParseFailed(format!("yaml serialize: {e}")))?;
            Ok(s.into_bytes())
        }
        "env" => Ok(value_to_env_text(value).into_bytes()),
        "ini" => Ok(value_to_ini_text(value).into_bytes()),
        other => Err(AgentError::UnsupportedFormat(other.into())),
    }
}

// ---------- jq-style path ops ----------

/// jq-style path:  `.a.b.c`  /  `a.b.c`  /  `.a.b[0]`
/// Array indexing is intentionally not supported in `set` for now (manifest
/// authors haven't needed it; can add later). We treat anything between
/// `.` separators as a string key.
fn split_path(jq: &str) -> Vec<String> {
    jq.trim_start_matches('.')
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn get_by_jq_path<'a>(value: &'a Value, jq: &str) -> Option<&'a Value> {
    let parts = split_path(jq);
    let mut current = value;
    for p in parts {
        current = current.as_object()?.get(&p)?;
    }
    Some(current)
}

fn set_by_jq_path(value: &mut Value, jq: &str, new: Value) -> Result<()> {
    let parts = split_path(jq);
    if parts.is_empty() {
        return Err(AgentError::ParseFailed(format!(
            "empty jq path {jq:?} when applying setting"
        )));
    }
    let mut current = value;
    for (i, p) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let map = current.as_object_mut().expect("just ensured");
        if is_last {
            map.insert(p.clone(), new.clone());
            return Ok(());
        }
        let entry = map
            .entry(p.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        current = entry;
    }
    Ok(())
}

// ---------- format conversions ----------

fn toml_value_to_json(t: toml::Value) -> Value {
    match t {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_value_to_json).collect()),
        toml::Value::Table(tbl) => {
            let mut m = Map::new();
            for (k, v) in tbl {
                m.insert(k, toml_value_to_json(v));
            }
            Value::Object(m)
        }
    }
}

fn json_to_toml_value(v: &Value) -> Result<toml::Value> {
    Ok(match v {
        Value::Null => toml::Value::String(String::new()), // TOML has no null
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(json_to_toml_value(item)?);
            }
            toml::Value::Array(out)
        }
        Value::Object(map) => {
            let mut tbl = toml::map::Map::new();
            for (k, val) in map {
                tbl.insert(k.clone(), json_to_toml_value(val)?);
            }
            toml::Value::Table(tbl)
        }
    })
}

fn yaml_value_to_json(v: serde_yaml::Value) -> Value {
    use serde_yaml::Value as Y;
    match v {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Y::String(s) => Value::String(s),
        Y::Sequence(arr) => Value::Array(arr.into_iter().map(yaml_value_to_json).collect()),
        Y::Mapping(map) => {
            let mut m = Map::new();
            for (k, v) in map {
                let key = match k {
                    Y::String(s) => s,
                    other => serde_yaml::to_string(&other)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                };
                m.insert(key, yaml_value_to_json(v));
            }
            Value::Object(m)
        }
        Y::Tagged(_) => Value::Null,
    }
}

// .env-style: lines of KEY=value, # comments preserved as-is on write.
// We never *modify* comments — just keep them so the output isn't a stripped
// version of the input.
fn env_text_to_value(text: &str) -> Value {
    let mut map = Map::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let v = v.trim().trim_matches(|c| c == '\'' || c == '"');
            map.insert(k.trim().to_string(), Value::String(v.to_string()));
        }
    }
    Value::Object(map)
}

fn value_to_env_text(v: &Value) -> String {
    let mut s = String::new();
    if let Value::Object(map) = v {
        for (k, val) in map {
            let value_str = match val {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            s.push_str(&format!("{k}={value_str}\n"));
        }
    }
    s
}

// Minimalist INI: [section]\nkey=value lines. Top-level keys go before any
// section. We don't preserve comments — agents that use INI are rare, and
// the ones that do (mostly proxy / Aider's older style) are happy with
// stripped output.
fn ini_text_to_value(text: &str) -> Value {
    let mut root = Map::new();
    let mut current_section: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current_section = Some(name.to_string());
            root.entry(name.to_string())
                .or_insert(Value::Object(Map::new()));
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            let val = Value::String(v.trim().to_string());
            match &current_section {
                Some(section) => {
                    if let Some(Value::Object(sec)) = root.get_mut(section) {
                        sec.insert(key, val);
                    }
                }
                None => {
                    root.insert(key, val);
                }
            }
        }
    }
    Value::Object(root)
}

fn value_to_ini_text(v: &Value) -> String {
    let mut s = String::new();
    if let Value::Object(map) = v {
        // Top-level scalars first.
        for (k, val) in map {
            if !val.is_object() {
                s.push_str(&format!("{k}={}\n", scalar_to_ini(val)));
            }
        }
        for (k, val) in map {
            if let Value::Object(section) = val {
                s.push_str(&format!("\n[{k}]\n"));
                for (sk, sv) in section {
                    s.push_str(&format!("{sk}={}\n", scalar_to_ini(sv)));
                }
            }
        }
    }
    s
}

fn scalar_to_ini(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

// ---------- atomic write ----------

fn write_bytes(path: &Path, bytes: &[u8], atomic: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !atomic {
        std::fs::write(path, bytes)?;
        return Ok(());
    }
    let parent = path.parent().expect("path has parent");
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn spec(format: &str, key_map: &[(&str, &str)]) -> StorageFile {
        let mut km = BTreeMap::new();
        for (k, v) in key_map {
            km.insert((*k).to_string(), (*v).to_string());
        }
        StorageFile {
            path: "ignored".into(),
            format: format.into(),
            merge: "preserve_unknown_keys".into(),
            atomic_write: true,
            key_map: km,
            single_key: None,
        }
    }

    #[test]
    fn json_apply_preserves_unknown_keys() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            br#"{"model":"old","keep_me":42,"nested":{"keep":1}}"#,
        )
        .unwrap();
        let mut updates: BTreeMap<String, Value> = BTreeMap::new();
        updates.insert("model".into(), json!("new"));
        updates.insert("max_tokens".into(), json!(8192));
        let spec = spec(
            "json",
            &[("model", ".model"), ("max_tokens", ".max_tokens")],
        );
        apply_settings_to_file(&spec, tmp.path(), &updates).unwrap();
        let after = std::fs::read_to_string(tmp.path()).unwrap();
        let v: Value = serde_json::from_str(&after).unwrap();
        assert_eq!(v["model"], "new");
        assert_eq!(v["max_tokens"], 8192);
        assert_eq!(v["keep_me"], 42, "unknown top-level keys preserved");
        assert_eq!(v["nested"]["keep"], 1, "unknown nested keys preserved");
    }

    #[test]
    fn toml_round_trips() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"[model]\nname = \"old\"\n").unwrap();
        let mut updates: BTreeMap<String, Value> = BTreeMap::new();
        updates.insert("model".into(), json!("new"));
        let spec = spec("toml", &[("model", ".model.name")]);
        apply_settings_to_file(&spec, tmp.path(), &updates).unwrap();
        let after = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(after.contains("name = \"new\""), "{after}");
    }

    #[test]
    fn raw_text_single_key() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut updates: BTreeMap<String, Value> = BTreeMap::new();
        updates.insert("system_prompt".into(), json!("be helpful"));
        let mut spec = spec("raw_text", &[]);
        spec.single_key = Some("system_prompt".into());
        apply_settings_to_file(&spec, tmp.path(), &updates).unwrap();
        assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), "be helpful");
    }

    #[test]
    fn import_reads_known_keys() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), br#"{"model":"opus","extra":"hidden"}"#).unwrap();
        let spec = spec("json", &[("model", ".model")]);
        let imported = read_settings_from_file(&spec, tmp.path()).unwrap();
        assert_eq!(imported.get("model").unwrap(), &json!("opus"));
        assert!(
            !imported.contains_key("extra"),
            "extra keys are not exposed unless they're in the schema"
        );
    }
}
