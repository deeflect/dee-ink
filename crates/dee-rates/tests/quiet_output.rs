#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-rates").unwrap()
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

/// get with invalid currency code gives INVALID_ARGUMENT in JSON mode
#[test]
fn get_invalid_currency_json_error() {
    let out = bin().args(["get", "--json", "TOOLONG"]).output().unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("INVALID_ARGUMENT"));
}

/// convert with invalid source currency gives INVALID_ARGUMENT
#[test]
fn convert_invalid_from_currency() {
    let out = bin()
        .args(["convert", "--json", "100", "XX", "EUR"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("INVALID_ARGUMENT"));
}

/// convert with invalid target currency gives INVALID_ARGUMENT
#[test]
fn convert_invalid_to_currency() {
    let out = bin()
        .args(["convert", "--json", "100", "USD", "NOPE"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("INVALID_ARGUMENT"));
}
