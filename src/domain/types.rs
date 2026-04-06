use std::path::PathBuf;

use serde::Serialize;

use crate::domain::{Record, Workout};

pub type JsonValue = serde_json::Value;

#[derive(Debug, Clone)]
pub enum ParsedEntity {
    Record(Record),
    Workout(Workout),
}

#[derive(Debug, Clone)]
pub struct IngestConfig {
    pub input_path: PathBuf,
    pub db_path: PathBuf,
    pub batch_size: usize,
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct InspectConfig {
    pub db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StatsConfig {
    pub db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub db_path: PathBuf,
    pub sql: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DateRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeCount {
    pub record_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceCount {
    pub source_name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectSummary {
    pub tables: Vec<String>,
    pub record_count: i64,
    pub workout_count: i64,
    pub date_range: DateRange,
    pub sources: Vec<String>,
    pub record_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSummary {
    pub total_records: i64,
    pub total_workouts: i64,
    pub top_types: Vec<TypeCount>,
    pub top_sources: Vec<SourceCount>,
    pub recent_activity: bool,
}
