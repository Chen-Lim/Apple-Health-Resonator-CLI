use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Record {
    pub record_type: String,
    pub value_text: Option<String>,
    pub value_num: Option<f64>,
    pub unit: Option<String>,
    pub source_name: Option<String>,
    pub source_version: Option<String>,
    pub device: Option<String>,
    pub creation_date: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub dedupe_key: String,
}
