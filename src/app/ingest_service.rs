use std::io::{BufRead, BufReader};
use std::time::Instant;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rusqlite::params;

use crate::domain::{IngestConfig, IngestRun, ParsedEntity};
use crate::infra::time::now_utc_rfc3339;
use crate::parser::extractor::extract;
use crate::parser::input::{open_input, InputSource};
use crate::parser::normalizer::normalize;
use crate::parser::xml_reader::XmlStream;
use crate::storage::connection::open_db;
use crate::storage::schema::{init_schema, schema_version};
use crate::storage::writer::BatchWriter;

#[derive(Debug, Clone)]
pub struct IngestReport {
    pub records_inserted: i64,
    pub workouts_inserted: i64,
    pub records_skipped: i64,
    pub errors_count: i64,
    pub elapsed_ms: u128,
}

pub fn run_ingest(config: IngestConfig) -> Result<IngestReport> {
    let started_at = now_utc_rfc3339();
    let timer = Instant::now();
    let input = open_input(&config.input_path)?;
    let mut conn = open_db(&config.db_path)?;
    init_schema(&conn)?;

    let progress = if config.quiet {
        None
    } else {
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::with_template("{spinner} processed={pos} errors={msg}")?
                .tick_chars("/|\\- "),
        );
        progress.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(progress)
    };

    let (records_inserted, workouts_inserted, records_skipped, errors_count) = match input {
        InputSource::Xml(reader) => {
            run_ingest_reader(reader, &mut conn, config.batch_size, progress.as_ref())?
        }
        InputSource::Zip {
            mut archive,
            entry_index,
        } => {
            let entry = archive
                .by_index(entry_index)
                .map_err(anyhow::Error::from)
                .context("failed to open export.xml entry from zip")?;
            let reader = BufReader::new(entry);
            run_ingest_reader(reader, &mut conn, config.batch_size, progress.as_ref())?
        }
    };

    let finished_at = now_utc_rfc3339();
    let run = IngestRun {
        started_at,
        finished_at: Some(finished_at),
        input_path: config.input_path.display().to_string(),
        records_inserted,
        workouts_inserted,
        records_skipped,
        errors_count,
        schema_version: schema_version().to_string(),
    };
    conn.execute(
        r#"
        INSERT INTO ingest_runs (
            started_at, finished_at, input_path, records_inserted,
            workouts_inserted, records_skipped, errors_count, schema_version
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            run.started_at,
            run.finished_at,
            run.input_path,
            run.records_inserted,
            run.workouts_inserted,
            run.records_skipped,
            run.errors_count,
            run.schema_version,
        ],
    )?;

    if let Some(progress) = progress {
        progress.finish_with_message(format!("errors={errors_count}"));
    }

    Ok(IngestReport {
        records_inserted,
        workouts_inserted,
        records_skipped,
        errors_count,
        elapsed_ms: timer.elapsed().as_millis(),
    })
}

fn run_ingest_reader<R: BufRead>(
    reader: R,
    conn: &mut rusqlite::Connection,
    batch_size: usize,
    progress: Option<&ProgressBar>,
) -> Result<(i64, i64, i64, i64)> {
    let mut stream = XmlStream::new(reader);
    let mut writer = BatchWriter::new(conn, batch_size)?;
    let mut errors_count = 0_i64;
    let mut processed = 0_u64;

    while let Some(entity) = stream.next_entity()? {
        processed += 1;
        match normalize(extract(entity)) {
            Ok(ParsedEntity::Record(record)) => writer.write_record(&record)?,
            Ok(ParsedEntity::Workout(workout)) => writer.write_workout(&workout)?,
            Err(error) => {
                errors_count += 1;
                tracing::warn!(%error, "entity skipped during ingest");
            }
        }

        if let Some(progress) = progress {
            progress.set_position(processed);
            progress.set_message(errors_count.to_string());
        }
    }

    writer.flush()?;
    Ok((
        writer.records_inserted(),
        writer.workouts_inserted(),
        writer.records_skipped(),
        errors_count,
    ))
}
