use std::path::Path;

use anyhow::Result;
use duckdb::Connection;

pub fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    Ok(conn)
}
