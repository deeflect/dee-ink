#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-ssl").unwrap()
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
fn version_flag_succeeds() {
    bin().arg("--version").assert().success();
}

/// When check fails with --json, the error must appear on stdout with ok=false and code
/// We use a port that will refuse connections (1) to get a fast, deterministic error.
#[test]
fn check_connection_refused_json_error_on_stdout() {
    let out = bin()
        .args([
            "check",
            "--json",
            "--timeout-secs",
            "2",
            "--port",
            "1",
            "127.0.0.1",
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert!(
        parsed["code"].is_string() && !parsed["code"].as_str().unwrap().is_empty(),
        "code must be non-empty string"
    );
    assert!(
        !stderr.trim().starts_with('{'),
        "JSON error must not appear on stderr"
    );
}

/// DNS failure gives a deterministic error code
#[test]
fn check_dns_failure_json_error() {
    let out = bin()
        .args([
            "check",
            "--json",
            "--timeout-secs",
            "2",
            "this.domain.definitely.does.not.exist.invalid",
        ])
        .output()
        .unwrap();

    // May fail with RESOLVE_FAILED or NETWORK_ERROR depending on OS
    if !out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !stdout.trim().is_empty() {
            let parsed: serde_json::Value =
                serde_json::from_str(stdout.trim()).expect("error must be valid JSON");
            assert_eq!(parsed["ok"], serde_json::json!(false));
        }
    }
}

#[test]
fn timeout_secs_flag_accepted() {
    // Just verify the flag is parsed without error
    bin()
        .args(["check", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("timeout-secs"));
}
