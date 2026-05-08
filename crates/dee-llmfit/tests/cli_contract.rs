use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_dee-llmfit")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .output()
        .expect("dee-llmfit command runs")
}

fn stdout_json(args: &[&str]) -> serde_json::Value {
    let output = run(args);
    assert!(
        output.status.success(),
        "args={args:?}\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn help_includes_examples_and_version_succeeds() {
    let help = run(&["--help"]);
    assert!(help.status.success());
    let help_stdout = String::from_utf8_lossy(&help.stdout);
    assert!(help_stdout.contains("EXAMPLES:"));
    assert!(help_stdout.contains("dee-llmfit recommend --use-case coding --json"));

    let version = run(&["--version"]);
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("dee-llmfit"));
}

#[test]
fn search_json_contract() {
    let value = stdout_json(&["search", "santacoder", "--json"]);
    assert_eq!(value["ok"], true);
    assert!(value["count"].as_u64().unwrap() >= 1);
    assert!(value["items"].as_array().unwrap()[0]["name"].as_str().is_some());
}

#[test]
fn system_json_contract() {
    let value = stdout_json(&["system", "--json"]);
    assert_eq!(value["ok"], true);
    assert!(value["item"]["total_ram_gb"].as_f64().is_some());
    assert!(value["item"]["cpu_cores"].as_u64().is_some());
}

#[test]
fn info_and_plan_json_contract() {
    let info = stdout_json(&["info", "bigcode/gpt_bigcode-santacoder", "--json"]);
    assert_eq!(info["ok"], true);
    assert_eq!(info["item"]["name"], "bigcode/gpt_bigcode-santacoder");

    let plan = stdout_json(&[
        "plan",
        "bigcode/gpt_bigcode-santacoder",
        "--context",
        "8192",
        "--json",
    ]);
    assert_eq!(plan["ok"], true);
    assert_eq!(plan["item"]["model_name"], "bigcode/gpt_bigcode-santacoder");
}

#[test]
fn json_errors_have_machine_readable_shape() {
    let output = run(&["info", "qwen", "--json"]);
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(value["ok"], false);
    assert!(value["error"].as_str().unwrap().contains("ambiguous"));
    assert_eq!(value["code"], "AMBIGUOUS");
}

#[test]
fn quiet_mode_has_no_ansi_decorations() {
    let output = run(&["search", "santacoder", "--quiet"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\u{1b}["));
    assert!(stdout.contains("santacoder"));
}
