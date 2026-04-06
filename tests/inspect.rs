mod support;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

use support::{fixture_path, temp_db};

#[test]
fn inspect_returns_stable_shape() {
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
        .args(["inspect", "--db", db.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json["tables"].is_array());
    assert_eq!(json["record_count"], 2);
    assert_eq!(json["workout_count"], 1);
    assert_eq!(json["date_range"]["start"], "2024-01-15T00:00:00Z");
}
