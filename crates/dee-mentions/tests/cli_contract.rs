use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-mentions").expect("binary should build")
    }
}

fn with_temp_home(command: &mut Command, home: &TempDir) {
    command.env("HOME", home.path());
    command.env("XDG_DATA_HOME", home.path().join(".local/share"));
}

#[test]
fn check_json_contract_with_mock_sources() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/api/v1/search");
        then.status(200).json_body_obj(&serde_json::json!({
            "hits": [
                {
                    "title": "dee.ink launch",
                    "url": "https://news.ycombinator.com/item?id=1",
                    "author": "alice",
                    "points": 10,
                    "created_at": "2026-02-26T00:00:00Z"
                }
            ]
        }));
    });

    server.mock(|when, then| {
        when.method(GET).path("/search.json");
        then.status(200).json_body_obj(&serde_json::json!({
            "data": {
                "children": [
                    {
                        "data": {
                            "title": "I use dee.ink",
                            "permalink": "/r/test/comments/1/abc",
                            "selftext": "Useful tool",
                            "author": "bob",
                            "score": 5,
                            "created_utc": 1700000000
                        }
                    }
                ]
            }
        }));
    });

    let mut c = cmd();
    c.env("DEE_MENTIONS_HN_BASE", server.base_url())
        .env("DEE_MENTIONS_REDDIT_BASE", server.base_url())
        .args([
            "check",
            "dee.ink",
            "--sources",
            "hn,reddit",
            "--limit",
            "5",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"count\":2"))
        .stdout(predicate::str::contains("dee.ink launch"))
        .stdout(predicate::str::contains("I use dee.ink"));
}

#[test]
fn watch_add_list_run_remove_flow() {
    let server = MockServer::start();
    let home = TempDir::new().expect("temp dir");

    server.mock(|when, then| {
        when.method(GET).path("/api/v1/search");
        then.status(200)
            .json_body_obj(&serde_json::json!({"hits": []}));
    });

    server.mock(|when, then| {
        when.method(GET).path("/search.json");
        then.status(200)
            .json_body_obj(&serde_json::json!({"data": {"children": []}}));
    });

    let mut add = cmd();
    with_temp_home(&mut add, &home);
    add.args(["watch", "add", "dee.ink", "--tag", "brand", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut list = cmd();
    with_temp_home(&mut list, &home);
    list.args(["watch", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\":1"));

    let mut run = cmd();
    with_temp_home(&mut run, &home);
    run.env("DEE_MENTIONS_HN_BASE", server.base_url())
        .env("DEE_MENTIONS_REDDIT_BASE", server.base_url())
        .args(["run", "--all", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"));

    let mut remove = cmd();
    with_temp_home(&mut remove, &home);
    remove
        .args(["watch", "remove", "1", "--json"])
        .assert()
        .success();
}

#[test]
fn json_error_for_run_without_selector() {
    let mut c = cmd();
    c.args(["run", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"code\":\"INVALID_ARGUMENT\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn quiet_mode_emits_data() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/api/v1/search");
        then.status(200)
            .json_body_obj(&serde_json::json!({"hits": []}));
    });

    let mut c = cmd();
    c.env("DEE_MENTIONS_HN_BASE", server.base_url())
        .args(["check", "dee.ink", "--sources", "hn", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0"));
}
