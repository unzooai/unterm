use std::process::Command;

#[test]
fn removed_legacy_commands_have_machine_readable_json_errors() {
    for command in ["cli", "record", "replay", "connect"] {
        let output = Command::new(env!("CARGO_BIN_EXE_unterm-cli"))
            .args(["--json", command])
            .output()
            .unwrap_or_else(|error| panic!("run unterm-cli --json {command}: {error}"));

        assert!(
            !output.status.success(),
            "{command} should remain a non-zero compatibility error"
        );
        let value: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "{command} stdout should be JSON, got {:?}: {error}",
                    String::from_utf8_lossy(&output.stdout)
                )
            });

        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "legacy_removed");
        assert_eq!(value["error"]["command"], command);
        assert!(value["error"]["message"].as_str().is_some_and(|text| {
            text.contains("old") || text.contains("Legacy")
        }));
        assert!(value["error"]["replacement"]
            .as_str()
            .is_some_and(|text| text.contains("unterm-cli") || text.contains("native replay")));
    }
}
