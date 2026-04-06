mod support;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

use support::{create_zip_fixture, create_zip_fixture_with_entry_name, fixture_path, temp_db};

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

#[test]
fn ingest_zip_input_with_localized_xml_name() {
    let temp = tempdir().unwrap();
    let db = temp_db(&temp);
    let zip_path = temp.path().join("localized-export.zip");
    create_zip_fixture_with_entry_name(&fixture_path("export-small.xml"), &zip_path, "导出.xml")
        .unwrap();

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

#[test]
fn ingest_zip_input_with_nested_localized_xml_name() {
    let temp = tempdir().unwrap();
    let db = temp_db(&temp);
    let zip_path = temp.path().join("nested-localized-export.zip");
    create_zip_fixture_with_entry_name(
        &fixture_path("export-small.xml"),
        &zip_path,
        "apple_health/导出.xml",
    )
    .unwrap();

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
