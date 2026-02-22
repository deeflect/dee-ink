#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-whois").unwrap()
}

#[test]
fn help_shows_examples() {
    let out = bin().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("EXAMPLES"), "help must include EXAMPLES");
}

#[test]
fn version_flag() {
    bin().arg("--version").assert().success();
}
