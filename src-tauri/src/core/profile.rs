use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub onboarding_completed: bool,
    pub default_available_minutes: u32,
}

impl Profile {
    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Your name is required.".to_owned());
        }

        if name.trim().chars().count() > 120 {
            return Err("Your name must be at most 120 characters.".to_owned());
        }

        Ok(())
    }
}
