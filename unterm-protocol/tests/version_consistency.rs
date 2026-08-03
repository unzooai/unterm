use std::path::PathBuf;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn manifests_and_installer_share_the_product_version() {
    let root = repository_root();
    let workspace: toml::Value = toml::from_str(
        &std::fs::read_to_string(root.join("Cargo.toml")).expect("read workspace Cargo.toml"),
    )
    .expect("parse workspace Cargo.toml");
    let expected = workspace["workspace"]["package"]["version"]
        .as_str()
        .expect("workspace.package.version");
    assert_eq!(expected, unterm_protocol::PRODUCT_VERSION);

    for manifest in ["unterm-app/Cargo.toml", "unterm-cli/Cargo.toml"] {
        let package: toml::Value = toml::from_str(
            &std::fs::read_to_string(root.join(manifest)).expect("read product manifest"),
        )
        .expect("parse product manifest");
        assert_eq!(
            package["package"]["version"]["workspace"].as_bool(),
            Some(true)
        );
    }

    let installer = std::fs::read_to_string(root.join("installer/Unterm.wxs"))
        .expect("read Windows installer manifest");
    assert!(
        installer.contains(&format!("Version=\"{expected}\"")),
        "installer version does not match {expected}"
    );
}

#[test]
fn release_workflows_build_both_product_binaries() {
    let root = repository_root();
    for workflow in [
        ".github/workflows/release-windows.yml",
        ".github/workflows/release-linux.yml",
    ] {
        let text = std::fs::read_to_string(root.join(workflow)).expect("read release workflow");
        assert!(text.contains("-p unterm-app -p unterm-cli"), "{workflow}");
    }
}
