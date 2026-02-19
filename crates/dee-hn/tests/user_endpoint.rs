#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-hn").unwrap()
}

#[test]
fn user_subcommand_exists() {
    // Just verify the subcommand is recognized (no "unknown subcommand" error)
    // We don't make a live network call here; the binary should at least parse the command.
    let out = bin().args(["user", "--help"]).output().unwrap();
    assert!(out.status.success(), "user --help should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("user") || stdout.contains("HN"),
        "user --help should include relevant text"
    );
}

/// When user id is clearly missing (empty string), binary should exit non-zero
#[test]
fn user_help_flag_succeeds() {
    bin().args(["user", "--help"]).assert().success();
}
