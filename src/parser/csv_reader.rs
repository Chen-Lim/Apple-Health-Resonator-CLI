use std::io::{BufRead, Read};

use anyhow::{anyhow, Context, Result};
use csv::{ReaderBuilder, StringRecord};

use crate::parser::xml_reader::XmlEntity;

/// Streams rows from a single SimpleHealthExportCSV file and adapts them to
/// `XmlEntity` so the rest of the ingest pipeline (extractor → normalizer →
/// writer) is reused unchanged.
///
/// `type_id` is the HealthKit identifier carried by the file name, e.g.
/// `HKQuantityTypeIdentifierHeartRate` or `HKWorkoutActivityTypeSwimming`.
pub struct CsvStream {
    inner: csv::Reader<Box<dyn Read>>,
    headers: Vec<String>,
    record_buf: StringRecord,
    kind: CsvKind,
}

enum CsvKind {
    Record { type_id: String },
    Workout { activity_type: String },
}

pub fn open_csv_stream<R: BufRead + 'static>(mut reader: R, type_id: &str) -> Result<CsvStream> {
    // SimpleHealthExportCSV prefixes every file with `sep=,` (Excel hint). Skip
    // it; otherwise treat the line as the header row.
    let mut first_line = String::new();
    let bytes = reader
        .read_line(&mut first_line)
        .context("failed to read first line of csv")?;
    if bytes == 0 {
        return Err(anyhow!("empty csv file"));
    }
    let trimmed = first_line.trim_end_matches(['\r', '\n']);
    let body: Box<dyn Read> = if trimmed.eq_ignore_ascii_case("sep=,") {
        Box::new(reader)
    } else {
        let cursor = std::io::Cursor::new(first_line.into_bytes());
        Box::new(cursor.chain(reader))
    };

    let mut csv = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(body);

    let headers = csv
        .headers()
        .context("failed to read csv header row")?
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let kind = if let Some(activity) = type_id.strip_prefix("HKWorkoutActivityType") {
        CsvKind::Workout {
            activity_type: activity.to_string(),
        }
    } else {
        CsvKind::Record {
            type_id: type_id.to_string(),
        }
    };

    Ok(CsvStream {
        inner: csv,
        headers,
        record_buf: StringRecord::new(),
        kind,
    })
}

impl CsvStream {
    pub fn next_entity(&mut self) -> Result<Option<XmlEntity>> {
        let has = self
            .inner
            .read_record(&mut self.record_buf)
            .context("failed to read csv row")?;
        if !has {
            return Ok(None);
        }
        Ok(Some(match &self.kind {
            CsvKind::Record { type_id } => {
                XmlEntity::Record(record_attrs(type_id, &self.headers, &self.record_buf))
            }
            CsvKind::Workout { activity_type } => XmlEntity::Workout(workout_attrs(
                activity_type,
                &self.headers,
                &self.record_buf,
            )),
        }))
    }
}

fn record_attrs(type_id: &str, headers: &[String], row: &StringRecord) -> Vec<(String, String)> {
    let mut attrs = Vec::with_capacity(headers.len() + 1);
    attrs.push(("type".to_string(), type_id.to_string()));
    for (idx, header) in headers.iter().enumerate() {
        let Some(raw) = row.get(idx) else { continue };
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        match header.as_str() {
            "type" => continue, // file name is authoritative
            "value" | "unit" | "sourceName" | "sourceVersion" | "device" | "creationDate"
            | "startDate" | "endDate" => {
                attrs.push((header.clone(), value.to_string()));
            }
            _ => {}
        }
    }
    attrs
}

fn workout_attrs(
    activity_type: &str,
    headers: &[String],
    row: &StringRecord,
) -> Vec<(String, String)> {
    let mut attrs = Vec::with_capacity(headers.len() + 1);
    attrs.push((
        "workoutActivityType".to_string(),
        format!("HKWorkoutActivityType{activity_type}"),
    ));
    for (idx, header) in headers.iter().enumerate() {
        let Some(raw) = row.get(idx) else { continue };
        let value = raw.trim();
        if value.is_empty() {
            continue;
        }
        match header.as_str() {
            "duration" | "durationUnit" | "sourceName" | "creationDate" | "startDate"
            | "endDate" => {
                attrs.push((header.clone(), value.to_string()));
            }
            // CSV embeds units inside the value (e.g. "96.8736 kcal", "800 m");
            // strip the unit so normalizer sees a parseable number.
            "totalEnergyBurned" | "totalDistance" => {
                let numeric = value.split_whitespace().next().unwrap_or(value);
                attrs.push((header.clone(), numeric.to_string()));
            }
            _ => {}
        }
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    fn open(bytes: &[u8], type_id: &str) -> CsvStream {
        open_csv_stream(BufReader::new(Cursor::new(bytes.to_vec())), type_id).unwrap()
    }

    #[test]
    fn parses_quantity_record() {
        let csv = b"sep=,\ntype,sourceName,sourceVersion,productType,device,startDate,endDate,unit,value\nHKQuantityTypeIdentifierHeartRate,Watch,26.3,Watch7,dev,2026-03-25 08:42:57 +0000,2026-03-25 08:42:57 +0000,count/min,92.0\n";
        let mut s = open(csv, "HKQuantityTypeIdentifierHeartRate");
        let e = s.next_entity().unwrap().expect("row");
        match e {
            XmlEntity::Record(attrs) => {
                let m: std::collections::HashMap<_, _> = attrs.into_iter().collect();
                assert_eq!(m.get("type").unwrap(), "HKQuantityTypeIdentifierHeartRate");
                assert_eq!(m.get("value").unwrap(), "92.0");
                assert_eq!(m.get("unit").unwrap(), "count/min");
                assert_eq!(m.get("startDate").unwrap(), "2026-03-25 08:42:57 +0000");
            }
            _ => panic!("expected record"),
        }
        assert!(s.next_entity().unwrap().is_none());
    }

    #[test]
    fn parses_workout_with_units_in_values() {
        let csv = b"sep=,\ntype,sourceName,sourceVersion,productType,device,startDate,endDate,activityType,duration,durationUnit,totalEnergyBurned,totalDistance\nHKWorkoutTypeIdentifier,Watch,26.4,Watch7,dev,2026-04-02 05:36:25 +0000,2026-04-02 06:02:54 +0000,Swimming,1589.16,sec,96.8736 kcal,800 m\n";
        let mut s = open(csv, "HKWorkoutActivityTypeSwimming");
        let e = s.next_entity().unwrap().expect("row");
        match e {
            XmlEntity::Workout(attrs) => {
                let m: std::collections::HashMap<_, _> = attrs.into_iter().collect();
                assert_eq!(
                    m.get("workoutActivityType").unwrap(),
                    "HKWorkoutActivityTypeSwimming"
                );
                assert_eq!(m.get("duration").unwrap(), "1589.16");
                assert_eq!(m.get("durationUnit").unwrap(), "sec");
                assert_eq!(m.get("totalEnergyBurned").unwrap(), "96.8736");
                assert_eq!(m.get("totalDistance").unwrap(), "800");
            }
            _ => panic!("expected workout"),
        }
    }

    #[test]
    fn parses_category_value_text() {
        let csv = b"sep=,\ntype,sourceName,sourceVersion,productType,device,startDate,endDate,value\nHKCategoryTypeIdentifierAppleStandHour,Watch,26.3,Watch7,dev,2026-03-25 08:00:00 +0000,2026-03-25 09:00:00 +0000,stood\n";
        let mut s = open(csv, "HKCategoryTypeIdentifierAppleStandHour");
        let e = s.next_entity().unwrap().expect("row");
        match e {
            XmlEntity::Record(attrs) => {
                let m: std::collections::HashMap<_, _> = attrs.into_iter().collect();
                assert_eq!(m.get("value").unwrap(), "stood");
            }
            _ => panic!(),
        }
    }
}
