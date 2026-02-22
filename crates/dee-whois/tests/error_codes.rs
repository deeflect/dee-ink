#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-whois").unwrap()
}

#[test]
fn help_includes_examples() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("EXAMPLES"));
}

/// --raw and --expires together should give INVALID_ARGUMENT error
#[test]
fn raw_and_expires_together_gives_invalid_argument() {
    let out = bin()
        .args(["--json", "--raw", "--expires", "example.com"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("INVALID_ARGUMENT"));
}

#[test]
fn version_flag_succeeds() {
    bin().arg("--version").assert().success();
}

/// lookup against a reserved invalid TLD should classify as NETWORK_ERROR
#[test]
fn invalid_tld_lookup_classifies_network_error() {
    let out = bin()
        .args(["--json", "no-such-domain-deedee-zzzz.invalid"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("NETWORK_ERROR"));
}
