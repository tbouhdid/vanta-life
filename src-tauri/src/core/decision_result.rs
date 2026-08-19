use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DecisionScoreComponents {
    pub impact_score: f32,
    pub urgency_score: f32,
    pub goal_alignment_score: f32,
    pub energy_fit_score: f32,
    pub time_fit_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DecisionResult {
    pub action_id: String,
    pub action_title: String,
    pub score: f32,
    pub feasible: bool,
    pub reason: String,
    pub components: DecisionScoreComponents,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DecisionResponse {
    pub id: String,
    pub timestamp: String,
    pub next_best_action: Option<DecisionResult>,
    pub ranking: Vec<DecisionResult>,
}
