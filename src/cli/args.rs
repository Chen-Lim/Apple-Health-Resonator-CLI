use clap::{Parser, Subcommand};

use crate::cli::{ingest::IngestArgs, inspect::InspectArgs, query::QueryArgs, stats::StatsArgs};

#[derive(Debug, Parser)]
#[command(name = "ahr")]
#[command(version)]
#[command(
    about = "Import Apple Health exports into SQLite and expose inspect, stats, and query commands for agents",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Ingest an Apple Health export.xml or export.zip into SQLite")]
    Ingest(IngestArgs),
    #[command(about = "Inspect database coverage and schema-level summary as pretty JSON")]
    Inspect(InspectArgs),
    #[command(about = "Return compact JSON summary statistics for agent consumption")]
    Stats(StatsArgs),
    #[command(about = "Run a validated read-only SQL query and return compact JSON rows")]
    Query(QueryArgs),
}
