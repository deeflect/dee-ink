use std::thread;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use tiny_http::{Response, Server};

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-amazon").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_CONFIG_HOME", home.path().join(".config"));
}

fn start_mock_amazon() -> (String, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("bind test server");
    let base = format!("http://{}/s", server.server_addr());

    let handle = thread::spawn(move || {
        for req in server.incoming_requests().take(1) {
            let body = r#"
                <html><body>
                  <div data-component-type='s-search-result' data-asin='B001'>
                    <h2><a href='/dp/B001'><span>Test Keyboard</span></a></h2>
                    <span class='a-price'><span class='a-offscreen'>$99.99</span></span>
                    <span class='a-icon-alt'>4.5 out of 5 stars</span>
                    <span class='a-size-base s-underline-text'>1,234</span>
                  </div>
                </body></html>
            "#;
            let _ = req.respond(Response::from_string(body).with_status_code(200));
        }
    });

    (base, handle)
}

#[test]
fn config_set_and_show_json_contract() {
    let home = TempDir::new().expect("temp dir");

    let mut set = cmd();
    with_temp_home(&mut set, &home);
    set.args(["config", "set", "amazon.user-agent", "agent", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut show = cmd();
    with_temp_home(&mut show, &home);
    show.args(["config", "show", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"user_agent\":\"agent\""));
}

#[test]
fn search_parses_mock_html_json_contract() {
    let home = TempDir::new().expect("temp dir");
    let (base, handle) = start_mock_amazon();

    let mut search = cmd();
    with_temp_home(&mut search, &home);
    search
        .args([
            "search",
            "keyboard",
            "--base-url",
            &base,
            "--limit",
            "5",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("Test Keyboard"));

    handle.join().expect("server thread should finish");
}

#[test]
fn invalid_base_url_returns_invalid_argument() {
    let home = TempDir::new().expect("temp dir");

    let mut search = cmd();
    with_temp_home(&mut search, &home);
    search
        .args(["search", "keyboard", "--base-url", "notaurl", "--json"])
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
        .stdout(predicate::str::contains("dee-amazon"));
}
