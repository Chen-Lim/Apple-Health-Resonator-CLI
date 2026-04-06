mod support;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

use support::{fixture_path, temp_db};

#[test]
fn stats_returns_compact_json() {
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
        .args(["stats", "--db", db.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json["total_records"], 2);
    assert_eq!(json["total_workouts"], 1);
    assert!(json["top_types"].is_array());
}
