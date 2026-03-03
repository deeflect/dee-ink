use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-transit").expect("binary should build")
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
    set.args(["config", "set", "google.api-key", "key123", "--json"])
        .assert()
        .success();

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["config", "show", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"api_key\":\"key123\""));
}

#[test]
fn route_without_auth_emits_json_error_on_stdout() {
    let home = TempDir::new().expect("temp dir");

    let mut route = cmd();
    with_temp_home(&mut route, &home);
    route
        .args(["route", "A", "B", "--json"])
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
        .stdout(predicate::str::contains("dee-transit"));
}
