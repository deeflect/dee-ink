use assert_cmd::Command;
use chrono::{Duration, Utc};
use httpmock::Method::POST;
use httpmock::MockServer;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-crosspost").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

#[test]
fn auth_set_and_status_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut set = cmd();
    with_temp_home(&mut set, &home);
    set.args([
        "auth",
        "set-token",
        "--platform",
        "bluesky",
        "--token",
        "token-1",
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ok\":true"));

    let mut status = cmd();
    with_temp_home(&mut status, &home);
    status
        .args(["auth", "status", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":5"))
        .stdout(predicate::str::contains("\"platform\":\"bluesky\""));
}

#[test]
fn queue_schedule_list_show_cancel_flow() {
    let home = TempDir::new().expect("temp dir");
    let at = (Utc::now() + Duration::minutes(30)).to_rfc3339();

    let mut schedule = cmd();
    with_temp_home(&mut schedule, &home);
    let output = schedule
        .args([
            "schedule",
            "--at",
            &at,
            "--to",
            "x,linkedin",
            "--text",
            "test post",
            "--json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let body = String::from_utf8(output).expect("utf8");
    let value: serde_json::Value = serde_json::from_str(&body).expect("valid json");
    let id = value["id"].as_str().expect("id present").to_string();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["queue", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"));

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["queue", "show", &id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"item\""))
        .stdout(predicate::str::contains("\"targets\""));

    let mut cancel = cmd();
    with_temp_home(&mut cancel, &home);
    cancel
        .args(["queue", "cancel", &id, "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"job canceled\""));
}

#[test]
fn run_requires_mode() {
    let mut c = cmd();
    c.args(["run", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"INVALID_ARGUMENT\""));
}

#[test]
fn post_x_uses_mock_and_returns_sent() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(POST).path("/2/tweets");
        then.status(200).json_body_obj(&serde_json::json!({
            "data": {"id": "12345"}
        }));
    });

    let mut c = cmd();
    c.env("DEE_CROSSPOST_X_TOKEN", "tok")
        .env("DEE_CROSSPOST_X_BASE", server.base_url())
        .args(["post", "--to", "x", "--text", "hello", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"sent\""))
        .stdout(predicate::str::contains("\"platform\":\"x\""));
}

#[test]
fn run_once_processes_due_jobs() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(POST).path("/2/tweets");
        then.status(200).json_body_obj(&serde_json::json!({
            "data": {"id": "888"}
        }));
    });

    let home = TempDir::new().expect("temp dir");
    let at = (Utc::now() + Duration::seconds(1)).to_rfc3339();

    let mut schedule = cmd();
    with_temp_home(&mut schedule, &home);
    schedule
        .args([
            "schedule",
            "--at",
            &at,
            "--to",
            "x",
            "--text",
            "scheduled",
            "--json",
        ])
        .assert()
        .success();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let mut run = cmd();
    with_temp_home(&mut run, &home);
    run.env("DEE_CROSSPOST_X_TOKEN", "tok")
        .env("DEE_CROSSPOST_X_BASE", server.base_url())
        .args(["run", "--once", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"jobs_processed\":1"))
        .stdout(predicate::str::contains("\"targets_sent\":1"));
}
