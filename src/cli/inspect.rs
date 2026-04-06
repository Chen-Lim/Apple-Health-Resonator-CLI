use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::app::inspect_service::run_inspect;
use crate::domain::InspectConfig;
use crate::output::json::to_pretty_json;

#[derive(Debug, Args)]
pub struct InspectArgs {
    #[arg(long)]
    pub db: PathBuf,
}

impl From<InspectArgs> for InspectConfig {
    fn from(value: InspectArgs) -> Self {
        Self { db_path: value.db }
    }
}

pub fn run(args: InspectArgs) -> Result<()> {
    let summary = run_inspect(args.into())?;
    println!("{}", to_pretty_json(&summary)?);
    Ok(())
}
