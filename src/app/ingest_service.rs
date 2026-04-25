use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use duckdb::params;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::domain::{IngestConfig, IngestRun, ParsedEntity, RawRecord, RawWorkout};
use crate::infra::time::now_utc_rfc3339;
use crate::parser::extractor::{extract, ExtractedRaw};
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
    pub error_log_path: Option<String>,
    pub error_log_warning: Option<String>,
    pub error_log_suppressed: bool,
    pub elapsed_ms: u128,
}

#[derive(Debug)]
struct IngestReadReport {
    records_inserted: i64,
    workouts_inserted: i64,
    records_skipped: i64,
    errors_count: i64,
    error_log_path: Option<String>,
    error_log_warning: Option<String>,
    error_log_suppressed: bool,
}

#[derive(Debug, Serialize)]
struct IngestErrorLogEntry {
    ts: String,
    ingest_run_id: Option<i64>,
    entity_kind: &'static str,
    field_hint: IngestErrorFieldHint,
    error: String,
}

#[derive(Debug, Serialize)]
struct IngestErrorFieldHint {
    record_type: Option<String>,
    source_name: Option<String>,
    start_date_raw: Option<String>,
}

enum ErrorLogState {
    Active(BufWriter<File>),
    Disabled,
    Suppressed,
}

struct IngestErrorLogger {
    state: ErrorLogState,
    path: Option<PathBuf>,
    warning: Option<String>,
    suppressed: bool,
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

    let ingest_result = match input {
        InputSource::Xml(reader) => run_ingest_reader(
            reader,
            &mut conn,
            config.batch_size,
            progress.as_ref(),
            &config.db_path,
            config.error_log_path.as_deref(),
        )?,
        InputSource::Zip {
            mut archive,
            entry_index,
        } => {
            let entry = archive
                .by_index(entry_index)
                .map_err(anyhow::Error::from)
                .context("failed to open matched Apple Health export xml entry from zip")?;
            let reader = BufReader::new(entry);
            run_ingest_reader(
                reader,
                &mut conn,
                config.batch_size,
                progress.as_ref(),
                &config.db_path,
                config.error_log_path.as_deref(),
            )?
        }
    };

    let finished_at = now_utc_rfc3339();
    let run = IngestRun {
        started_at,
        finished_at: Some(finished_at),
        input_path: config.input_path.display().to_string(),
        records_inserted: ingest_result.records_inserted,
        workouts_inserted: ingest_result.workouts_inserted,
        records_skipped: ingest_result.records_skipped,
        errors_count: ingest_result.errors_count,
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
        progress.finish_with_message(format!("errors={}", ingest_result.errors_count));
    }

    Ok(IngestReport {
        records_inserted: ingest_result.records_inserted,
        workouts_inserted: ingest_result.workouts_inserted,
        records_skipped: ingest_result.records_skipped,
        errors_count: ingest_result.errors_count,
        error_log_path: ingest_result.error_log_path,
        error_log_warning: ingest_result.error_log_warning,
        error_log_suppressed: ingest_result.error_log_suppressed,
        elapsed_ms: timer.elapsed().as_millis(),
    })
}

fn run_ingest_reader<R: BufRead>(
    reader: R,
    conn: &mut duckdb::Connection,
    batch_size: usize,
    progress: Option<&ProgressBar>,
    db_path: &Path,
    error_log_path: Option<&Path>,
) -> Result<IngestReadReport> {
    let mut stream = XmlStream::new(reader);
    let mut writer = BatchWriter::new(conn, batch_size)?;
    let mut errors_count = 0_i64;
    let mut processed = 0_u64;
    let mut error_logger = IngestErrorLogger::new(db_path, error_log_path);

    while let Some(entity) = stream.next_entity()? {
        processed += 1;
        match extract(entity) {
            ExtractedRaw::Record(raw) => match normalize(ExtractedRaw::Record(raw.clone())) {
                Ok(ParsedEntity::Record(record)) => writer.write_record(&record)?,
                Ok(ParsedEntity::Workout(_)) => unreachable!(),
                Err(error) => {
                    errors_count += 1;
                    tracing::warn!(%error, "record skipped during ingest");
                    error_logger.log_record_error(&raw, &error.to_string());
                }
            },
            ExtractedRaw::Workout(raw) => match normalize(ExtractedRaw::Workout(raw.clone())) {
                Ok(ParsedEntity::Workout(workout)) => writer.write_workout(&workout)?,
                Ok(ParsedEntity::Record(_)) => unreachable!(),
                Err(error) => {
                    errors_count += 1;
                    tracing::warn!(%error, "workout skipped during ingest");
                    error_logger.log_workout_error(&raw, &error.to_string());
                }
            },
        }

        if let Some(progress) = progress {
            progress.set_position(processed);
            progress.set_message(errors_count.to_string());
        }
    }

    writer.flush()?;
    error_logger.finish();

    Ok(IngestReadReport {
        records_inserted: writer.records_inserted(),
        workouts_inserted: writer.workouts_inserted(),
        records_skipped: writer.records_skipped(),
        errors_count,
        error_log_path: error_logger.path_string(),
        error_log_warning: error_logger.warning,
        error_log_suppressed: error_logger.suppressed,
    })
}

impl IngestErrorLogger {
    fn new(db_path: &Path, requested_path: Option<&Path>) -> Self {
        #[cfg(unix)]
        if requested_path == Some(Path::new("/dev/null")) {
            return Self {
                state: ErrorLogState::Suppressed,
                path: None,
                warning: None,
                suppressed: true,
            };
        }

        let path = resolve_error_log_path(db_path, requested_path);
        match open_error_log_file(&path) {
            Ok(file) => Self {
                state: ErrorLogState::Active(BufWriter::new(file)),
                path: Some(path),
                warning: None,
                suppressed: false,
            },
            Err(error) => Self {
                state: ErrorLogState::Disabled,
                path: None,
                warning: Some(format!(
                    "failed to initialize ingest error log at {}: {error}",
                    path.display()
                )),
                suppressed: false,
            },
        }
    }

    fn log_record_error(&mut self, raw: &RawRecord, error: &str) {
        let entry = IngestErrorLogEntry {
            ts: now_utc_rfc3339(),
            ingest_run_id: None,
            entity_kind: "record",
            field_hint: IngestErrorFieldHint {
                record_type: raw.record_type.clone(),
                source_name: raw.source_name.clone(),
                start_date_raw: raw.start_date.clone(),
            },
            error: error.to_string(),
        };
        self.write_entry(entry);
    }

    fn log_workout_error(&mut self, raw: &RawWorkout, error: &str) {
        let entry = IngestErrorLogEntry {
            ts: now_utc_rfc3339(),
            ingest_run_id: None,
            entity_kind: "workout",
            field_hint: IngestErrorFieldHint {
                record_type: raw.workout_type.clone(),
                source_name: raw.source_name.clone(),
                start_date_raw: raw.start_date.clone(),
            },
            error: error.to_string(),
        };
        self.write_entry(entry);
    }

    fn write_entry(&mut self, entry: IngestErrorLogEntry) {
        if let ErrorLogState::Active(writer) = &mut self.state {
            let result = serde_json::to_writer(&mut *writer, &entry)
                .map_err(anyhow::Error::from)
                .and_then(|_| writer.write_all(b"\n").map_err(anyhow::Error::from));
            if let Err(error) = result {
                self.warning = Some(self.write_failure_message(&error));
                self.path = None;
                self.state = ErrorLogState::Disabled;
            }
        }
    }

    fn finish(&mut self) {
        if let ErrorLogState::Active(writer) = &mut self.state {
            if let Err(error) = writer.flush() {
                self.warning = Some(self.flush_failure_message(&error));
                self.path = None;
                self.state = ErrorLogState::Disabled;
            }
        }
    }

    fn path_string(&self) -> Option<String> {
        self.path.as_ref().map(|path| path.display().to_string())
    }

    fn write_failure_message(&self, error: &anyhow::Error) -> String {
        if let Some(path) = &self.path {
            format!(
                "failed to write ingest error log at {}: {error}",
                path.display()
            )
        } else {
            format!("failed to write ingest error log: {error}")
        }
    }

    fn flush_failure_message(&self, error: &std::io::Error) -> String {
        if let Some(path) = &self.path {
            format!(
                "failed to flush ingest error log at {}: {error}",
                path.display()
            )
        } else {
            format!("failed to flush ingest error log: {error}")
        }
    }
}

fn resolve_error_log_path(db_path: &Path, requested_path: Option<&Path>) -> PathBuf {
    if let Some(path) = requested_path {
        return path.to_path_buf();
    }

    let dir = db_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = db_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("{stem}.ingest-errors.jsonl"))
        .unwrap_or_else(|| "ingest-errors.jsonl".to_string());
    dir.join(file_name)
}

fn open_error_log_file(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open ingest error log {}", path.display()))
}
