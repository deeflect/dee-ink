use assert_cmd::Command;
use httpmock::Method::GET;
use httpmock::MockServer;
use predicates::prelude::*;

fn cmd() -> Command {
    #[allow(deprecated)]
    {
        Command::cargo_bin("dee-trends").expect("binary should build")
    }
}

fn explore_body() -> String {
    concat!(
        ")]}\'\n",
        "{\"widgets\":[",
        "{\"id\":\"TIMESERIES\",\"title\":\"Interest over time\",\"token\":\"tok-ts\",\"request\":{\"a\":1}},",
        "{\"id\":\"RELATED_QUERIES\",\"title\":\"Related queries\",\"token\":\"tok-rq\",\"request\":{\"b\":2}}",
        "]}"
    )
    .to_string()
}

#[test]
fn interest_json_contract() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/explore");
        then.status(200)
            .header("content-type", "application/json")
            .body(explore_body());
    });

    server.mock(|when, then| {
        when.method(GET).path("/widgetdata/multiline");
        then.status(200).body(
            ")]}\'\n{\"default\":{\"timelineData\":[{\"time\":\"1700000000\",\"formattedTime\":\"Nov 2023\",\"value\":[42]}]}}",
        );
    });

    let mut c = cmd();
    c.env("DEE_TRENDS_BASE_URL", server.base_url())
        .args(["interest", "rust", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("\"count\":1"))
        .stdout(predicate::str::contains("\"value\":42"));
}

#[test]
fn related_json_contract() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/explore");
        then.status(200).body(explore_body());
    });

    server.mock(|when, then| {
        when.method(GET).path("/widgetdata/relatedsearches");
        then.status(200).body(
            ")]}\'\n{\"default\":{\"rankedList\":[{\"rankedKeyword\":[{\"query\":\"rust async\",\"queryType\":\"top\",\"value\":100,\"formattedValue\":\"100\"}]}]}}",
        );
    });

    let mut c = cmd();
    c.env("DEE_TRENDS_BASE_URL", server.base_url())
        .args(["related", "rust", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\":true"))
        .stdout(predicate::str::contains("rust async"))
        .stdout(predicate::str::contains("\"query_type\":\"top\""));
}

#[test]
fn json_error_goes_to_stdout() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/explore");
        then.status(500);
    });

    let mut c = cmd();
    c.env("DEE_TRENDS_BASE_URL", server.base_url())
        .args(["interest", "rust", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\":false"))
        .stdout(predicate::str::contains("\"code\":\"API_ERROR\""))
        .stderr(predicate::str::is_empty());
}

#[test]
fn quiet_mode_emits_data() {
    let server = MockServer::start();

    server.mock(|when, then| {
        when.method(GET).path("/explore");
        then.status(200)
            .header("content-type", "application/json")
            .body(explore_body());
    });

    let mut c = cmd();
    c.env("DEE_TRENDS_BASE_URL", server.base_url())
        .args(["explore", "rust", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}
