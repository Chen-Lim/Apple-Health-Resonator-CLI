use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};
use duckdb::types::ValueRef;
use duckdb::Connection;
use serde_json::{Map, Value};

use crate::domain::{DateRange, InspectSummary, SourceCount, StatsSummary, TypeCount};

pub fn fetch_inspect_summary(conn: &Connection) -> Result<InspectSummary> {
    let tables = {
        let mut stmt = conn.prepare(
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = 'main' AND table_name NOT IN ('records_staging', 'workouts_staging') \
             ORDER BY table_name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<duckdb::Result<Vec<_>>>()?
    };

    let record_count: i64 = conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    let workout_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;
    let date_range = conn.query_row(
        r#"
        SELECT MIN(start_date), MAX(end_date)
        FROM (
            SELECT start_date, end_date FROM records
            UNION ALL
            SELECT start_date, end_date FROM workouts
        ) AS combined
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
            ) AS combined
            WHERE source_name IS NOT NULL AND source_name != ''
            ORDER BY source_name
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<duckdb::Result<Vec<_>>>()?
    };

    let record_types = {
        let mut stmt =
            conn.prepare("SELECT DISTINCT record_type FROM records ORDER BY record_type")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<duckdb::Result<Vec<_>>>()?
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
    let total_records: i64 =
        conn.query_row("SELECT COUNT(*) FROM records", [], |row| row.get(0))?;
    let total_workouts: i64 =
        conn.query_row("SELECT COUNT(*) FROM workouts", [], |row| row.get(0))?;

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
        rows.collect::<duckdb::Result<Vec<_>>>()?
    };

    let top_sources = {
        let mut stmt = conn.prepare(
            r#"
            SELECT source_name, COUNT(*) AS count
            FROM (
                SELECT source_name FROM records
                UNION ALL
                SELECT source_name FROM workouts
            ) AS combined
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
        rows.collect::<duckdb::Result<Vec<_>>>()?
    };

    let recent_threshold = (Utc::now() - Duration::days(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let recent_activity: bool = conn.query_row(
        r#"
        SELECT
            EXISTS(SELECT 1 FROM records WHERE start_date >= ?)
         OR EXISTS(SELECT 1 FROM workouts WHERE start_date >= ?)
        "#,
        [&recent_threshold, &recent_threshold],
        |row| row.get(0),
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
    validate_select_sql(sql)?;
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;

    // DuckDB only resolves the result schema once the query has executed, so we
    // pull column names through `Rows::as_ref()` after `query()` rather than
    // off the prepared statement.
    let column_names: Vec<String> = match rows.as_ref() {
        Some(s) => s
            .column_names()
            .into_iter()
            .map(|name| name.to_string())
            .collect(),
        None => Vec::new(),
    };

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
        ValueRef::Boolean(v) => Value::from(v),
        ValueRef::TinyInt(v) => Value::from(v as i64),
        ValueRef::SmallInt(v) => Value::from(v as i64),
        ValueRef::Int(v) => Value::from(v as i64),
        ValueRef::BigInt(v) => Value::from(v),
        ValueRef::HugeInt(v) => Value::from(v.to_string()),
        ValueRef::UTinyInt(v) => Value::from(v as u64),
        ValueRef::USmallInt(v) => Value::from(v as u64),
        ValueRef::UInt(v) => Value::from(v as u64),
        ValueRef::UBigInt(v) => Value::from(v),
        ValueRef::Float(v) => Value::from(v as f64),
        ValueRef::Double(v) => Value::from(v),
        ValueRef::Text(bytes) => Value::from(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Value::from(hex::encode(bytes)),
        other => Value::from(format!("{:?}", other)),
    }
}

/// Tokens that may not appear anywhere in user-supplied SQL.
///
/// DuckDB has no equivalent of SQLite's `sqlite3_stmt_readonly`, so we enforce
/// read-only intent by combining a leading-keyword check with a token-level
/// blacklist over the SQL with string literals and comments stripped.
const FORBIDDEN_TOKENS: &[&str] = &[
    "INSERT",
    "UPDATE",
    "DELETE",
    "MERGE",
    "UPSERT",
    "REPLACE",
    "TRUNCATE",
    "CREATE",
    "DROP",
    "ALTER",
    "RENAME",
    "VACUUM",
    "ANALYZE",
    "REINDEX",
    "ATTACH",
    "DETACH",
    "COPY",
    "EXPORT",
    "IMPORT",
    "INSTALL",
    "LOAD",
    "PRAGMA",
    "SET",
    "RESET",
    "CALL",
    "USE",
    "GRANT",
    "REVOKE",
    "CHECKPOINT",
];

fn validate_select_sql(sql: &str) -> Result<()> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        bail!("query must not be empty");
    }

    let stripped = strip_strings_and_comments(trimmed)?;

    if stripped.contains(';') {
        bail!("multiple SQL statements are not allowed");
    }

    let first_token = leading_keyword(&stripped)
        .ok_or_else(|| anyhow!("failed to determine SQL statement type"))?;
    if first_token != "select" && first_token != "with" {
        bail!("only SELECT statements are allowed");
    }

    let upper = stripped.to_ascii_uppercase();
    for forbidden in FORBIDDEN_TOKENS {
        if contains_word(&upper, forbidden) {
            bail!("forbidden keyword in query: {forbidden}");
        }
    }

    if first_token == "with" && !contains_word(&upper, "SELECT") {
        bail!("WITH statements must resolve to a SELECT");
    }

    Ok(())
}

/// Replace string literals with empty strings and remove SQL comments so that
/// keyword scanning does not match content inside quotes or comments.
fn strip_strings_and_comments(sql: &str) -> Result<String> {
    let mut out = String::with_capacity(sql.len());
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        // Line comment
        if c == '-' && i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
            }
            continue;
        }
        // Block comment
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] as char == '*' && bytes[i + 1] as char == '/') {
                i += 1;
            }
            if i + 1 >= bytes.len() {
                bail!("unterminated block comment");
            }
            i += 2;
            continue;
        }
        // Single-quoted string (with '' escape)
        if c == '\'' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] as char == '\'' {
                    if i + 1 < bytes.len() && bytes[i + 1] as char == '\'' {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str("''");
            continue;
        }
        // Double-quoted identifier (with "" escape)
        if c == '"' {
            out.push('"');
            i += 1;
            while i < bytes.len() {
                if bytes[i] as char == '"' {
                    if i + 1 < bytes.len() && bytes[i + 1] as char == '"' {
                        out.push_str("\"\"");
                        i += 2;
                        continue;
                    }
                    out.push('"');
                    i += 1;
                    break;
                }
                out.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    Ok(out)
}

fn contains_word(haystack_upper: &str, word_upper: &str) -> bool {
    let bytes = haystack_upper.as_bytes();
    let needle = word_upper.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    let is_word_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_idx = i + needle.len();
            let after_ok = after_idx == bytes.len() || !is_word_char(bytes[after_idx]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn leading_keyword(sql: &str) -> Option<String> {
    sql.split_whitespace()
        .next()
        .map(|token| token.trim_start_matches('(').trim_end_matches(';').trim())
        .map(str::to_ascii_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_select() {
        validate_select_sql("SELECT * FROM records").unwrap();
        validate_select_sql("  select 1  ").unwrap();
        validate_select_sql("WITH t AS (SELECT 1) SELECT * FROM t").unwrap();
    }

    #[test]
    fn rejects_writes() {
        assert!(validate_select_sql("INSERT INTO records VALUES (1)").is_err());
        assert!(validate_select_sql("DELETE FROM records").is_err());
        assert!(validate_select_sql("UPDATE records SET unit = 'x'").is_err());
        assert!(validate_select_sql("DROP TABLE records").is_err());
    }

    #[test]
    fn rejects_duckdb_sandbox_escape() {
        assert!(validate_select_sql("COPY records TO 'out.csv'").is_err());
        assert!(validate_select_sql("ATTACH 'evil.db'").is_err());
        assert!(validate_select_sql("PRAGMA threads=1").is_err());
        assert!(validate_select_sql("INSTALL httpfs").is_err());
        assert!(validate_select_sql("LOAD httpfs").is_err());
    }

    #[test]
    fn rejects_multi_statement() {
        assert!(validate_select_sql("SELECT 1; DELETE FROM records").is_err());
    }

    #[test]
    fn rejects_keywords_hidden_in_comments_safely() {
        // Comment-stripped SELECT with a forbidden keyword in a real position should still fail.
        assert!(validate_select_sql("/* comment */ DROP TABLE records").is_err());
        // But a forbidden keyword purely inside a comment must not falsely trigger.
        validate_select_sql("SELECT 1 -- INSERT not real").unwrap();
        validate_select_sql("SELECT 1 /* DELETE comment */").unwrap();
    }

    #[test]
    fn allows_forbidden_word_inside_string_literal() {
        validate_select_sql("SELECT 'INSERT into thing' AS s").unwrap();
    }
}
