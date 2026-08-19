use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::decision_result::DecisionResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    InProgress,
    Completed,
    Abandoned,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(format!("Unknown execution status '{value}'.")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecution {
    pub id: String,
    pub action_id: String,
    pub action_title: String,
    pub decision_id: Option<String>,
    pub decision_score: f32,
    pub started_at: DateTime<Utc>,
    pub energy_before: f32,
    pub status: ExecutionStatus,
}

#[derive(Debug, Deserialize)]
pub struct CompleteActionInput {
    pub result_quality: f32,
    pub energy_after: f32,
    pub difficulty: f32,
}

#[derive(Debug, Deserialize)]
pub struct AbandonActionInput {
    pub energy_after: f32,
    pub difficulty: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionExecutionError {
    InvalidScaleValue { field: &'static str, value: f32 },
    ExecutionNotInProgress,
}

impl std::fmt::Display for ActionExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidScaleValue { field, value } => {
                write!(
                    formatter,
                    "{field} must be a finite value between 0 and 10, received {value}."
                )
            }
            Self::ExecutionNotInProgress => {
                write!(formatter, "The action is no longer in progress.")
            }
        }
    }
}

impl std::error::Error for ActionExecutionError {}

pub fn start_action(
    decision: &DecisionResult,
    energy_before: f32,
    started_at: DateTime<Utc>,
) -> Result<ActionExecution, ActionExecutionError> {
    validate_scale_value("energy_before", energy_before)?;

    Ok(ActionExecution {
        id: String::new(),
        action_id: decision.action_id.clone(),
        action_title: decision.action_title.clone(),
        decision_id: None,
        decision_score: decision.score,
        started_at,
        energy_before,
        status: ExecutionStatus::InProgress,
    })
}

pub(crate) fn validate_scale_value(
    field: &'static str,
    value: f32,
) -> Result<(), ActionExecutionError> {
    if value.is_finite() && (0.0..=10.0).contains(&value) {
        Ok(())
    } else {
        Err(ActionExecutionError::InvalidScaleValue { field, value })
    }
}
