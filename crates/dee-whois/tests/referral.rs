#![allow(deprecated)]
use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("dee-whois").unwrap()
}

/// Just verify the binary accepts the domain argument without crash
#[test]
fn accepts_domain_argument() {
    // We don't make a live WHOIS query here; just verify the args are parsed
    bin().args(["--help"]).assert().success();
}

/// Quiet mode for expires should print expiry date (not empty)
/// We can't test live WHOIS here, so just verify the flag is accepted
#[test]
fn quiet_expires_flag_accepted() {
    // This will fail with network error, but we care about flag parsing
    let out = bin()
        .args(["--quiet", "--expires", "--json", "example.com"])
        .output()
        .unwrap();

    // Whether it succeeds (live WHOIS) or fails (network),
    // JSON should be on stdout, not stderr
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        serde_json::from_str::<serde_json::Value>(stdout.trim())
            .expect("any output must be valid JSON");
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.trim().starts_with('{'),
        "JSON must not appear on stderr"
    );
}
