mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use support::{fixture_path, temp_db};

#[test]
fn ingest_xml_and_report_counts() {
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
        .success()
        .stdout(predicate::str::contains("Records: 2 | Workouts: 1"));
}
