use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-habit").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

#[test]
fn add_and_list_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Drink water", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"id\":"));

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("Drink water"));
}

#[test]
fn done_and_streak_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Read", "--json"]).assert().success();

    let mut done = cmd();
    with_temp_home(&mut done, &home);
    done.args(["done", "Read", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut streak = cmd();
    with_temp_home(&mut streak, &home);
    streak
        .args(["streak", "Read", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"current_streak\":1"));
}

#[test]
fn json_error_for_missing_habit_uses_stdout() {
    let home = TempDir::new().expect("temp dir");

    let mut streak = cmd();
    with_temp_home(&mut streak, &home);
    streak
        .args(["streak", "missing", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"code\":\"NOT_FOUND\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn quiet_mode_emits_data() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Quiet habit"]).assert().success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}
