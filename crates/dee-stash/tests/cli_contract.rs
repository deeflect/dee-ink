use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-stash").expect("binary should build")
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
        "https://example.com",
        "--title",
        "Example",
        "--tags",
        "research,tools",
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ok\":true"));

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--status", "all", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"));

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"item\""))
        .stdout(predicate::str::contains("https://example.com"));
}

#[test]
fn archive_and_search_flow() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "https://rust-lang.org", "--notes", "language site"])
        .assert()
        .success();

    let mut arch = cmd();
    with_temp_home(&mut arch, &home);
    arch.args(["archive", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut search = cmd();
    with_temp_home(&mut search, &home);
    search
        .args(["search", "rust", "--status", "archived", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"));
}

#[test]
fn json_error_on_missing_bookmark() {
    let home = TempDir::new().expect("temp dir");

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "999", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"NOT_FOUND\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn export_import_roundtrip_json() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "https://example.org", "--title", "Org"])
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

    let file = home.path().join("stash.json");
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
}

#[test]
fn quiet_mode_emits_data() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["add", "https://quiet.example"])
        .assert()
        .success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--status", "all", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}
