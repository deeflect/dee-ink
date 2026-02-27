use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-invoice").expect("binary should build")
    }
}

fn sample_yaml() -> String {
    r#"invoice_number: INV-100
issue_date: 2026-02-26
due_date: 2026-03-12
currency: USD
seller:
  name: Dee Agency
  email: billing@dee.ink
buyer:
  name: Client Co
  email: ap@client.co
items:
  - description: Design sprint
    quantity: 8
    unit_price: 120
  - description: Implementation
    quantity: 12
    unit_price: 140
tax_rate: 10
notes: Thank you
"#
    .to_string()
}

#[test]
fn calc_json_contract() {
    let dir = TempDir::new().expect("temp dir");
    let input = dir.path().join("invoice.yaml");
    fs::write(&input, sample_yaml()).expect("write input");

    let mut c = cmd();
    c.args(["calc", input.to_string_lossy().as_ref(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"subtotal\":2640.0"))
        .stdout(predicate::str::contains("\"total\":2904.0"));
}

#[test]
fn generate_pdf_creates_file() {
    let dir = TempDir::new().expect("temp dir");
    let input = dir.path().join("invoice.yaml");
    let output = dir.path().join("invoice.pdf");
    fs::write(&input, sample_yaml()).expect("write input");

    let mut c = cmd();
    c.args([
        "generate",
        input.to_string_lossy().as_ref(),
        "--format",
        "pdf",
        "--output",
        output.to_string_lossy().as_ref(),
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"ok\":true"));

    let meta = fs::metadata(output).expect("pdf exists");
    assert!(meta.len() > 100, "pdf should be non-empty");
}

#[test]
fn json_error_on_invalid_input_goes_stdout() {
    let dir = TempDir::new().expect("temp dir");
    let input = dir.path().join("bad.yaml");
    fs::write(&input, "invoice_number: \"\"\nitems: []\n").expect("write input");

    let mut c = cmd();
    c.args(["calc", input.to_string_lossy().as_ref(), "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"code\":\"PARSE_FAILED\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn quiet_mode_emits_data() {
    let mut c = cmd();
    c.args(["template", "--format", "yaml", "--quiet"])
        .assert()
        .success();
}
