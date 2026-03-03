use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-receipt").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_CONFIG_HOME", home.path().join(".config"));
}

#[test]
fn config_set_and_show_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut set = cmd();
    with_temp_home(&mut set, &home);
    set.args(["config", "set", "openai.api-key", "sk-test", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["config", "show", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"openai_api_key\":\"sk-test\""));
}

#[test]
fn scan_without_api_key_emits_json_error_on_stdout() {
    let home = TempDir::new().expect("temp dir");
    let image_path = home.path().join("receipt.jpg");
    fs::write(&image_path, b"fake-image").expect("write image");

    let mut scan = cmd();
    with_temp_home(&mut scan, &home);
    scan.args(["scan", image_path.to_str().unwrap(), "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"code\":\"AUTH_MISSING\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn invalid_config_key_returns_invalid_argument_code() {
    let home = TempDir::new().expect("temp dir");

    let mut set = cmd();
    with_temp_home(&mut set, &home);
    set.args(["config", "set", "bad.key", "value", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"INVALID_ARGUMENT\""));
}

#[test]
fn quiet_path_output_is_non_empty() {
    let home = TempDir::new().expect("temp dir");

    let mut path = cmd();
    with_temp_home(&mut path, &home);
    path.args(["config", "path", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dee-receipt"));
}

#[test]
fn config_show_json_omits_unset_null_fields() {
    let home = TempDir::new().expect("temp dir");

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    let out = show
        .args(["config", "show", "--json"])
        .output()
        .expect("run config show");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains(":null"),
        "config show JSON must not contain null values"
    );
}
