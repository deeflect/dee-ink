use assert_cmd::Command;

#[test]
fn emits_json_error_for_missing_auth() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("dee-events"));
    cmd.args(["search", "Austin", "--json"]);

    let out = cmd.assert().failure().get_output().stdout.clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("valid json");

    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["code"], "AUTH_MISSING");
}
