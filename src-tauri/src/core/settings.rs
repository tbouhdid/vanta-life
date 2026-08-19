use serde::{Deserialize, Serialize};

pub const API_CONFIGURATION_NOT_CONFIGURED: &str = "not_configured";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalSettings {
    pub start_week_day: String,
    pub default_available_minutes: u32,
    pub ai_enabled: bool,
    pub api_configuration_status: String,
    pub contextual_review_enabled: bool,
    pub activity_awareness_enabled: bool,
    pub notifications_enabled: bool,
    pub intervention_cooldown_minutes: u32,
    pub start_with_windows: bool,
}

impl Default for LocalSettings {
    fn default() -> Self {
        Self {
            start_week_day: "monday".to_owned(),
            default_available_minutes: 120,
            ai_enabled: false,
            // This is deliberately a status only. No API secret is stored in
            // SQLite; a future secure-secret adapter can supply this value.
            api_configuration_status: API_CONFIGURATION_NOT_CONFIGURED.to_owned(),
            contextual_review_enabled: false,
            activity_awareness_enabled: false,
            notifications_enabled: false,
            intervention_cooldown_minutes: 90,
            start_with_windows: false,
        }
    }
}

impl LocalSettings {
    pub fn validate(&self) -> Result<(), String> {
        const VALID_WEEK_DAYS: [&str; 7] = [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
        ];

        if !VALID_WEEK_DAYS.contains(&self.start_week_day.as_str()) {
            return Err(
                "start_week_day must be a weekday written in lowercase English.".to_owned(),
            );
        }

        if self.default_available_minutes > 1_440 {
            return Err("default_available_minutes must be between 0 and 1440.".to_owned());
        }

        if !(15..=1_440).contains(&self.intervention_cooldown_minutes) {
            return Err("intervention_cooldown_minutes must be between 15 and 1440.".to_owned());
        }

        Ok(())
    }
}
