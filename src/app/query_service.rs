use anyhow::Result;
use serde_json::{Map, Value};

use crate::domain::QueryConfig;
use crate::storage::connection::open_db;
use crate::storage::query::run_select_query;

pub fn run_query(config: QueryConfig) -> Result<Vec<Map<String, Value>>> {
    let conn = open_db(&config.db_path)?;
    run_select_query(&conn, &config.sql, config.limit)
}
