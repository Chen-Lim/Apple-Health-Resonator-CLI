use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::app::ingest_service::run_ingest;
use crate::domain::IngestConfig;

#[derive(Debug, Args)]
pub struct IngestArgs {
    #[arg(help = "Path to Apple Health export.xml or export.zip")]
    pub path: PathBuf,
    #[arg(long, default_value = "./health_data.db", help = "SQLite database path to create or update")]
    pub db: PathBuf,
    #[arg(long, default_value_t = 10_000, help = "Number of rows to write per batch")]
    pub batch_size: usize,
    #[arg(long, default_value_t = false, help = "Disable progress output")]
    pub quiet: bool,
}

impl From<IngestArgs> for IngestConfig {
    fn from(value: IngestArgs) -> Self {
        Self {
            input_path: value.path,
            db_path: value.db,
            batch_size: value.batch_size,
            quiet: value.quiet,
        }
    }
}

pub fn run(args: IngestArgs) -> Result<()> {
    let report = run_ingest(args.into())?;
    println!(
        "Records: {} | Workouts: {} | Skipped: {} | Errors: {} | Elapsed(ms): {}",
        report.records_inserted,
        report.workouts_inserted,
        report.records_skipped,
        report.errors_count,
        report.elapsed_ms
    );
    Ok(())
}
