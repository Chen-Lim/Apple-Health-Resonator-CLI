use anyhow::{anyhow, Result};

use crate::domain::{ParsedEntity, RawRecord, RawWorkout, Record, Workout};
use crate::infra::hashing::sha256_hex;
use crate::infra::time::parse_apple_health_datetime;

pub fn normalize_record(raw: RawRecord) -> Result<Record> {
    let record_type = required(raw.record_type, "record type")?;
    let start_date = parse_apple_health_datetime(&required(raw.start_date, "record startDate")?)?;
    let end_date = parse_apple_health_datetime(&required(raw.end_date, "record endDate")?)?;
    let creation_date = normalize_optional_datetime(raw.creation_date)?;
    let value_text = raw.value.filter(|value| !value.trim().is_empty());
    let value_num = value_text
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok());
    let unit = raw.unit.filter(|value| !value.trim().is_empty());
    let source_name = raw.source_name.filter(|value| !value.trim().is_empty());
    let source_version = raw.source_version.filter(|value| !value.trim().is_empty());
    let device = raw.device.filter(|value| !value.trim().is_empty());

    let dedupe_key = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}",
        record_type,
        source_name.clone().unwrap_or_default(),
        start_date,
        end_date,
        value_text.clone().unwrap_or_default(),
        unit.clone().unwrap_or_default(),
    ));

    Ok(Record {
        record_type,
        value_text,
        value_num,
        unit,
        source_name,
        source_version,
        device,
        creation_date,
        start_date,
        end_date,
        dedupe_key,
    })
}

pub fn normalize_workout(raw: RawWorkout) -> Result<Workout> {
    let workout_type = required(raw.workout_type, "workout type")?;
    let start_date = parse_apple_health_datetime(&required(raw.start_date, "workout startDate")?)?;
    let end_date = parse_apple_health_datetime(&required(raw.end_date, "workout endDate")?)?;
    let creation_date = normalize_optional_datetime(raw.creation_date)?;
    let duration = raw.duration.and_then(|value| value.parse::<f64>().ok());
    let duration_unit = raw.duration_unit.filter(|value| !value.trim().is_empty());
    let total_distance = raw
        .total_distance
        .and_then(|value| value.parse::<f64>().ok());
    let total_energy_burned = raw
        .total_energy_burned
        .and_then(|value| value.parse::<f64>().ok());
    let source_name = raw.source_name.filter(|value| !value.trim().is_empty());

    let dedupe_key = sha256_hex(&format!(
        "{}|{}|{}|{}|{}|{}",
        workout_type,
        source_name.clone().unwrap_or_default(),
        start_date,
        end_date,
        duration.map(|value| value.to_string()).unwrap_or_default(),
        duration_unit.clone().unwrap_or_default(),
    ));

    Ok(Workout {
        workout_type,
        duration,
        duration_unit,
        total_distance,
        total_energy_burned,
        source_name,
        creation_date,
        start_date,
        end_date,
        dedupe_key,
    })
}

pub fn normalize(entity: crate::parser::extractor::ExtractedRaw) -> Result<ParsedEntity> {
    match entity {
        crate::parser::extractor::ExtractedRaw::Record(raw) => {
            Ok(ParsedEntity::Record(normalize_record(raw)?))
        }
        crate::parser::extractor::ExtractedRaw::Workout(raw) => {
            Ok(ParsedEntity::Workout(normalize_workout(raw)?))
        }
    }
}

fn required(value: Option<String>, label: &str) -> Result<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("missing {label}"))
}

fn normalize_optional_datetime(value: Option<String>) -> Result<Option<String>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| parse_apple_health_datetime(&value))
        .transpose()
}
