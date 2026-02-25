use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-timer").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

#[test]
fn start_status_stop_flow_json() {
    let home = TempDir::new().expect("temp dir");

    let mut start = cmd();
    with_temp_home(&mut start, &home);
    start
        .args(["start", "Write docs", "--project", "launch", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"id\":"));

    let mut status = cmd();
    with_temp_home(&mut status, &home);
    status
        .args(["status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"active\":true"));

    let mut stop = cmd();
    with_temp_home(&mut stop, &home);
    stop.args(["stop", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));
}

#[test]
fn report_and_show_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut start = cmd();
    with_temp_home(&mut start, &home);
    start
        .args(["start", "Session one", "--project", "alpha"])
        .assert()
        .success();

    let mut stop = cmd();
    with_temp_home(&mut stop, &home);
    stop.args(["stop"]).assert().success();

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["show", "1", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"item\""))
        .stdout(predicate::str::contains("Session one"));

    let mut report = cmd();
    with_temp_home(&mut report, &home);
    report
        .args(["report", "--period", "all", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"items\""))
        .stdout(predicate::str::contains("alpha"));
}

#[test]
fn json_errors_for_active_and_not_found() {
    let home = TempDir::new().expect("temp dir");

    let mut start = cmd();
    with_temp_home(&mut start, &home);
    start.args(["start", "A", "--json"]).assert().success();

    let mut second_start = cmd();
    with_temp_home(&mut second_start, &home);
    second_start
        .args(["start", "B", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "\"code\":\"ACTIVE_SESSION_EXISTS\"",
        ))
        .stderr(predicate::str::is_empty());

    let mut missing = cmd();
    with_temp_home(&mut missing, &home);
    missing
        .args(["show", "999", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"NOT_FOUND\""));
}

#[test]
fn quiet_mode_emits_data() {
    let home = TempDir::new().expect("temp dir");

    let mut start = cmd();
    with_temp_home(&mut start, &home);
    start.args(["start", "Quiet session"]).assert().success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--status", "all", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}
