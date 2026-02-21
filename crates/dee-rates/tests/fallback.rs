#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-rates").unwrap()
}

/// When RATES_TEST_BASE_URL points to a non-existent server,
/// get exits non-zero with a JSON error on stdout (not stderr).
#[test]
fn unreachable_base_url_gives_json_error() {
    let out = bin()
        .env("RATES_TEST_BASE_URL", "http://127.0.0.1:1") // refused connection
        .args(["get", "--json", "USD"])
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "should exit non-zero on network error"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert!(parsed["code"].is_string(), "code must be present");
}

/// Same for list command
#[test]
fn list_unreachable_gives_json_error() {
    let out = bin()
        .env("RATES_TEST_BASE_URL", "http://127.0.0.1:1")
        .args(["list", "--json"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["ok"], serde_json::json!(false));
}
