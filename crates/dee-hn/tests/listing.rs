#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-hn").unwrap()
}

#[test]
fn help_includes_examples_with_dee_hn() {
    let out = bin().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("EXAMPLES"), "help must include EXAMPLES");
    assert!(
        stdout.contains("dee-hn"),
        "examples must reference dee-hn"
    );
    assert!(!stdout.contains("ink-hn"), "must not reference ink-hn");
}

#[test]
fn version_flag_succeeds() {
    bin().arg("--version").assert().success();
}
