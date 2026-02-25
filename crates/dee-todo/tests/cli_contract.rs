use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-todo").expect("binary should build")
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
    add.args(["add", "Write tests", "--priority", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"id\":"));

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--status", "all", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("Write tests"));
}

#[test]
fn json_error_for_missing_todo_uses_stdout() {
    let home = TempDir::new().expect("temp dir");

    let mut done = cmd();
    with_temp_home(&mut done, &home);
    done.args(["done", "999", "--json"])
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
    add.args(["add", "Quiet todo"]).assert().success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--status", "all", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn show_returns_single_item_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Single item todo", "--json"])
        .assert()
        .success();

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"item\""))
        .stdout(predicate::str::contains("Single item todo"));
}

#[test]
fn edit_requires_at_least_one_field() {
    let home = TempDir::new().expect("temp dir");

    let mut edit = cmd();
    with_temp_home(&mut edit, &home);
    edit.args(["edit", "1", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"INVALID_ARGUMENT\""));
}
