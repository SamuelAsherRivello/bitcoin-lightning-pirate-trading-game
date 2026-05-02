use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateData {
    pub id: i64,
    pub message: String,
}

impl TemplateData {
    pub fn seed() -> Self {
        Self {
            id: 1,
            message: "Hello, World!".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum TemplateDataSource {
    BrowserSnapshot,
    Database,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TemplateDataLoadResult {
    pub data: TemplateData,
    pub source: TemplateDataSource,
    pub db_last_loaded_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemplateDataLoadRequest {
    pub sequence: u64,
}

impl TemplateDataLoadRequest {
    pub fn initial() -> Self {
        Self { sequence: 0 }
    }
}
