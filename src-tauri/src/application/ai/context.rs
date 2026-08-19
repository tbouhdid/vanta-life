use crate::core::{
    action_execution::ActionExecution,
    candidate_action::{ActionStatus, CandidateAction},
    decision_result::DecisionResponse,
    goal::Goal,
    life_state::LifeState,
    memory::StoredMemory,
    outcome::ActionOutcome,
    profile::Profile,
};

pub const MAX_CONTEXT_CHARS: usize = 12_000;

#[derive(Debug, Clone)]
pub struct AiContextSnapshot {
    pub profile: Option<Profile>,
    pub life_state: Option<LifeState>,
    pub active_goals: Vec<Goal>,
    pub active_execution: Option<ActionExecution>,
    pub available_actions: Vec<CandidateAction>,
    pub latest_decision: Option<DecisionResponse>,
    pub recent_outcomes: Vec<ActionOutcome>,
    pub memories: Vec<StoredMemory>,
}

#[derive(Debug, Clone)]
pub struct BuiltAiContext {
    pub text: String,
}

/// Limits every collection before rendering it so AI requests contain a useful
/// working set rather than a full local database export.
pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build(snapshot: &AiContextSnapshot) -> BuiltAiContext {
        let mut sections = Vec::new();
        if let Some(profile) = &snapshot.profile {
            sections.push(format!("PROFILE\nName: {}", profile.name));
        }
        if let Some(state) = &snapshot.life_state {
            sections.push(format!(
                "CURRENT LIFE STATE\nEnergy: {:.1}/10\nFocus: {:.1}/10\nStress: {:.1}/10\nSleep: {:.1} hours\nAvailable time: {} minutes\nCaptured: {}",
                state.energy, state.focus, state.stress, state.sleep_hours, state.available_minutes, state.timestamp
            ));
        } else {
            sections.push("CURRENT LIFE STATE\nNot recorded yet.".to_owned());
        }
        sections.push(list_lines(
            "ACTIVE GOALS",
            snapshot.active_goals.iter().take(6).map(|goal| {
                format!(
                    "- {} (priority {:.1}): {}",
                    goal.title, goal.priority, goal.description
                )
            }),
            "No active goals.",
        ));
        if let Some(execution) = &snapshot.active_execution {
            sections.push(format!(
                "ACTIVE EXECUTION\n{} started at {} (energy before {:.1}/10).",
                execution.action_title, execution.started_at, execution.energy_before
            ));
        }
        sections.push(list_lines(
            "AVAILABLE ACTIONS",
            snapshot.available_actions.iter().filter(|action| action.status == ActionStatus::Available).take(12).map(|action| format!("- {} [id:{}] — impact {:.1}, urgency {:.1}, alignment {:.1}, requires energy {:.1}, {} min", action.title, action.id, action.impact, action.urgency, action.goal_alignment, action.energy_required, action.duration_minutes)),
            "No available actions.",
        ));
        if let Some(decision) = &snapshot.latest_decision {
            let selected = decision
                .next_best_action
                .as_ref()
                .map(|item| {
                    format!(
                        "{} [id:{}], score {:.2}; {}",
                        item.action_title, item.action_id, item.score, item.reason
                    )
                })
                .unwrap_or_else(|| "No feasible action.".to_owned());
            sections.push(format!("LATEST DETERMINISTIC DECISION\n{selected}"));
        }
        sections.push(list_lines(
            "RECENT OUTCOMES",
            snapshot.recent_outcomes.iter().take(6).map(|outcome| {
                format!(
                    "- {}: {} after {} min (quality: {})",
                    outcome.action_title,
                    if outcome.completed {
                        "completed"
                    } else {
                        "abandoned"
                    },
                    outcome.actual_duration_minutes,
                    outcome
                        .result_quality
                        .map(|value| format!("{value:.1}/10"))
                        .unwrap_or_else(|| "not rated".to_owned())
                )
            }),
            "No outcomes recorded.",
        ));
        sections.push(list_lines(
            "STORED MEMORIES (not assumptions)",
            snapshot
                .memories
                .iter()
                .take(6)
                .map(|memory| format!("- {}", memory.statement)),
            "No stored memories.",
        ));
        let mut text = sections.join("\n\n");
        if text.chars().count() > MAX_CONTEXT_CHARS {
            text = text.chars().take(MAX_CONTEXT_CHARS).collect::<String>();
            text.push_str("\n[Context truncated to budget]");
        }
        BuiltAiContext { text }
    }
}

fn list_lines<I>(heading: &str, lines: I, empty: &str) -> String
where
    I: IntoIterator<Item = String>,
{
    let lines = lines.into_iter().collect::<Vec<_>>();
    if lines.is_empty() {
        format!("{heading}\n{empty}")
    } else {
        format!("{heading}\n{}", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::{AiContextSnapshot, ContextBuilder, MAX_CONTEXT_CHARS};

    #[test]
    fn context_builder_keeps_a_bounded_relevant_context() {
        let snapshot = AiContextSnapshot {
            profile: None,
            life_state: None,
            active_goals: vec![],
            active_execution: None,
            available_actions: vec![],
            latest_decision: None,
            recent_outcomes: vec![],
            memories: vec![],
        };
        let context = ContextBuilder::build(&snapshot);
        assert!(context.text.contains("CURRENT LIFE STATE"));
        assert!(context.text.contains("No active goals."));
        assert!(context.text.chars().count() <= MAX_CONTEXT_CHARS + 40);
    }
}
