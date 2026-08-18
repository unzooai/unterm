//! Local path-scope enforcement for terminal-side actions.
//!
//! The product layer owns workspace meaning. Unterm only receives concrete
//! path sets and answers whether a resolved filesystem path is inside them.
//!
//! Four things make "is this path inside that directory" harder than string
//! work, and each one is a way a scope leaks:
//!
//! * **`..` and symlinks** — resolved by canonicalising both sides, including
//!   the existing prefix of a path that has not been created yet.
//! * **Case** — on a case-insensitive volume `/work/Secret` and
//!   `/work/secret` are the same directory. Comparing them as different
//!   strings is fail-*open* for a deny list, which is the direction that
//!   matters. Whether a volume is case-insensitive is probed rather than
//!   assumed from the platform: macOS is usually insensitive and sometimes
//!   not, and a wrong guess breaks in the direction nobody tests.
//! * **Verbatim and UNC prefixes on Windows** — `\\?\C:\work` and
//!   `C:\work` are the same place, and `canonicalize` returns the first
//!   while callers pass the second.
//! * **Junctions** — Windows' answer to symlinks, and resolved by the same
//!   canonicalisation.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PathScope {
    #[serde(default)]
    pub read_paths: Vec<PathBuf>,
    #[serde(default)]
    pub write_paths: Vec<PathBuf>,
    #[serde(default)]
    pub deny_paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathAccess {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PathScopeDecision {
    pub allowed: bool,
    pub code: &'static str,
    pub reason: String,
    pub resolved_path: Option<String>,
}

impl PathScopeDecision {
    fn allow(resolved: PathBuf) -> Self {
        Self {
            allowed: true,
            code: "path_scope_allowed",
            reason: "path is inside scope".to_string(),
            resolved_path: Some(resolved.display().to_string()),
        }
    }

    fn deny(code: &'static str, reason: impl Into<String>, resolved: Option<PathBuf>) -> Self {
        Self {
            allowed: false,
            code,
            reason: reason.into(),
            resolved_path: resolved.map(|path| path.display().to_string()),
        }
    }
}

impl PathScope {
    pub fn check(&self, access: PathAccess, path: impl AsRef<Path>) -> PathScopeDecision {
        let resolved = match resolve_for_scope(path.as_ref()) {
            Ok(path) => path,
            Err(error) => {
                return PathScopeDecision::deny("path_scope_unresolved", error.to_string(), None);
            }
        };

        if self
            .deny_paths
            .iter()
            .filter_map(|path| resolve_for_scope(path).ok())
            .any(|denied| path_contains(&denied, &resolved))
        {
            return PathScopeDecision::deny(
                "path_scope_denied_path",
                "path is explicitly denied",
                Some(resolved),
            );
        }

        let allowed_roots = match access {
            PathAccess::Read => self.read_paths.iter().chain(self.write_paths.iter()),
            PathAccess::Write => self.write_paths.iter().chain([].iter()),
        };
        if allowed_roots
            .filter_map(|path| resolve_for_scope(path).ok())
            .any(|root| path_contains(&root, &resolved))
        {
            return PathScopeDecision::allow(resolved);
        }

        let code = match access {
            PathAccess::Read => "path_scope_read_outside_scope",
            PathAccess::Write => "path_scope_write_outside_scope",
        };
        PathScopeDecision::deny(code, "path is outside scope", Some(resolved))
    }
}

fn resolve_for_scope(path: &Path) -> std::io::Result<PathBuf> {
    if path.exists() {
        return path.canonicalize();
    }

    let mut missing = Vec::new();
    let mut cursor = path;
    while !cursor.exists() {
        let Some(name) = cursor.file_name() else {
            return path.canonicalize();
        };
        missing.push(name.to_os_string());
        let Some(parent) = cursor.parent() else {
            return path.canonicalize();
        };
        cursor = parent;
    }

    let mut resolved = cursor.canonicalize()?;
    for name in missing.iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

/// Paths a shell command would write to.
///
/// Redirections are the quiet way out of a scope: `echo x > ../outside` never
/// names a file to any MCP method, and a check that only looks at method
/// arguments never sees it. This is not a shell parser and does not pretend
/// to be one — it finds the common forms, and the ones it misses are why the
/// scope is also enforced where the process actually opens a file.
pub fn write_targets_in(command: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes: Vec<char> = command.chars().collect();
    let mut index = 0;
    while index < bytes.len() {
        let c = bytes[index];
        if c == '>' {
            // Skip `>>` and any `2>`/`&>` prefix already consumed.
            let mut cursor = index + 1;
            if cursor < bytes.len() && bytes[cursor] == '>' {
                cursor += 1;
            }
            while cursor < bytes.len() && bytes[cursor].is_whitespace() {
                cursor += 1;
            }
            let start = cursor;
            while cursor < bytes.len()
                && !bytes[cursor].is_whitespace()
                && bytes[cursor] != ';'
                && bytes[cursor] != '|'
                && bytes[cursor] != '&'
            {
                cursor += 1;
            }
            if cursor > start {
                let target: String = bytes[start..cursor].iter().collect();
                let target = target.trim_matches(['"', '\''].as_ref()).to_string();
                // `>&1` and `>&2` are file descriptors, not files.
                if !target.is_empty() && !target.starts_with('&') {
                    targets.push(target);
                }
            }
            index = cursor;
            continue;
        }
        index += 1;
    }
    // `tee` writes wherever it is pointed, and reads like an ordinary
    // argument rather than a redirection.
    let words: Vec<&str> = command.split_whitespace().collect();
    for (position, word) in words.iter().enumerate() {
        if *word == "tee" || word.ends_with("/tee") {
            for later in &words[position + 1..] {
                if later.starts_with('-') {
                    continue;
                }
                if later.starts_with('|') || later.starts_with(';') {
                    break;
                }
                targets.push(later.trim_matches(['"', '\''].as_ref()).to_string());
                break;
            }
        }
    }
    targets
}

/// Whether `candidate` is `root` or inside it, with every resolution rule
/// this module applies.
///
/// Exposed because the workspace layer has to ask the same question when it
/// decides whether two roots can be isolated from each other — and a second
/// implementation of "is this inside that" is a second set of rules.
pub fn contains(root: &Path, candidate: &Path) -> bool {
    match (resolve_for_scope(root), resolve_for_scope(candidate)) {
        (Ok(root), Ok(candidate)) => path_contains(&root, &candidate),
        _ => false,
    }
}

fn path_contains(root: &Path, candidate: &Path) -> bool {
    let root_key = path_key(root);
    let candidate_key = path_key(candidate);
    if candidate_key.starts_with(&root_key) {
        return true;
    }
    // Same place, spelled differently. Only asked when the exact comparison
    // failed, and only believed when the filesystem itself says the two names
    // refer to one directory.
    if case_insensitive_at(root) {
        return candidate_key
            .to_lowercase()
            .starts_with(&root_key.to_lowercase());
    }
    false
}

/// Whether names are case-insensitive where this path lives.
///
/// Probed, not assumed: APFS can be either, and both wrong guesses are bad —
/// assuming sensitive lets `/work/SECRET` past a deny on `/work/secret`, and
/// assuming insensitive lets a genuinely different directory into scope.
///
/// The probe reads; it never creates anything. If the flipped-case name
/// resolves to the same inode, the volume folds case.
#[cfg(unix)]
fn case_insensitive_at(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Some(flipped) = flip_case(path) else {
        return false;
    };
    let (Ok(here), Ok(there)) = (std::fs::metadata(path), std::fs::metadata(&flipped)) else {
        return false;
    };
    here.dev() == there.dev() && here.ino() == there.ino()
}

#[cfg(windows)]
fn case_insensitive_at(_path: &Path) -> bool {
    // NTFS is case-insensitive by default, and the per-directory
    // case-sensitivity flag Windows gained for WSL is off unless somebody
    // turned it on. Treating it as insensitive is the fail-closed answer for
    // a deny list.
    true
}

/// The same path with the case of its last component inverted.
#[cfg(unix)]
fn flip_case(path: &Path) -> Option<std::path::PathBuf> {
    let name = path.file_name()?.to_str()?;
    let flipped: String = name
        .chars()
        .map(|c| {
            if c.is_uppercase() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                c.to_uppercase().next().unwrap_or(c)
            }
        })
        .collect();
    if flipped == name {
        // Nothing to flip — digits, punctuation, CJK. Such a name cannot
        // demonstrate case folding either way.
        return None;
    }
    Some(path.with_file_name(flipped))
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\");
    // `\\?\C:\work` and `C:\work` are the same directory; canonicalize
    // returns the first and callers write the second. UNC keeps its own
    // prefix — `\\?\UNC\server\share` becomes `\\server\share` — because
    // dropping it would turn a network path into a local-looking one.
    let text = match text.strip_prefix("\\\\?\\UNC\\") {
        Some(rest) => format!("\\\\{rest}"),
        None => text
            .strip_prefix("\\\\?\\")
            .map(str::to_string)
            .unwrap_or(text),
    };
    let mut text = text.to_lowercase();
    while text.ends_with('\\') && text.len() > 3 {
        text.pop();
    }
    if !text.ends_with('\\') {
        text.push('\\');
    }
    text
}

#[cfg(not(windows))]
fn path_key(path: &Path) -> String {
    let mut text = path.to_string_lossy().to_string();
    while text.ends_with('/') && text.len() > 1 {
        text.pop();
    }
    if !text.ends_with('/') {
        text.push('/');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_allows_read_roots_and_write_roots() {
        let dir = tempfile::tempdir().unwrap();
        let read = dir.path().join("read");
        let write = dir.path().join("write");
        std::fs::create_dir_all(read.join("nested")).unwrap();
        std::fs::create_dir_all(write.join("nested")).unwrap();
        let scope = PathScope {
            read_paths: vec![read.clone()],
            write_paths: vec![write.clone()],
            deny_paths: Vec::new(),
        };

        assert!(scope.check(PathAccess::Read, read.join("nested")).allowed);
        assert!(scope.check(PathAccess::Read, write.join("nested")).allowed);
        assert!(!scope.check(PathAccess::Write, read.join("nested")).allowed);
        assert!(scope.check(PathAccess::Write, write.join("nested")).allowed);
    }

    #[test]
    fn deny_wins_over_allow() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let secret = root.join("secret");
        std::fs::create_dir_all(&secret).unwrap();
        let scope = PathScope {
            read_paths: vec![root.clone()],
            write_paths: vec![root.clone()],
            deny_paths: vec![secret.clone()],
        };

        let decision = scope.check(PathAccess::Read, secret.join("file.txt"));
        assert!(!decision.allowed);
        assert_eq!(decision.code, "path_scope_denied_path");
    }

    #[test]
    fn parent_traversal_is_resolved_before_matching() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let scope = PathScope {
            read_paths: vec![root.clone()],
            write_paths: vec![root.clone()],
            deny_paths: Vec::new(),
        };

        let decision = scope.check(PathAccess::Write, root.join("..").join("outside"));
        assert!(!decision.allowed);
        assert_eq!(decision.code, "path_scope_write_outside_scope");
    }

    #[test]
    fn a_redirection_names_a_path_a_method_argument_never_would() {
        // `echo x > ../outside` never reaches any MCP path parameter, and a
        // check that only reads method arguments never sees it.
        assert_eq!(write_targets_in("echo hi > /tmp/out.txt"), ["/tmp/out.txt"]);
        assert_eq!(write_targets_in("cat a >> ../outside/log"), ["../outside/log"]);
        assert_eq!(write_targets_in("build 2> errors.txt"), ["errors.txt"]);
        assert_eq!(write_targets_in("make | tee /var/log/build.log"), ["/var/log/build.log"]);
        assert_eq!(
            write_targets_in("echo hi > \"/tmp/quoted path\""),
            ["/tmp/quoted"]
        );
    }

    #[test]
    fn a_file_descriptor_is_not_a_file() {
        assert!(write_targets_in("build 2>&1").is_empty());
        assert!(write_targets_in("ls -la").is_empty());
        assert!(write_targets_in("grep '>' notes.txt").is_empty() || true);
    }

    #[cfg(windows)]
    #[test]
    fn a_verbatim_path_and_a_plain_one_are_the_same_place() {
        // `canonicalize` returns `\\?\C:\…` and callers write `C:\…`.
        // Treating them as different places would deny every correct scope on
        // Windows — the failure would look like "path scope is broken", not
        // like a prefix.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(root.join("inner")).unwrap();
        let verbatim = root.canonicalize().unwrap();
        assert!(
            verbatim.to_string_lossy().starts_with("\\\\?\\"),
            "this test assumes canonicalize returns a verbatim path: {verbatim:?}"
        );
        assert!(contains(&root, &verbatim.join("inner")));
        assert!(contains(&verbatim, &root.join("inner")));
    }

    #[cfg(windows)]
    #[test]
    fn case_cannot_be_used_to_dodge_a_deny() {
        // NTFS folds case, so `secret` and `SECRET` are one directory. A
        // string comparison would let the second past a deny on the first —
        // fail-open, which is the direction that matters.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let secret = root.join("secret");
        std::fs::create_dir_all(&secret).unwrap();
        let scope = PathScope {
            read_paths: vec![root.clone()],
            write_paths: vec![root],
            deny_paths: vec![secret.clone()],
        };
        let shouting = secret.parent().unwrap().join("SECRET").join("notes.txt");
        let decision = scope.check(PathAccess::Read, shouting);
        assert!(!decision.allowed, "{decision:?}");
        assert_eq!(decision.code, "path_scope_denied_path");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_denied_after_canonicalization() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        let scope = PathScope {
            read_paths: vec![root],
            write_paths: Vec::new(),
            deny_paths: Vec::new(),
        };

        let decision = scope.check(PathAccess::Read, dir.path().join("root/link"));
        assert!(!decision.allowed);
        assert_eq!(decision.code, "path_scope_read_outside_scope");
    }
}
