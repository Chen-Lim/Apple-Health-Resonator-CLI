use std::path::Path;

const SUPPORTED_EXPORT_XML_NAMES: &[&str] = &["export.xml", "导出.xml"];
const SIMPLE_HEALTH_CSV_SUFFIX: &str = "_SimpleHealthExportCSV.csv";

pub(crate) fn is_supported_export_xml_name(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SUPPORTED_EXPORT_XML_NAMES.contains(&name))
}

pub(crate) fn is_simple_health_export_csv_name(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(SIMPLE_HEALTH_CSV_SUFFIX))
}

/// Extract the HealthKit type identifier from a SimpleHealthExportCSV file name.
///
/// Names look like `<TypeId>_<YYYY-MM-DD..._HH-MM-SS>_SimpleHealthExportCSV.csv`.
/// The type id is everything up to the first underscore followed by a 4-digit year.
pub(crate) fn type_id_from_csv_filename(path: &str) -> Option<String> {
    let name = Path::new(path).file_name().and_then(|n| n.to_str())?;
    let stem = name.strip_suffix(SIMPLE_HEALTH_CSV_SUFFIX)?;
    let bytes = stem.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b == b'_' && idx + 5 <= bytes.len() {
            let after = &bytes[idx + 1..idx + 5];
            if after.iter().all(|c| c.is_ascii_digit()) {
                return Some(stem[..idx].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_quantity_type() {
        let n = "HKQuantityTypeIdentifierHeartRate_2026-04-116_19-34-19_SimpleHealthExportCSV.csv";
        assert_eq!(
            type_id_from_csv_filename(n).as_deref(),
            Some("HKQuantityTypeIdentifierHeartRate")
        );
    }

    #[test]
    fn extracts_workout_activity_type() {
        let n = "HKWorkoutActivityTypeSwimming_2026-04-116_19-34-24_SimpleHealthExportCSV.csv";
        assert_eq!(
            type_id_from_csv_filename(n).as_deref(),
            Some("HKWorkoutActivityTypeSwimming")
        );
    }

    #[test]
    fn detects_csv_suffix() {
        assert!(is_simple_health_export_csv_name(
            "HKQuantityTypeIdentifierHeartRate_2026-04-116_19-34-19_SimpleHealthExportCSV.csv"
        ));
        assert!(!is_simple_health_export_csv_name("export.xml"));
        assert!(!is_simple_health_export_csv_name("random.csv"));
    }
}
