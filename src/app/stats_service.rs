use anyhow::Result;

use crate::domain::{StatsConfig, StatsSummary};
use crate::storage::connection::open_db;
use crate::storage::query::fetch_stats_summary;

pub fn run_stats(config: StatsConfig) -> Result<StatsSummary> {
    let conn = open_db(&config.db_path)?;
    fetch_stats_summary(&conn)
}
