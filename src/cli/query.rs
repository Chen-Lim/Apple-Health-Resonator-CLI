use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::app::query_service::run_query;
use crate::domain::QueryConfig;
use crate::output::json::to_compact_json;

#[derive(Debug, Args)]
pub struct QueryArgs {
    #[arg(long)]
    pub db: PathBuf,
    #[arg(long)]
    pub sql: String,
    #[arg(long, default_value_t = 1000)]
    pub limit: usize,
}

impl From<QueryArgs> for QueryConfig {
    fn from(value: QueryArgs) -> Self {
        Self {
            db_path: value.db,
            sql: value.sql,
            limit: value.limit,
        }
    }
}

pub fn run(args: QueryArgs) -> Result<()> {
    let rows = run_query(args.into())?;
    println!("{}", to_compact_json(&rows)?);
    Ok(())
}
