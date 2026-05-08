use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_dee-package")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("dee-package command runs")
}

#[test]
fn help_includes_examples_and_version_succeeds() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("EXAMPLES:"));
    assert!(stdout.contains("dee-package search crates serde --limit 5 --json"));

    let version = run(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("dee-package"));
}

#[test]
fn unsupported_ecosystem_json_error_shape() {
    let output = run(&["latest", "npm", "react", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert!(value["error"]
        .as_str()
        .unwrap()
        .contains("Unsupported ecosystem"));
    assert_eq!(value["code"], "UNSUPPORTED_ECOSYSTEM");
}

#[test]
fn invalid_limit_json_error_shape() {
    let output = run(&["search", "crates", "serde", "--limit", "0", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGUMENT");
}

#[test]
fn clap_errors_can_be_json() {
    let output = run(&["info", "crates", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGUMENT");
}

#[test]
fn quiet_error_has_no_ansi_decorations() {
    let output = run(&["latest", "npm", "react", "--quiet"]);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty());
    assert!(!stderr.contains("\u{1b}["));
    assert!(stderr.contains("Unsupported ecosystem"));
}
