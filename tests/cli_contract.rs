use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn root_help_describes_available_commands() {
    Command::cargo_bin("ahr")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Import Apple Health exports into SQLite"))
        .stdout(predicate::str::contains("ingest"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("query"));
}

#[test]
fn root_version_is_available() {
    Command::cargo_bin("ahr")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ahr"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn ingest_help_describes_arguments() {
    Command::cargo_bin("ahr")
        .unwrap()
        .args(["ingest", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Path to Apple Health export.xml or export.zip"))
        .stdout(predicate::str::contains("SQLite database path to create or update"))
        .stdout(predicate::str::contains("Number of rows to write per batch"))
        .stdout(predicate::str::contains("Disable progress output"));
}

#[test]
fn query_help_describes_sql_and_limit() {
    Command::cargo_bin("ahr")
        .unwrap()
        .args(["query", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Single read-only SQL statement to execute"))
        .stdout(predicate::str::contains("Maximum rows to emit in CLI output"));
}
