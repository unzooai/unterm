use std::path::{Path, PathBuf};
use std::process::Command;

fn git_dir() -> Option<PathBuf> {
    let dot_git = Path::new("../.git");
    if dot_git.is_dir() {
        return Some(dot_git.to_path_buf());
    }

    std::fs::read_to_string(dot_git)
        .ok()
        .and_then(|value| value.trim().strip_prefix("gitdir: ").map(str::to_string))
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                Path::new("..").join(path)
            }
        })
}

fn common_git_dir(git_dir: &Path) -> PathBuf {
    std::fs::read_to_string(git_dir.join("commondir"))
        .ok()
        .map(|value| PathBuf::from(value.trim()))
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                git_dir.join(path)
            }
        })
        .unwrap_or_else(|| git_dir.to_path_buf())
}

fn main() {
    println!("cargo:rerun-if-env-changed=UNTERM_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    if let Some(git_dir) = git_dir() {
        let common_git_dir = common_git_dir(&git_dir);
        let head_path = git_dir.join("HEAD");
        println!("cargo:rerun-if-changed={}", head_path.display());
        if let Ok(head) = std::fs::read_to_string(&head_path) {
            if let Some(reference) = head.trim().strip_prefix("ref: ") {
                println!(
                    "cargo:rerun-if-changed={}",
                    common_git_dir.join(reference).display()
                );
            }
        }
        let packed_refs = common_git_dir.join("packed-refs");
        if packed_refs.exists() {
            println!("cargo:rerun-if-changed={}", packed_refs.display());
        }
    }

    let commit = std::env::var("UNTERM_BUILD_COMMIT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("GITHUB_SHA").ok())
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=12", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| String::from_utf8(output.stdout).ok())
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=UNTERM_BUILD_COMMIT={commit}");
}
