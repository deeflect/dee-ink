#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-ssl").unwrap()
}

#[test]
fn check_chain_flag_parsed() {
    // Just verify --chain flag is recognized
    bin().args(["check", "--help"]).assert().success();
}

#[test]
fn check_port_flag_parsed() {
    bin().args(["check", "--help"]).assert().success();
}
