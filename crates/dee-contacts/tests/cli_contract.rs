use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-contacts").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

#[test]
fn add_list_show_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args([
        "add",
        "Ada Lovelace",
        "--email",
        "ada@example.com",
        "--tags",
        "founder,math",
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ok\":true"));

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("Ada Lovelace"));

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "ada", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"item\""))
        .stdout(predicate::str::contains("\"interaction_count\":0"));
}

#[test]
fn interaction_add_and_list_json() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Linus Torvalds", "--json"])
        .assert()
        .success();

    let mut iadd = cmd();
    with_temp_home(&mut iadd, &home);
    iadd.args([
        "interaction",
        "add",
        "linus",
        "--kind",
        "note",
        "--summary",
        "intro call",
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ok\":true"));

    let mut ilist = cmd();
    with_temp_home(&mut ilist, &home);
    ilist
        .args(["interaction", "list", "linus", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("intro call"));
}

#[test]
fn ambiguous_fuzzy_name_returns_json_error_stdout() {
    let home = TempDir::new().expect("temp dir");

    for name in ["Ada One", "Ada Two"] {
        let mut add = cmd();
        with_temp_home(&mut add, &home);
        add.args(["add", name]).assert().success();
    }

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "ada", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"AMBIGUOUS\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn export_import_json_roundtrip() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Grace Hopper", "--company", "Navy"])
        .assert()
        .success();

    let mut export = cmd();
    with_temp_home(&mut export, &home);
    let out = export
        .args(["export", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let file = home.path().join("contacts.json");
    fs::write(&file, out).expect("write export file");

    let home2 = TempDir::new().expect("temp dir");
    let mut import = cmd();
    with_temp_home(&mut import, &home2);
    import
        .args([
            "import",
            "--format",
            "json",
            file.to_string_lossy().as_ref(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"));

    let mut list = cmd();
    with_temp_home(&mut list, &home2);
    list.args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Grace Hopper"));
}

#[test]
fn quiet_mode_emits_data() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "Quiet User"]).assert().success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}
