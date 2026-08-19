use serde::{Deserialize, Serialize};

use crate::core::{
    action_execution::{AbandonActionInput, ActionExecution, CompleteActionInput},
    candidate_action::CandidateAction,
    chat::ChatMessage,
    decision_result::DecisionResponse,
    goal::Goal,
    life_state::LifeState,
    memory::CandidateMemory,
    outcome::ActionOutcome,
    profile::Profile,
    settings::LocalSettings,
};

#[derive(Debug, Clone, Serialize)]
pub struct AiStatus {
    pub enabled: bool,
    pub configured: bool,
    pub available: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetOpenAiApiKeyInput {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatSendInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub user_message: ChatMessage,
    pub assistant_message: Option<ChatMessage>,
    pub status: AiStatus,
    pub tool_calls_used: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionConsultationInput {
    pub decision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiDecisionReview {
    pub ranking_override_recommended: bool,
    pub preferred_action_id: Option<String>,
    pub confidence: f32,
    pub contextual_factors: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionConsultationResponse {
    pub deterministic_next_best_action: Option<crate::core::decision_result::DecisionResult>,
    pub ai_contextual_note: Option<AiDecisionReview>,
    pub status: AiStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryAnalysisResponse {
    pub candidate_memory: Option<CandidateMemory>,
    pub status: AiStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LifeStateInput {
    pub energy: f32,
    pub focus: f32,
    pub stress: f32,
    pub sleep_hours: f32,
    pub available_minutes: u32,
    #[serde(default)]
    pub optional_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoalInput {
    pub title: String,
    pub description: String,
    pub priority: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGoalInput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToggleGoalActiveInput {
    pub id: String,
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityIdInput {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionItemInput {
    pub title: String,
    pub description: String,
    pub goal_id: Option<String>,
    pub impact: f32,
    pub urgency: f32,
    pub goal_alignment: f32,
    pub energy_required: f32,
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateActionItemInput {
    pub id: String,
    pub title: String,
    pub description: String,
    pub goal_id: Option<String>,
    pub impact: f32,
    pub urgency: f32,
    pub goal_alignment: f32,
    pub energy_required: f32,
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CalculateDecisionInput {
    #[serde(default)]
    pub excluded_action_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartRecommendedActionInput {
    pub action_id: String,
    pub decision_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsInput {
    #[serde(default)]
    pub name: Option<String>,
    pub start_week_day: String,
    pub default_available_minutes: u32,
    pub ai_enabled: bool,
    #[serde(default)]
    pub contextual_review_enabled: bool,
    #[serde(default)]
    pub activity_awareness_enabled: bool,
    #[serde(default)]
    pub notifications_enabled: bool,
    #[serde(default = "default_intervention_cooldown_minutes")]
    pub intervention_cooldown_minutes: u32,
    #[serde(default)]
    pub start_with_windows: bool,
}

fn default_intervention_cooldown_minutes() -> u32 {
    90
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileInput {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteOnboardingInput {
    pub name: String,
    pub goal: GoalInput,
    pub life_state: LifeStateInput,
    #[serde(default)]
    pub actions: Vec<ActionItemInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BootstrapData {
    pub profile: Option<Profile>,
    pub settings: LocalSettings,
    pub life_state: Option<LifeState>,
    pub goals: Vec<Goal>,
    pub actions: Vec<CandidateAction>,
    pub decision: Option<DecisionResponse>,
    pub active_execution: Option<ActionExecution>,
    pub recent_outcomes: Vec<ActionOutcome>,
    pub ai_status: AiStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsUpdateResult {
    pub profile: Profile,
    pub settings: LocalSettings,
}

// Re-export these existing core input types from the application boundary so
// Tauri commands only depend on a single application module.
pub type CompleteExecutionInput = CompleteActionInput;
pub type AbandonExecutionInput = AbandonActionInput;
