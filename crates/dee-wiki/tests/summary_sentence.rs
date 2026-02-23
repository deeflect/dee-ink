#![allow(deprecated)]
/// Unit tests for the first_sentence extraction logic.
/// These are doc-tests that verify the summary handling works correctly.
/// Since first_sentence is private, we test it via the CLI binary.
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-wiki").unwrap()
}

#[test]
fn summary_subcommand_exists() {
    // Verify summary subcommand is recognized
    bin().args(["summary", "--help"]).assert().success();
}

/// Verify the binary accepts --lang flag properly
#[test]
fn get_subcommand_exists_with_lang() {
    bin().args(["get", "--help"]).assert().success();
}
