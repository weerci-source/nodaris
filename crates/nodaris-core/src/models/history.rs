use serde::{Deserialize, Serialize};

use crate::error::ApiError;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    Session,
    Ethernet,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HistoryEntry<T> {
    pub id: String,
    pub kind: HistoryKind,
    pub target: Option<String>,
    pub report: T,
}

#[derive(Serialize)]
pub struct RecordedReport<T> {
    #[serde(flatten)]
    pub report: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_error: Option<ApiError>,
}
