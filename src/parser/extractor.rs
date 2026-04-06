use crate::domain::{RawRecord, RawWorkout};
use crate::parser::xml_reader::XmlEntity;

pub enum ExtractedRaw {
    Record(RawRecord),
    Workout(RawWorkout),
}

pub fn extract(entity: XmlEntity) -> ExtractedRaw {
    match entity {
        XmlEntity::Record(attrs) => ExtractedRaw::Record(extract_record(attrs)),
        XmlEntity::Workout(attrs) => ExtractedRaw::Workout(extract_workout(attrs)),
    }
}

fn extract_record(attrs: Vec<(String, String)>) -> RawRecord {
    let mut raw = RawRecord::default();
    for (key, value) in attrs {
        match key.as_str() {
            "type" => raw.record_type = some_if_present(value),
            "value" => raw.value = some_if_present(value),
            "unit" => raw.unit = some_if_present(value),
            "sourceName" => raw.source_name = some_if_present(value),
            "sourceVersion" => raw.source_version = some_if_present(value),
            "device" => raw.device = some_if_present(value),
            "creationDate" => raw.creation_date = some_if_present(value),
            "startDate" => raw.start_date = some_if_present(value),
            "endDate" => raw.end_date = some_if_present(value),
            _ => {}
        }
    }
    raw
}

fn extract_workout(attrs: Vec<(String, String)>) -> RawWorkout {
    let mut raw = RawWorkout::default();
    for (key, value) in attrs {
        match key.as_str() {
            "workoutActivityType" => raw.workout_type = some_if_present(value),
            "duration" => raw.duration = some_if_present(value),
            "durationUnit" => raw.duration_unit = some_if_present(value),
            "totalDistance" => raw.total_distance = some_if_present(value),
            "totalEnergyBurned" => raw.total_energy_burned = some_if_present(value),
            "sourceName" => raw.source_name = some_if_present(value),
            "creationDate" => raw.creation_date = some_if_present(value),
            "startDate" => raw.start_date = some_if_present(value),
            "endDate" => raw.end_date = some_if_present(value),
            _ => {}
        }
    }
    raw
}

fn some_if_present(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
