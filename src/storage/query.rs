use std::collections::BTreeMap;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use rusqlite::{types::ValueRef, Connection};
use serde_json::{Map, Value};

use crate::domain::{DateRange, InspectSummary, SourceCount, StatsSummary, TypeCount};

pub fn fetch_inspect_summary(conn: &Connection) -> Result<InspectSummary> {
    let tables = {
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let record_count = conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    let workout_count = conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;
    let date_range = conn.query_row(
        r#"
        SELECT MIN(start_date), MAX(end_date)
        FROM (
            SELECT start_date, end_date FROM records
            UNION ALL
            SELECT start_date, end_date FROM workouts
        )
        "#,
        [],
        |row| {
            Ok(DateRange {
                start: row.get(0)?,
                end: row.get(1)?,
            })
        },
    )?;

    let sources = {
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT source_name
            FROM (
                SELECT source_name FROM records
                UNION
                SELECT source_name FROM workouts
            )
            WHERE source_name IS NOT NULL AND source_name != ''
            ORDER BY source_name
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let record_types = {
        let mut stmt =
            conn.prepare("SELECT DISTINCT record_type FROM records ORDER BY record_type")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    Ok(InspectSummary {
        tables,
        record_count,
        workout_count,
        date_range,
        sources,
        record_types,
    })
}

pub fn fetch_stats_summary(conn: &Connection) -> Result<StatsSummary> {
    let total_records = conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    let total_workouts = conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;

    let top_types = {
        let mut stmt = conn.prepare(
            "SELECT record_type, COUNT(*) AS count FROM records GROUP BY record_type ORDER BY count DESC, record_type ASC LIMIT 10",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TypeCount {
                record_type: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let top_sources = {
        let mut stmt = conn.prepare(
            r#"
            SELECT source_name, COUNT(*) AS count
            FROM (
                SELECT source_name FROM records
                UNION ALL
                SELECT source_name FROM workouts
            )
            WHERE source_name IS NOT NULL AND source_name != ''
            GROUP BY source_name
            ORDER BY count DESC, source_name ASC
            LIMIT 10
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SourceCount {
                source_name: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let recent_threshold = (Utc::now() - Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let recent_activity = conn.query_row(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM records WHERE start_date >= ?1
            UNION ALL
            SELECT 1 FROM workouts WHERE start_date >= ?1
        )
        "#,
        [recent_threshold],
        |row| row.get::<_, bool>(0),
    )?;

    Ok(StatsSummary {
        total_records,
        total_workouts,
        top_types,
        top_sources,
        recent_activity,
    })
}

pub fn run_select_query(
    conn: &Connection,
    sql: &str,
    limit: usize,
) -> Result<Vec<Map<String, Value>>> {
    validate_select_sql(conn, sql)?;
    let mut stmt = conn.prepare(sql)?;
    let column_names = stmt
        .column_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect::<Vec<_>>();

    let mut rows = stmt.query([])?;
    let mut results = Vec::new();
    while let Some(row) = rows.next()? {
        if results.len() >= limit {
            break;
        }
        let mut object = BTreeMap::new();
        for (idx, name) in column_names.iter().enumerate() {
            let value = value_ref_to_json(row.get_ref(idx)?);
            object.insert(name.clone(), value);
        }
        results.push(object.into_iter().collect());
    }
    Ok(results)
}

fn value_ref_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::from(value),
        ValueRef::Real(value) => Value::from(value),
        ValueRef::Text(value) => Value::from(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::from(hex::encode(value)),
    }
}

fn validate_select_sql(conn: &Connection, sql: &str) -> Result<()> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        bail!("query must not be empty");
    }

    let c_sql = CString::new(trimmed)?;
    let mut stmt = std::ptr::null_mut();
    let mut tail = std::ptr::null();
    let rc = unsafe {
        rusqlite::ffi::sqlite3_prepare_v3(
            conn.handle(),
            c_sql.as_ptr(),
            -1,
            0,
            &mut stmt,
            &mut tail,
        )
    };
    if rc != rusqlite::ffi::SQLITE_OK {
        let message = unsafe { CStr::from_ptr(rusqlite::ffi::sqlite3_errmsg(conn.handle())) }
            .to_string_lossy()
            .into_owned();
        bail!("invalid SQL: {message}");
    }
    if stmt.is_null() {
        bail!("invalid SQL statement");
    }

    let readonly = unsafe { rusqlite::ffi::sqlite3_stmt_readonly(stmt) != 0 };
    let tail_sql = unsafe { CStr::from_ptr(tail) }
        .to_string_lossy()
        .trim()
        .to_string();
    let _ = unsafe { rusqlite::ffi::sqlite3_finalize(stmt) };

    if !tail_sql.is_empty() {
        bail!("multiple SQL statements are not allowed");
    }
    if !readonly {
        bail!("only read-only SELECT statements are allowed");
    }

    let first_token = leading_keyword(trimmed)
        .ok_or_else(|| anyhow!("failed to determine SQL statement type"))?;
    if first_token != "select" && first_token != "with" {
        bail!("only SELECT statements are allowed");
    }
    if first_token == "with" && !trimmed.to_ascii_lowercase().contains("select") {
        bail!("WITH statements must resolve to a SELECT");
    }
    Ok(())
}

fn leading_keyword(sql: &str) -> Option<String> {
    sql.split_whitespace()
        .next()
        .map(|token| {
            token
                .trim_start_matches(|c: char| c == '(')
                .trim_end_matches(';')
                .trim()
        })
        .map(str::to_ascii_lowercase)
}
