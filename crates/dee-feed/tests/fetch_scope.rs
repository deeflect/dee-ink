#![allow(deprecated)]
use assert_cmd::Command;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("dee-feed").unwrap()
}

fn with_home(dir: &TempDir) -> Command {
    let mut cmd = bin();
    cmd.env("HOME", dir.path());
    cmd.env("XDG_CONFIG_HOME", dir.path().join("config"));
    cmd.env("XDG_DATA_HOME", dir.path().join("data"));
    cmd
}

/// fetch with an invalid feed name/id returns a JSON error on stdout
#[test]
fn fetch_unknown_feed_json_error() {
    let home = TempDir::new().unwrap();

    let out = with_home(&home)
        .args(["fetch", "--json", "nonexistent-feed-xyz"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert!(parsed["code"].is_string());
}

/// fetch with no feeds registered returns empty items (no crash)
#[test]
fn fetch_no_feeds_returns_empty_ok() {
    let home = TempDir::new().unwrap();

    let out = with_home(&home).args(["fetch", "--json"]).output().unwrap();

    // With no feeds, should succeed with empty items
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("should emit valid JSON");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert_eq!(parsed["count"], serde_json::json!(0));
}
