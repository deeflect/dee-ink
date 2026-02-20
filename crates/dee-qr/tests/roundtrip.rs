#![allow(deprecated)]
use assert_cmd::Command;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("dee-qr").unwrap()
}

/// Generate a QR code as PNG, then decode it — should round-trip correctly
#[test]
fn generate_png_then_decode_roundtrip() {
    let dir = TempDir::new().unwrap();
    let png_path = dir.path().join("test.png");

    // Generate
    bin()
        .args([
            "generate",
            "--format",
            "png",
            "--out",
            png_path.to_str().unwrap(),
            "hello-roundtrip",
        ])
        .assert()
        .success();

    assert!(png_path.exists(), "PNG file should have been created");

    // Decode
    let out = bin()
        .args(["decode", "--json", png_path.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(out.status.success(), "decode should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("decode output must be valid JSON");
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert_eq!(parsed["item"]["data"], serde_json::json!("hello-roundtrip"));
}

/// Generate a QR code as SVG, verify file is created
#[test]
fn generate_svg_creates_file() {
    let dir = TempDir::new().unwrap();
    let svg_path = dir.path().join("test.svg");

    let out = bin()
        .args([
            "generate",
            "--json",
            "--format",
            "svg",
            "--out",
            svg_path.to_str().unwrap(),
            "test-svg-content",
        ])
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["ok"], serde_json::json!(true));
    assert!(svg_path.exists(), "SVG file should have been created");
}

/// --stdin flag reads content from stdin
#[test]
fn generate_stdin_terminal_format() {
    let out = bin()
        .args(["generate", "--stdin", "--format", "terminal"])
        .write_stdin("stdin-test-content")
        .output()
        .unwrap();

    assert!(out.status.success(), "stdin generate should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.trim().is_empty(), "should produce terminal output");
}
