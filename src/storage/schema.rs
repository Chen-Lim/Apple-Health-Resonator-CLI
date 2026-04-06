use anyhow::Result;
use rusqlite::Connection;

const SCHEMA_VERSION: &str = "v1.0";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS records (
          id INTEGER PRIMARY KEY,
          record_type TEXT NOT NULL,
          value_text TEXT,
          value_num REAL,
          unit TEXT,
          source_name TEXT,
          source_version TEXT,
          device TEXT,
          creation_date TEXT,
          start_date TEXT NOT NULL,
          end_date TEXT NOT NULL,
          dedupe_key TEXT UNIQUE
        );

        CREATE TABLE IF NOT EXISTS workouts (
          id INTEGER PRIMARY KEY,
          workout_type TEXT NOT NULL,
          duration REAL,
          duration_unit TEXT,
          total_distance REAL,
          total_energy_burned REAL,
          source_name TEXT,
          creation_date TEXT,
          start_date TEXT NOT NULL,
          end_date TEXT NOT NULL,
          dedupe_key TEXT UNIQUE
        );

        CREATE TABLE IF NOT EXISTS ingest_runs (
          id INTEGER PRIMARY KEY,
          started_at TEXT NOT NULL,
          finished_at TEXT,
          input_path TEXT NOT NULL,
          records_inserted INTEGER,
          workouts_inserted INTEGER,
          records_skipped INTEGER,
          errors_count INTEGER,
          schema_version TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_records_type_date ON records(record_type, start_date);
        CREATE INDEX IF NOT EXISTS idx_records_source_date ON records(source_name, start_date);
        CREATE INDEX IF NOT EXISTS idx_workouts_type_date ON workouts(workout_type, start_date);
        "#,
    )?;
    Ok(())
}

pub fn schema_version() -> &'static str {
    SCHEMA_VERSION
}
