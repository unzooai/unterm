//! Local path-scope enforcement for terminal-side actions.
//!
//! The product layer owns workspace meaning. Unterm only receives concrete
//! path sets and answers whether a resolved filesystem path is inside them.

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

fn path_contains(root: &Path, candidate: &Path) -> bool {
    path_key(candidate).starts_with(&path_key(root))
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\").to_lowercase();
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
