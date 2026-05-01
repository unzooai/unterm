//! Render-time redaction. Redaction is *never* applied to the .log file
//! (which holds the immutable source-of-truth bytes); it is a transform
//! applied only when we render markdown.

use parking_lot::Mutex;
use regex::Regex;
use std::sync::OnceLock;

const KV_PATTERN: &str =
    r"(?i)\b(token|key|secret|password|api[_-]?key|credential)s?\s*[:=]\s*\S+";
const GITHUB_PATTERN: &str = r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}\b";
const GENERIC_TOKEN_PATTERN: &str = r"\b[A-Za-z0-9+/=_\-]{40,}\b";

/// A compiled set of redaction patterns. Built-ins are always present;
/// user-supplied custom regex strings come from
/// `~/.unterm/recording.json:redaction.custom_patterns`.
struct Compiled {
    kv: Regex,
    github: Regex,
    generic: Regex,
    custom: Vec<Regex>,
}

fn compiled() -> &'static Mutex<Option<Compiled>> {
    static C: OnceLock<Mutex<Option<Compiled>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

fn build(custom_patterns: &[String]) -> Compiled {
    Compiled {
        kv: Regex::new(KV_PATTERN).expect("compile KV pattern"),
        github: Regex::new(GITHUB_PATTERN).expect("compile GitHub pattern"),
        generic: Regex::new(GENERIC_TOKEN_PATTERN).expect("compile generic pattern"),
        custom: custom_patterns
            .iter()
            .filter_map(|p| match Regex::new(p) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::warn!("invalid custom redaction pattern {p:?}: {e}");
                    None
                }
            })
            .collect(),
    }
}

/// Return (redacted_text, count_of_redactions).
pub fn redact(text: &str, custom_patterns: &[String]) -> (String, u64) {
    let mut guard = compiled().lock();
    if guard.is_none() {
        *guard = Some(build(custom_patterns));
    }
    let c = guard.as_ref().unwrap();

    let mut count: u64 = 0;
    let mut out = text.to_string();

    // KV form: `token: <value>` -> `token: <redacted>`
    out = c
        .kv
        .replace_all(&out, |caps: &regex::Captures| {
            count += 1;
            let full = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            // Split on first `=` or `:` to keep the key intact
            if let Some(idx) = full.find(|c: char| c == '=' || c == ':') {
                let (k, _) = full.split_at(idx + 1);
                format!("{} <redacted>", k.trim_end())
            } else {
                "<redacted>".to_string()
            }
        })
        .into_owned();

    // GitHub tokens
    out = c
        .github
        .replace_all(&out, |caps: &regex::Captures| {
            count += 1;
            let m = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let prefix = m.get(0..8).unwrap_or(m);
            format!("<token:{}...redacted>", prefix)
        })
        .into_owned();

    // Generic high-entropy strings (40+ chars). Apply last so it doesn't
    // shadow the key/value form.
    out = c
        .generic
        .replace_all(&out, |caps: &regex::Captures| {
            count += 1;
            let m = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let hash = sha256_first_8(m);
            format!("<token:{}...redacted>", hash)
        })
        .into_owned();

    for r in &c.custom {
        out = r
            .replace_all(&out, |_caps: &regex::Captures| {
                count += 1;
                "<redacted>"
            })
            .into_owned();
    }

    (out, count)
}

fn sha256_first_8(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // We don't depend on a real sha2 crate (avoid adding deps); a stable
    // shorthand from DefaultHasher is good enough for "first-8 chars
    // identifier" purposes — it's only used to disambiguate redacted
    // tokens in human-readable output.
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    let h = hasher.finish();
    format!("{:08x}", (h & 0xffff_ffff) as u32)
}

/// Drop the cached compiled patterns so the next call to `redact()`
/// rebuilds them with the supplied custom patterns. Called when the
/// recording.json config changes.
#[allow(dead_code)]
pub fn invalidate_cache() {
    *compiled().lock() = None;
}
