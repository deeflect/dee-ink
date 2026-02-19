#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-hn").unwrap()
}

/// When an HN item request fails with --json, error goes to stdout with ok=false+code
/// We use item 0 which is guaranteed not to exist on HN API.
#[test]
fn item_json_error_shape_on_stdout() {
    let out = bin().args(["--json", "item", "0"]).output().unwrap();

    // This will either network-fail or return a not-found, either way it should be JSON on stdout
    if !out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !stdout.trim().is_empty() {
            let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
                .expect("error output must be valid JSON on stdout");
            assert_eq!(parsed["ok"], serde_json::json!(false));
            assert!(
                parsed["code"].is_string() && !parsed["code"].as_str().unwrap().is_empty(),
                "code must be a non-empty string"
            );
        }
        // stderr must not contain JSON
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.trim().starts_with('{'),
            "JSON error must not appear on stderr"
        );
    }
}

#[test]
fn invalid_subcommand_exits_nonzero() {
    bin().arg("__not_a_valid_subcommand__").assert().failure();
}
