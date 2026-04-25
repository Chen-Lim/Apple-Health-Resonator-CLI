use anyhow::Result;
use duckdb::{params, Connection};

use crate::domain::{Record, Workout};

pub struct BatchWriter<'a> {
    conn: &'a mut Connection,
    batch_size: usize,
    record_batch: Vec<Record>,
    workout_batch: Vec<Workout>,
    records_inserted: i64,
    workouts_inserted: i64,
    records_skipped: i64,
}

impl<'a> BatchWriter<'a> {
    pub fn new(conn: &'a mut Connection, batch_size: usize) -> Result<Self> {
        // Clear staging tables in case a prior run crashed mid-flush.
        conn.execute_batch("DELETE FROM records_staging; DELETE FROM workouts_staging;")?;
        Ok(Self {
            conn,
            batch_size: batch_size.max(1),
            record_batch: Vec::with_capacity(batch_size),
            workout_batch: Vec::with_capacity(batch_size),
            records_inserted: 0,
            workouts_inserted: 0,
            records_skipped: 0,
        })
    }

    pub fn write_record(&mut self, record: &Record) -> Result<()> {
        self.record_batch.push(record.clone());
        if self.record_batch.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    pub fn write_workout(&mut self, workout: &Workout) -> Result<()> {
        self.workout_batch.push(workout.clone());
        if self.workout_batch.len() >= self.batch_size {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.record_batch.is_empty() && self.workout_batch.is_empty() {
            return Ok(());
        }

        let record_batch_size = self.record_batch.len() as i64;
        let workout_batch_size = self.workout_batch.len() as i64;

        if !self.record_batch.is_empty() {
            let mut appender = self.conn.appender("records_staging")?;
            for r in &self.record_batch {
                appender.append_row(params![
                    r.record_type,
                    r.value_text,
                    r.value_num,
                    r.unit,
                    r.source_name,
                    r.source_version,
                    r.device,
                    r.creation_date,
                    r.start_date,
                    r.end_date,
                    r.dedupe_key,
                ])?;
            }
            // Appender flushes pending rows on drop.
        }

        if !self.workout_batch.is_empty() {
            let mut appender = self.conn.appender("workouts_staging")?;
            for w in &self.workout_batch {
                appender.append_row(params![
                    w.workout_type,
                    w.duration,
                    w.duration_unit,
                    w.total_distance,
                    w.total_energy_burned,
                    w.source_name,
                    w.creation_date,
                    w.start_date,
                    w.end_date,
                    w.dedupe_key,
                ])?;
            }
        }

        let r_inserted = if record_batch_size > 0 {
            merge_records(self.conn)?
        } else {
            0
        };
        let w_inserted = if workout_batch_size > 0 {
            merge_workouts(self.conn)?
        } else {
            0
        };

        self.records_inserted += r_inserted;
        self.workouts_inserted += w_inserted;
        self.records_skipped +=
            (record_batch_size - r_inserted) + (workout_batch_size - w_inserted);

        self.record_batch.clear();
        self.workout_batch.clear();
        Ok(())
    }

    pub fn records_inserted(&self) -> i64 {
        self.records_inserted
    }

    pub fn workouts_inserted(&self) -> i64 {
        self.workouts_inserted
    }

    pub fn records_skipped(&self) -> i64 {
        self.records_skipped
    }
}

fn merge_records(conn: &mut Connection) -> Result<i64> {
    let before: i64 = conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    conn.execute_batch(
        r#"
        INSERT INTO records (
            record_type, value_text, value_num, unit, source_name,
            source_version, device, creation_date, start_date, end_date, dedupe_key
        )
        SELECT
            record_type, value_text, value_num, unit, source_name,
            source_version, device, creation_date, start_date, end_date, dedupe_key
        FROM (
            SELECT s.*, ROW_NUMBER() OVER (PARTITION BY s.dedupe_key ORDER BY s.start_date) AS rn
            FROM records_staging s
            WHERE NOT EXISTS (SELECT 1 FROM records r WHERE r.dedupe_key = s.dedupe_key)
        ) t
        WHERE t.rn = 1;
        DELETE FROM records_staging;
        "#,
    )?;
    let after: i64 = conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    Ok(after - before)
}

fn merge_workouts(conn: &mut Connection) -> Result<i64> {
    let before: i64 = conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;
    conn.execute_batch(
        r#"
        INSERT INTO workouts (
            workout_type, duration, duration_unit, total_distance,
            total_energy_burned, source_name, creation_date,
            start_date, end_date, dedupe_key
        )
        SELECT
            workout_type, duration, duration_unit, total_distance,
            total_energy_burned, source_name, creation_date,
            start_date, end_date, dedupe_key
        FROM (
            SELECT s.*, ROW_NUMBER() OVER (PARTITION BY s.dedupe_key ORDER BY s.start_date) AS rn
            FROM workouts_staging s
            WHERE NOT EXISTS (SELECT 1 FROM workouts w WHERE w.dedupe_key = s.dedupe_key)
        ) t
        WHERE t.rn = 1;
        DELETE FROM workouts_staging;
        "#,
    )?;
    let after: i64 = conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;
    Ok(after - before)
}
