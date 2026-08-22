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
    // Fixed 2026-08-22; the history is here so the fix is not undone.
    //
    // This failed in roughly half of `cargo test --workspace` runs and the
    // clock was never why. `CARGO_BIN_EXE_unterm` names
    // `target/debug/unterm.exe`, and cargo tagged two different artifacts
    // `kind=bin, name=unterm` -- the terminal, and the libtest harness built
    // from the same `main.rs` -- then uplifted both to that one path. The
    // loser of that race was what this test ran. Watched during one run, the
    // file sat at 37 MB and answered `--version` with getopts'
    // `error: Unrecognized option: 'version'`.
    //
    // `unterm-app` is now a library with a four-line bin around it, and that
    // bin has `test = false`. Unit tests are `kind=lib, name=unterm_app`,
    // land in `deps/`, and are never uplifted, so exactly one artifact can
    // claim `target/debug/unterm.exe`.
    //
    // Two wrong diagnoses were tried first and are recorded so they are not
    // tried again: "it is slow under load" (measured: 0.31 s idle, 1.48 s
    // loaded, both inside any threshold) and "the uplift is momentarily
    // racing" (measured: five spawns 200 ms apart, all five reached the
    // harness -- the race is between two linkers, not two reads).
    //
    // Three seconds, not one, and that part is not about the above. What
    // this test is for is the line below it: a version probe must not start
    // the product. The clock only catches "it initialised everything and
    // then printed a version", and a wide bound does that as well as a tight
    // one. A tight one does not survive the suite it lives in -- cargo
    // re-copies the uplifted binary on every invocation, so its file
    // identity is new and Windows rescans it on first exec. The warm-up
    // above exists for that, and under a full workspace run the warm-up
    // itself can lose the race: 0.31 s idle, 1.48 s while the suite ran.
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
