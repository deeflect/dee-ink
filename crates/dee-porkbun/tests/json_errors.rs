use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_includes_examples() {
    Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"))
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("EXAMPLES:"));
}

#[test]
fn config_set_invalid_key_json_error() {
    Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"))
        .args(["config", "set", "bad_key", "value", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"ok\":false"))
        .stdout(contains("\"code\":\"INVALID_ARGUMENT\""));
}

#[test]
fn domains_create_requires_confirm_json_error() {
    Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"))
        .args([
            "domains",
            "create",
            "example.com",
            "--cost",
            "1108",
            "--agree-to-terms",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"ok\":false"))
        .stdout(contains("\"code\":\"CONFIRM_REQUIRED\""));
}

#[test]
fn dns_create_requires_confirm_json_error() {
    Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"))
        .args([
            "dns",
            "create",
            "example.com",
            "--type",
            "A",
            "--name",
            "www",
            "--content",
            "1.1.1.1",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"ok\":false"))
        .stdout(contains("\"code\":\"CONFIRM_REQUIRED\""));
}

#[test]
fn update_auto_renew_requires_confirm_json_error() {
    Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"))
        .args([
            "domains",
            "update-auto-renew",
            "on",
            "example.com",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"ok\":false"))
        .stdout(contains("\"code\":\"CONFIRM_REQUIRED\""));
}

#[test]
fn domains_check_without_config_returns_config_missing() {
    let mut home = std::env::temp_dir();
    home.push(format!(
        "dee_ink_porkbun_test_no_config_{}",
        std::process::id()
    ));
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("dee-porkbun"));
    cmd.env("HOME", home)
        .args(["domains", "check", "example.com", "--json"])
        .assert()
        .failure()
        .stdout(contains("\"code\":\"CONFIG_MISSING\""));
}
