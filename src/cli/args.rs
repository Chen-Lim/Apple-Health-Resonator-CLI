use clap::{Parser, Subcommand};

use crate::cli::{ingest::IngestArgs, inspect::InspectArgs, query::QueryArgs, stats::StatsArgs};

#[derive(Debug, Parser)]
#[command(name = "ahr")]
#[command(about = "Apple Health Resonator CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Ingest(IngestArgs),
    Inspect(InspectArgs),
    Stats(StatsArgs),
    Query(QueryArgs),
}
