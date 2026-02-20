#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("dee-qr").unwrap()
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

/// generate without --out for png format gives MISSING_ARGUMENT error in JSON mode
#[test]
fn generate_png_without_out_gives_missing_argument_json() {
    let out = bin()
        .args(["generate", "--json", "--format", "png", "hello"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("MISSING_ARGUMENT"));
}

/// decode with unsupported image format gives UNSUPPORTED_FORMAT in JSON mode
#[test]
fn decode_unsupported_format_json_error() {
    let out = bin()
        .args(["decode", "--json", "image.xyz"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("UNSUPPORTED_FORMAT"));
}

/// decode with non-existent file also fails with an appropriate error
#[test]
fn decode_missing_file_json_error() {
    let out = bin()
        .args(["decode", "--json", "/tmp/definitely_not_exists_12345.png"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("NOT_FOUND"));
    assert!(parsed["error"]
        .as_str()
        .unwrap_or_default()
        .contains("Image file not found"));
}
