use anyhow::Result;
use duckdb::Connection;

const SCHEMA_VERSION: &str = "v1.0";

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS records (
          record_type VARCHAR NOT NULL,
          value_text VARCHAR,
          value_num DOUBLE,
          unit VARCHAR,
          source_name VARCHAR,
          source_version VARCHAR,
          device VARCHAR,
          creation_date VARCHAR,
          start_date VARCHAR NOT NULL,
          end_date VARCHAR NOT NULL,
          dedupe_key VARCHAR PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS workouts (
          workout_type VARCHAR NOT NULL,
          duration DOUBLE,
          duration_unit VARCHAR,
          total_distance DOUBLE,
          total_energy_burned DOUBLE,
          source_name VARCHAR,
          creation_date VARCHAR,
          start_date VARCHAR NOT NULL,
          end_date VARCHAR NOT NULL,
          dedupe_key VARCHAR PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS ingest_runs (
          started_at VARCHAR NOT NULL,
          finished_at VARCHAR,
          input_path VARCHAR NOT NULL,
          records_inserted BIGINT,
          workouts_inserted BIGINT,
          records_skipped BIGINT,
          errors_count BIGINT,
          schema_version VARCHAR NOT NULL
        );

        CREATE TABLE IF NOT EXISTS records_staging (
          record_type VARCHAR,
          value_text VARCHAR,
          value_num DOUBLE,
          unit VARCHAR,
          source_name VARCHAR,
          source_version VARCHAR,
          device VARCHAR,
          creation_date VARCHAR,
          start_date VARCHAR,
          end_date VARCHAR,
          dedupe_key VARCHAR
        );

        CREATE TABLE IF NOT EXISTS workouts_staging (
          workout_type VARCHAR,
          duration DOUBLE,
          duration_unit VARCHAR,
          total_distance DOUBLE,
          total_energy_burned DOUBLE,
          source_name VARCHAR,
          creation_date VARCHAR,
          start_date VARCHAR,
          end_date VARCHAR,
          dedupe_key VARCHAR
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
