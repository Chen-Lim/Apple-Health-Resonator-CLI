use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

const APPLE_HEALTH_FORMAT: &str = "%Y-%m-%d %H:%M:%S %z";

pub fn parse_apple_health_datetime(input: &str) -> Result<String> {
    let dt = DateTime::parse_from_str(input, APPLE_HEALTH_FORMAT)
        .with_context(|| format!("invalid Apple Health datetime: {input}"))?;
    Ok(dt
        .with_timezone(&Utc)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string())
}

pub fn now_utc_rfc3339() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
