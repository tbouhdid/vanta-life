use serde::{Deserialize, Serialize};

/// A user supplied snapshot of the conditions under which VANTA makes a
/// decision. Timestamps are RFC 3339 UTC strings so they cross the Tauri
/// boundary without a frontend-specific date representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeState {
    pub id: String,
    pub timestamp: String,
    pub energy: f32,
    pub focus: f32,
    pub stress: f32,
    pub sleep_hours: f32,
    pub available_minutes: u32,
    pub optional_note: Option<String>,
}

impl LifeState {
    pub fn validate(&self) -> Result<(), String> {
        validate_scale("energy", self.energy)?;
        validate_scale("focus", self.focus)?;
        validate_scale("stress", self.stress)?;

        if !self.sleep_hours.is_finite() || !(0.0..=24.0).contains(&self.sleep_hours) {
            return Err("sleep_hours must be a finite value between 0 and 24.".to_owned());
        }

        if self.available_minutes > 1_440 {
            return Err("available_minutes must be between 0 and 1440.".to_owned());
        }

        if self
            .optional_note
            .as_deref()
            .is_some_and(|note| note.chars().count() > 1_200)
        {
            return Err("optional_note must be at most 1200 characters.".to_owned());
        }

        Ok(())
    }
}

pub fn validate_scale(field: &str, value: f32) -> Result<(), String> {
    if value.is_finite() && (0.0..=10.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be a finite value between 0 and 10."))
    }
}
