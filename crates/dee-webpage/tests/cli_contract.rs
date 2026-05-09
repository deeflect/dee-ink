use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
  <title>Agent Test Page</title>
  <meta name="description" content="A page made for CLI contract tests.">
  <link rel="canonical" href="/canonical">
</head>
<body>
  <main>
    <h1>Agent Heading</h1>
    <p>Alpha text for extraction.</p>
    <p>Beta text for extraction.</p>
  </main>
  <a href="/internal" rel="next">Internal link</a>
  <a href="https://example.org/external">External link</a>
  <img src="/agent.png" alt="Agent image">
</body>
</html>"#;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_dee-webpage")
}

fn run(args: &[String]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("dee-webpage command runs")
}

fn run_str(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("dee-webpage command runs")
}

fn serve_once(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{addr}/page")
}

fn serve_once_without_content_length(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n{}",
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    format!("http://{addr}/page")
}

fn stdout_json(args: &[String]) -> serde_json::Value {
    let output = run(args);
    assert!(
        output.status.success(),
        "args={args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout JSON")
}

#[test]
fn help_includes_examples_and_version_succeeds() {
    let help = run_str(&["--help"]);
    assert!(help.status.success());
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("EXAMPLES:"));
    assert!(stdout.contains("dee-webpage metadata https://example.com --json"));

    let version = run_str(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("dee-webpage"));
}

#[test]
fn metadata_json_contract() {
    let url = serve_once(PAGE);
    let value = stdout_json(&["metadata".to_string(), url, "--json".to_string()]);
    assert_eq!(value["ok"], true);
    assert_eq!(value["item"]["title"], "Agent Test Page");
    assert_eq!(
        value["item"]["description"],
        "A page made for CLI contract tests."
    );
    assert_eq!(value["item"]["headings_count"], 1);
    assert_eq!(value["item"]["links_count"], 2);
    assert_eq!(value["item"]["images_count"], 1);
    assert!(value["item"]["content_sha256"].as_str().unwrap().len() >= 64);
}

#[test]
fn text_json_contract() {
    let url = serve_once(PAGE);
    let value = stdout_json(&[
        "text".to_string(),
        url,
        "--max-chars".to_string(),
        "60".to_string(),
        "--json".to_string(),
    ]);
    assert_eq!(value["ok"], true);
    assert_eq!(value["item"]["selector"], "main");
    assert!(value["item"]["text"].as_str().unwrap().contains("Alpha"));
    assert_eq!(value["item"]["truncated"], true);
}

#[test]
fn links_json_contract() {
    let url = serve_once(PAGE);
    let value = stdout_json(&["links".to_string(), url, "--json".to_string()]);
    assert_eq!(value["ok"], true);
    assert_eq!(value["count"], 2);
    assert_eq!(value["items"][0]["text"], "Internal link");
    assert_eq!(value["items"][0]["internal"], true);
    assert_eq!(value["items"][1]["internal"], false);
}

#[test]
fn links_filters_internal_and_external() {
    let internal_url = serve_once(PAGE);
    let internal = stdout_json(&[
        "links".to_string(),
        internal_url,
        "--internal".to_string(),
        "--json".to_string(),
    ]);
    assert_eq!(internal["ok"], true);
    assert_eq!(internal["count"], 1);
    assert_eq!(internal["items"][0]["internal"], true);

    let external_url = serve_once(PAGE);
    let external = stdout_json(&[
        "links".to_string(),
        external_url,
        "--external".to_string(),
        "--json".to_string(),
    ]);
    assert_eq!(external["ok"], true);
    assert_eq!(external["count"], 1);
    assert_eq!(external["items"][0]["internal"], false);
}

#[test]
fn text_selector_flag_extracts_target_region() {
    let url = serve_once(PAGE);
    let value = stdout_json(&[
        "text".to_string(),
        url,
        "--selector".to_string(),
        "h1".to_string(),
        "--json".to_string(),
    ]);
    assert_eq!(value["ok"], true);
    assert_eq!(value["item"]["selector"], "h1");
    assert_eq!(value["item"]["text"], "Agent Heading");
}

#[test]
fn markdown_json_contract() {
    let url = serve_once(PAGE);
    let value = stdout_json(&["markdown".to_string(), url, "--json".to_string()]);
    assert_eq!(value["ok"], true);
    assert_eq!(value["item"]["selector"], "main");
    let markdown = value["item"]["markdown"].as_str().unwrap();
    assert!(markdown.contains("# Agent Heading"));
    assert!(markdown.contains("Alpha text for extraction."));
}

#[test]
fn quiet_mode_has_no_ansi_decorations() {
    let url = serve_once(PAGE);
    let output = run(&["text".to_string(), url, "--quiet".to_string()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\u{1b}["));
    assert!(stdout.contains("Alpha text"));
}

#[test]
fn json_errors_have_machine_readable_shape() {
    let output = run_str(&["metadata", "not-a-url", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGUMENT");
    assert!(value["error"].as_str().unwrap().contains("url"));
}

#[test]
fn clap_errors_can_be_json() {
    let output = run_str(&["text", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "INVALID_ARGUMENT");
}

#[test]
fn max_bytes_is_enforced_without_content_length() {
    let url = serve_once_without_content_length(PAGE);
    let output = run(&[
        "metadata".to_string(),
        url,
        "--max-bytes".to_string(),
        "32".to_string(),
        "--json".to_string(),
    ]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert_eq!(value["code"], "RESPONSE_TOO_LARGE");
}
