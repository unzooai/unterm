use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn version_is_fast_and_does_not_register_an_instance() {
    // Untimed warm-up. Cargo re-copies the uplifted binary on every test
    // invocation, and a fresh file identity sends Gatekeeper scanning all of
    // it on first exec -- seconds, once, and nothing to do with our startup.
    // The timed spawn below measures the product, not the platform's scanner.
    let warm_home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_unterm"))
        .arg("--version")
        .env("USERPROFILE", warm_home.path())
        .env("HOME", warm_home.path())
        .output()
        .expect("warm up unterm --version");

    let home = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_unterm"))
        .arg("--version")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .output()
        .expect("run unterm --version");

    assert!(output.status.success());
    // Three seconds, not one. What this test is for is the line below it:
    // a version probe must not start the product. The clock is only here to
    // catch "it initialised everything and then printed a version", and for
    // that a wide bound works as well as a tight one.
    //
    // A tight one does not survive the suite it lives in. Cargo re-copies the
    // uplifted binary on every invocation, so its file identity is new and
    // Windows rescans it on first exec; the warm-up above exists for that, and
    // under a full `cargo test --workspace` -- dozens of test binaries in
    // parallel, some spawning real Cores and PTYs -- the warm-up itself can
    // lose the race. Measured on this machine at the same commit: 0.31 s idle,
    // 1.48 s while the suite ran. It failed once in a full run and passed on
    // the next, which is the worst kind of red.
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("unterm {}", unterm_protocol::PRODUCT_VERSION)
    );
    assert!(
        !home.path().join(".unterm").exists(),
        "version probe must not create product state"
    );
}

#[test]
fn help_is_fast_and_does_not_register_an_instance() {
    let warm_home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_unterm"))
        .arg("--help")
        .env("USERPROFILE", warm_home.path())
        .env("HOME", warm_home.path())
        .output()
        .expect("warm up unterm --help");

    let home = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_unterm"))
        .arg("--help")
        .env("USERPROFILE", home.path())
        .env("HOME", home.path())
        .output()
        .expect("run unterm --help");

    assert!(output.status.success());
    // Wide for the same reason as the probe above.
    assert!(started.elapsed() < Duration::from_secs(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("--cwd <dir>"));
    assert!(stdout.contains("--version"));
    assert!(
        !home.path().join(".unterm").exists(),
        "help probe must not create product state"
    );
}
