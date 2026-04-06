use anyhow::Result;
use rusqlite::Connection;

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

        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR IGNORE INTO records (
                    record_type, value_text, value_num, unit, source_name,
                    source_version, device, creation_date, start_date, end_date, dedupe_key
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
            )?;
            for record in &self.record_batch {
                let changed = stmt.execute(rusqlite::params![
                    record.record_type,
                    record.value_text,
                    record.value_num,
                    record.unit,
                    record.source_name,
                    record.source_version,
                    record.device,
                    record.creation_date,
                    record.start_date,
                    record.end_date,
                    record.dedupe_key,
                ])?;
                if changed == 0 {
                    self.records_skipped += 1;
                } else {
                    self.records_inserted += changed as i64;
                }
            }
        }
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT OR IGNORE INTO workouts (
                    workout_type, duration, duration_unit, total_distance,
                    total_energy_burned, source_name, creation_date,
                    start_date, end_date, dedupe_key
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )?;
            for workout in &self.workout_batch {
                let changed = stmt.execute(rusqlite::params![
                    workout.workout_type,
                    workout.duration,
                    workout.duration_unit,
                    workout.total_distance,
                    workout.total_energy_burned,
                    workout.source_name,
                    workout.creation_date,
                    workout.start_date,
                    workout.end_date,
                    workout.dedupe_key,
                ])?;
                if changed == 0 {
                    self.records_skipped += 1;
                } else {
                    self.workouts_inserted += changed as i64;
                }
            }
        }
        tx.commit()?;
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
