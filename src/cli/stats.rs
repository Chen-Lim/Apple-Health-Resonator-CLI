use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::app::stats_service::run_stats;
use crate::domain::StatsConfig;
use crate::output::json::to_compact_json;

#[derive(Debug, Args)]
pub struct StatsArgs {
    #[arg(long)]
    pub db: PathBuf,
}

impl From<StatsArgs> for StatsConfig {
    fn from(value: StatsArgs) -> Self {
        Self { db_path: value.db }
    }
}

pub fn run(args: StatsArgs) -> Result<()> {
    let summary = run_stats(args.into())?;
    println!("{}", to_compact_json(&summary)?);
    Ok(())
}
