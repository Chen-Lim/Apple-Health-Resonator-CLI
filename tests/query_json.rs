mod support;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

use support::{fixture_path, temp_db};

#[test]
fn query_returns_json_array_and_rejects_mutation() {
    let temp = tempdir().unwrap();
    let db = temp_db(&temp);

    Command::cargo_bin("ahr")
        .unwrap()
        .args([
            "ingest",
            fixture_path("export-small.xml").to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
            "--quiet",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("ahr")
        .unwrap()
        .args([
            "query",
            "--db",
            db.to_str().unwrap(),
            "--sql",
            "SELECT record_type, value_num FROM records ORDER BY id",
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);

    Command::cargo_bin("ahr")
        .unwrap()
        .args([
            "query",
            "--db",
            db.to_str().unwrap(),
            "--sql",
            "SELECT 1; DROP TABLE records",
        ])
        .assert()
        .failure();
}
