pub mod args;
pub mod ingest;
pub mod inspect;
pub mod query;
pub mod stats;

use anyhow::Result;

use crate::cli::args::{Cli, Commands};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Ingest(args) => ingest::run(args),
        Commands::Inspect(args) => inspect::run(args),
        Commands::Stats(args) => stats::run(args),
        Commands::Query(args) => query::run(args),
    }
}
