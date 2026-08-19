use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    LifeState,
    Decision,
    Execution,
    Outcome,
    Memory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub kind: HistoryKind,
    pub timestamp: String,
    pub title: String,
    pub detail: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryDay {
    pub date: String,
    pub entries: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub days: Vec<HistoryDay>,
}
