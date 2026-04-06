mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use support::{create_zip_fixture, fixture_path, temp_db};

#[test]
fn ingest_zip_input() {
    let temp = tempdir().unwrap();
    let db = temp_db(&temp);
    let zip_path = temp.path().join("export-small.zip");
    create_zip_fixture(&fixture_path("export-small.xml"), &zip_path).unwrap();

    Command::cargo_bin("ahr")
        .unwrap()
        .args([
            "ingest",
            zip_path.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
            "--quiet",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Workouts: 1"));
}
