#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("dee-openrouter").unwrap()
}

/// Run a command with HOME set to a temp dir so config is isolated
fn bin_with_home(dir: &TempDir) -> Command {
    let mut cmd = bin();
    cmd.env("HOME", dir.path());
    // macOS/Linux: dirs crate uses HOME for config dir
    cmd.env("XDG_CONFIG_HOME", dir.path().join("config"));
    cmd
}

#[test]
fn config_path_prints_path() {
    let home = TempDir::new().unwrap();
    bin_with_home(&home)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dee-openrouter"))
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn config_show_json_structure() {
    let home = TempDir::new().unwrap();
    let out = bin_with_home(&home)
        .args(["config", "show", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("config show --json must emit valid JSON");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert!(parsed["item"]["path"].is_string());
    assert!(parsed["item"]["api_key_set"].is_boolean());
}

#[test]
fn config_set_api_key_roundtrip() {
    let home = TempDir::new().unwrap();

    // Set the key
    bin_with_home(&home)
        .args(["config", "set", "openrouter.api-key", "test-key-123"])
        .assert()
        .success();

    // Show should now report api_key_set=true
    let out = bin_with_home(&home)
        .args(["config", "show", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["item"]["api_key_set"], serde_json::json!(true));
}

#[test]
fn config_set_unknown_key_fails_with_json_error() {
    let home = TempDir::new().unwrap();
    let out = bin_with_home(&home)
        .args(["config", "set", "--json", "bad.key", "value"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert_eq!(parsed["code"], serde_json::json!("INVALID_ARGUMENT"));
}
