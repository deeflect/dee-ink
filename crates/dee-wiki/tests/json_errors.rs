#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-wiki").unwrap()
}

#[test]
fn help_includes_examples() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("EXAMPLES"));
}

/// When get fails with --json, the error must appear on stdout (not stderr)
/// We use an invalid lang code to force a parse/validation error without network.
#[test]
fn get_invalid_lang_json_error_on_stdout() {
    let out = bin()
        .args(["get", "--json", "--lang", "123", "Rust"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Error must be on stdout
    assert!(
        !stdout.trim().is_empty(),
        "error JSON must appear on stdout"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error output must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert!(parsed["code"].is_string());

    // Error must NOT be JSON on stderr
    assert!(
        !stderr.trim().starts_with('{'),
        "JSON error must not appear on stderr"
    );
}

#[test]
fn search_invalid_lang_json_error_on_stdout() {
    let out = bin()
        .args(["search", "--json", "--lang", "!!", "anything"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error output must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
}

#[test]
fn version_flag_succeeds() {
    bin().arg("--version").assert().success();
}
