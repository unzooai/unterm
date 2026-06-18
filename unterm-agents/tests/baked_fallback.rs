//! End-to-end test: the baked-in fallback envelope passes signature
//! verification with the baked trusted keys. This catches the worst-case
//! regression: someone updates `keys/trusted.json` without re-signing the
//! envelope (or vice versa) and ships a binary that fails to verify its
//! own bundled fallback.

use unterm_agents::fetch_manifests_offline;

#[test]
fn baked_envelope_verifies_and_lists_five_agents() {
    // First delete any stale on-disk cache from previous test runs in this
    // home dir — we want to force the baked path.
    if let Ok(cache) = unterm_agents::paths::manifest_cache_path() {
        let _ = std::fs::remove_file(cache);
    }

    let set = fetch_manifests_offline().expect("baked envelope must verify");

    // Catches the "signed with old key" regression where keys/trusted.json
    // and baked/manifests-fallback.json drift apart.
    assert!(
        set.envelope.manifests.len() >= 1,
        "baked envelope should ship with at least one agent manifest"
    );

    // The 5 first-party agents we author by hand. If a release drops one,
    // update both the manifest folder and this test.
    let ids: Vec<&str> = set
        .envelope
        .manifests
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    let must = [
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "opencode",
        "aider",
    ];
    for id in must {
        assert!(
            ids.contains(&id),
            "baked envelope missing first-party manifest {id:?}; have {ids:?}"
        );
    }
}

#[test]
fn baked_envelope_advertises_a_recent_min_version() {
    let set = fetch_manifests_offline().unwrap();
    // The baked fallback's min_unterm_version should at least be older or
    // equal to the running crate version — if it's newer, every install
    // refuses to use the fallback.
    let pkg = env!("CARGO_PKG_VERSION");
    assert!(
        unterm_agents::envelope::compare_versions(pkg, &set.envelope.min_unterm_version) >= 0,
        "baked envelope requires Unterm >= {} but we are {pkg}",
        set.envelope.min_unterm_version
    );
}
