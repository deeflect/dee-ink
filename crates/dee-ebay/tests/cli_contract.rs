use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-ebay").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_CONFIG_HOME", home.path().join(".config"));
}

#[test]
fn config_set_and_show_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut set_id = cmd();
    with_temp_home(&mut set_id, &home);
    set_id
        .args(["config", "set", "ebay.client-id", "id123", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut set_secret = cmd();
    with_temp_home(&mut set_secret, &home);
    set_secret
        .args(["config", "set", "ebay.client-secret", "sec123", "--json"])
        .assert()
        .success();

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["config", "show", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"client_id\":\"id123\""));
}

#[test]
fn search_without_auth_emits_json_error_on_stdout() {
    let home = TempDir::new().expect("temp dir");

    let mut search = cmd();
    with_temp_home(&mut search, &home);
    search
        .args(["search", "camera", "--json"])
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
        .stdout(predicate::str::contains("dee-ebay"));
}
