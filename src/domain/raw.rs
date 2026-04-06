#[derive(Debug, Clone, Default)]
pub struct RawRecord {
    pub record_type: Option<String>,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub source_name: Option<String>,
    pub source_version: Option<String>,
    pub device: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RawWorkout {
    pub workout_type: Option<String>,
    pub duration: Option<String>,
    pub duration_unit: Option<String>,
    pub total_distance: Option<String>,
    pub total_energy_burned: Option<String>,
    pub source_name: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
