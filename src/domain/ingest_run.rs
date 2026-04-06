#[derive(Debug, Clone)]
pub struct IngestRun {
    pub started_at: String,
    pub finished_at: Option<String>,
    pub input_path: String,
    pub records_inserted: i64,
    pub workouts_inserted: i64,
    pub records_skipped: i64,
    pub errors_count: i64,
    pub schema_version: String,
}
