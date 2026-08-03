use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn version_is_fast_and_does_not_register_an_instance() {
    let home = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_unterm"))
        .arg("--version")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .output()
        .expect("run unterm --version");

    assert!(output.status.success());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("unterm {}", unterm_protocol::PRODUCT_VERSION)
    );
    assert!(
        !home.path().join(".unterm").exists(),
        "version probe must not create product state"
    );
}
