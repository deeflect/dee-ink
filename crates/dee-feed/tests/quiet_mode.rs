#![allow(deprecated)]
use assert_cmd::Command;
use predicates::prelude::*;
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

#[test]
fn help_includes_examples() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("EXAMPLES"));
}

/// list --quiet must print something (the feed ids), not be empty
#[test]
fn list_quiet_prints_ids_not_empty() {
    let home = TempDir::new().unwrap();

    // Add a feed first
    with_home(&home)
        .args(["add", "https://example.com/rss", "--name", "test-feed"])
        .assert()
        .success();

    // list --quiet must not produce empty output
    let out = with_home(&home).args(["list", "--quiet"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "list --quiet should not be empty"
    );
    // Should print the feed id (1)
    assert!(
        stdout.trim().contains('1'),
        "list --quiet should print feed id"
    );
}

/// add --quiet must print the new feed id, not be empty
#[test]
fn add_quiet_prints_id() {
    let home = TempDir::new().unwrap();

    let out = with_home(&home)
        .args([
            "add",
            "--quiet",
            "https://example.com/rss",
            "--name",
            "myfeed",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim().is_empty(), "add --quiet should not be empty");
    let id: i64 = stdout.trim().parse().expect("should print numeric feed id");
    assert!(id > 0);
}

/// remove --quiet must print the removed feed id, not be empty
#[test]
fn remove_quiet_prints_id() {
    let home = TempDir::new().unwrap();

    with_home(&home)
        .args(["add", "https://example.com/rss", "--name", "to-remove"])
        .assert()
        .success();

    let out = with_home(&home)
        .args(["remove", "--quiet", "to-remove"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "remove --quiet should not be empty"
    );
    let id: i64 = stdout.trim().parse().expect("should print numeric feed id");
    assert!(id > 0);
}
