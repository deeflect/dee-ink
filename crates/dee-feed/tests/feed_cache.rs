#![allow(deprecated)]
use assert_cmd::Command;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("dee-feed").unwrap()
}

fn with_home(dir: &TempDir) -> Command {
    let mut cmd = bin();
    cmd.env("HOME", dir.path());
    cmd.env("XDG_CONFIG_HOME", dir.path().join("config"));
    cmd.env("XDG_DATA_HOME", dir.path().join("data"));
    cmd
}

fn db_path(home: &TempDir) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return home
            .path()
            .join("Library")
            .join("Application Support")
            .join("dee-feed")
            .join("feed.db");
    }

    #[cfg(not(target_os = "macos"))]
    {
        home.path().join("data").join("dee-feed").join("feed.db")
    }
}

/// Verify that feed list operations produce valid JSON with ok=true
#[test]
fn list_json_ok_structure() {
    let home = TempDir::new().unwrap();

    let out = with_home(&home).args(["list", "--json"]).output().unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("list --json must emit valid JSON");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert!(parsed["count"].is_number());
    assert!(parsed["items"].is_array());
}

/// Adding a feed persists it; listing shows it with correct name
#[test]
fn add_then_list_shows_feed() {
    let home = TempDir::new().unwrap();

    with_home(&home)
        .args(["add", "https://example.com/feed.xml", "--name", "my-feed"])
        .assert()
        .success();

    let out = with_home(&home).args(["list", "--json"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["count"], serde_json::json!(1));
    let items = parsed["items"].as_array().unwrap();
    assert_eq!(items[0]["name"], serde_json::json!("my-feed"));
}

/// read on a non-existent item id gives a JSON error on stdout
#[test]
fn read_missing_item_json_error_on_stdout() {
    let home = TempDir::new().unwrap();

    let out = with_home(&home)
        .args(["read", "--json", "99999"])
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error must be valid JSON on stdout");
    assert_eq!(parsed["ok"], serde_json::json!(false));
    assert!(parsed["code"].is_string());
}

/// read should report the updated read-state in the same response
#[test]
fn read_returns_item_as_read_after_marking() {
    let home = TempDir::new().unwrap();

    with_home(&home)
        .args([
            "add",
            "https://example.com/feed.xml",
            "--name",
            "fixture",
            "--json",
        ])
        .assert()
        .success();

    let conn = Connection::open(db_path(&home)).unwrap();
    conn.execute(
        "INSERT INTO items (feed_id, ext_id, title, url, summary, published, read) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
        rusqlite::params![
            1_i64,
            "fixture-ext-id",
            "Fixture title",
            "https://example.com/item",
            "Fixture summary",
            "2026-02-25T20:00:00+00:00"
        ],
    )
    .unwrap();
    let item_id = conn.last_insert_rowid();

    let read = with_home(&home)
        .args(["read", "--json", &item_id.to_string()])
        .output()
        .unwrap();
    assert!(read.status.success());
    let read_json: serde_json::Value =
        serde_json::from_slice(&read.stdout).expect("read --json should return valid JSON");
    assert_eq!(read_json["ok"], serde_json::json!(true));
    assert_eq!(read_json["item"]["id"], serde_json::json!(item_id));
    assert_eq!(read_json["item"]["read"], serde_json::json!(true));
}
