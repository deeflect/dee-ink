use assert_cmd::Command;

#[test]
fn emits_json_error_for_missing_auth() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("dee-ph"));
    cmd.args(["top", "--json"]);

    let out = cmd.assert().failure().get_output().stdout.clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("valid json");

    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["code"], "AUTH_MISSING");
}

#[test]
fn config_path_prints_nonempty() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("dee-ph"));
    cmd.args(["config", "path"]);
    cmd.assert().success();
}
