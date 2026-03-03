use std::sync::{Arc, Mutex};
use std::thread;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;
use tiny_http::{Response, Server};

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-pricewatch").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

fn start_price_server(responses: Vec<&str>) -> (String, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("bind test server");
    let address = format!("http://{}/item", server.server_addr());
    let shared = Arc::new(Mutex::new(
        responses
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
    ));
    let shared_ref = Arc::clone(&shared);

    let handle = thread::spawn(move || {
        for request in server.incoming_requests().take(2) {
            let body = {
                let mut guard = shared_ref.lock().expect("lock queue");
                if guard.is_empty() {
                    "<html><body><span>$9.99</span></body></html>".to_string()
                } else {
                    guard.remove(0)
                }
            };

            let response = Response::from_string(body).with_status_code(200);
            let _ = request.respond(response);
        }
    });

    (address, handle)
}

#[test]
fn add_and_list_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args([
        "add",
        "https://example.com/product",
        "--label",
        "Example",
        "--target-price",
        "19.99",
        "--json",
    ])
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
        .stdout(predicate::str::contains("Example"));
}

#[test]
fn json_error_for_missing_watch_uses_stdout() {
    let home = TempDir::new().expect("temp dir");

    let mut delete = cmd();
    with_temp_home(&mut delete, &home);
    delete
        .args(["delete", "missing", "--json"])
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
    add.args(["add", "https://example.com/item"]) // label defaults from host
        .assert()
        .success();

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["list", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn check_detects_drop_and_target_hit() {
    let home = TempDir::new().expect("temp dir");
    let (url, handle) = start_price_server(vec![
        "<html><body><span class='price'>$25.00</span></body></html>",
        "<html><body><span class='price'>$19.00</span></body></html>",
    ]);

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args([
        "add",
        &url,
        "--label",
        "Local Item",
        "--target-price",
        "20",
        "--selector",
        ".price",
        "--json",
    ])
    .assert()
    .success();

    let mut first_check = cmd();
    with_temp_home(&mut first_check, &home);
    let first_output = first_check
        .args(["check", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let first_json: Value = serde_json::from_slice(&first_output).expect("valid json");
    assert_eq!(first_json["ok"], true);
    assert_eq!(first_json["items"][0]["ok"], true);
    assert_eq!(first_json["items"][0]["price"], 25.0);

    let mut second_check = cmd();
    with_temp_home(&mut second_check, &home);
    let second_output = second_check
        .args(["check", "1", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second_json: Value = serde_json::from_slice(&second_output).expect("valid json");
    assert_eq!(second_json["ok"], true);
    assert_eq!(second_json["items"][0]["ok"], true);
    assert_eq!(second_json["items"][0]["price"], 19.0);
    assert_eq!(second_json["items"][0]["dropped"], true);
    assert_eq!(second_json["items"][0]["target_hit"], true);

    handle.join().expect("server thread should finish");
}
