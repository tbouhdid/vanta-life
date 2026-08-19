use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: f32,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

impl Goal {
    pub fn validate_fields(title: &str, priority: f32) -> Result<(), String> {
        if title.trim().is_empty() {
            return Err("A goal title is required.".to_owned());
        }

        if title.trim().chars().count() > 160 {
            return Err("A goal title must be at most 160 characters.".to_owned());
        }

        if !priority.is_finite() || !(0.0..=10.0).contains(&priority) {
            return Err("goal priority must be a finite value between 0 and 10.".to_owned());
        }

        Ok(())
    }
}
