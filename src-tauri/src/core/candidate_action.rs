use serde::{Deserialize, Serialize};

use super::life_state::validate_scale;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Available,
    InProgress,
    Completed,
    Archived,
}

impl ActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            // `open` was the status used by the first persistence prototype.
            // Accept it when reading old snapshots; migration 3 writes
            // `available` from now on.
            "open" | "available" => Ok(Self::Available),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("Unknown action status '{value}'.")),
        }
    }
}

/// A real, user-owned action. The decision engine only reads this type; it
/// never knows whether the value came from SQLite, a test, or another source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateAction {
    pub id: String,
    pub title: String,
    pub description: String,
    pub goal_id: Option<String>,
    pub impact: f32,
    pub urgency: f32,
    pub goal_alignment: f32,
    pub energy_required: f32,
    pub duration_minutes: u32,
    pub status: ActionStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl CandidateAction {
    pub fn validate_fields(
        title: &str,
        impact: f32,
        urgency: f32,
        goal_alignment: f32,
        energy_required: f32,
        duration_minutes: u32,
    ) -> Result<(), String> {
        if title.trim().is_empty() {
            return Err("An action title is required.".to_owned());
        }

        if title.trim().chars().count() > 160 {
            return Err("An action title must be at most 160 characters.".to_owned());
        }

        validate_scale("impact", impact)?;
        validate_scale("urgency", urgency)?;
        validate_scale("goal_alignment", goal_alignment)?;
        validate_scale("energy_required", energy_required)?;

        if duration_minutes == 0 || duration_minutes > 1_440 {
            return Err("duration_minutes must be between 1 and 1440.".to_owned());
        }

        Ok(())
    }
}
