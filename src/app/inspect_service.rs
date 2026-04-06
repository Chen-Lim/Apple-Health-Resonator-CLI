use anyhow::Result;

use crate::domain::{InspectConfig, InspectSummary};
use crate::storage::connection::open_db;
use crate::storage::query::fetch_inspect_summary;

pub fn run_inspect(config: InspectConfig) -> Result<InspectSummary> {
    let conn = open_db(&config.db_path)?;
    fetch_inspect_summary(&conn)
}
