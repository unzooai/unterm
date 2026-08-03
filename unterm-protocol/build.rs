use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=UNTERM_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../.git/HEAD");

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
