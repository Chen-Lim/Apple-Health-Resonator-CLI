use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Workout {
    pub workout_type: String,
    pub duration: Option<f64>,
    pub duration_unit: Option<String>,
    pub total_distance: Option<f64>,
    pub total_energy_burned: Option<f64>,
    pub source_name: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub dedupe_key: String,
}
