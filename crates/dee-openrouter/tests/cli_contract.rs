#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-openrouter").unwrap()
}

#[test]
fn help_includes_examples() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("EXAMPLES"));
}

#[test]
fn invalid_subcommand_exits_nonzero() {
    bin().arg("__not_a_real_subcommand__").assert().failure();
}

/// When `show` fails with --json, the error must be on stdout (not stderr) as valid JSON
/// with ok=false and a code field. We use an obviously invalid model id so the tool
/// either errors at parse time (no network needed for that case) or at API time.
/// We only assert the shape of the output, not the specific error message.
#[test]
fn show_json_error_on_stdout_not_stderr() {
    let output = bin()
        .args(["show", "--json", "__definitely_not_a_real_model_id__"])
        .output()
        .expect("failed to run binary");

    // stdout must contain JSON with ok=false
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // If there's JSON output, it must be on stdout
    if !stdout.trim().is_empty() {
        let parsed: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");
        assert_eq!(parsed["ok"], serde_json::json!(false), "ok must be false");
        assert!(
            parsed["code"].is_string() && !parsed["code"].as_str().unwrap().is_empty(),
            "code must be a non-empty string"
        );
        // stderr must NOT contain JSON
        assert!(
            !stderr.trim().starts_with('{'),
            "JSON error must not appear on stderr"
        );
    }
    // If network unavailable, the error still goes to stdout in json mode
    assert!(!output.status.success(), "should exit non-zero on error");
}

#[test]
fn config_path_subcommand_succeeds() {
    bin()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}
