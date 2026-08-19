use serde_json::{json, Value};

use crate::core::{
    candidate_action::CandidateAction, decision_result::DecisionResponse, goal::Goal,
    life_state::LifeState, outcome::ActionOutcome,
};

use super::provider::{AiError, AiToolCall, AiToolDefinition, AiToolOutput};

#[derive(Debug, Clone)]
pub struct ReadOnlyToolContext {
    pub life_state: Option<LifeState>,
    pub active_goals: Vec<Goal>,
    pub available_actions: Vec<CandidateAction>,
    pub next_best_action: Option<DecisionResponse>,
    pub recent_outcomes: Vec<ActionOutcome>,
}

pub fn read_only_tool_definitions() -> Vec<AiToolDefinition> {
    let empty = json!({"type":"object","properties":{},"additionalProperties":false});
    vec![
        definition(
            "get_current_life_state",
            "Read the current recorded life state.",
            empty.clone(),
        ),
        definition("get_active_goals", "Read active goals.", empty.clone()),
        definition(
            "get_available_actions",
            "Read available open actions.",
            empty.clone(),
        ),
        definition(
            "get_next_best_action",
            "Read the latest deterministic next best action.",
            empty.clone(),
        ),
        definition(
            "get_recent_outcomes",
            "Read recent completed or abandoned action outcomes.",
            empty,
        ),
    ]
}

pub fn execute_read_only_tool(
    context: &ReadOnlyToolContext,
    call: &AiToolCall,
) -> Result<AiToolOutput, AiError> {
    let output = match call.name.as_str() {
        "get_current_life_state" => json!(context.life_state),
        "get_active_goals" => json!(context.active_goals),
        "get_available_actions" => json!(context.available_actions),
        "get_next_best_action" => json!(context.next_best_action),
        "get_recent_outcomes" => json!(context.recent_outcomes),
        "propose_create_action" | "propose_update_goal" => return Err(AiError::Provider("Mutation proposals require an explicit user confirmation UI and cannot be executed by AI tools.".to_owned())),
        _ => return Err(AiError::InvalidResponse(format!("Unknown AI tool '{}'.", call.name))),
    };
    Ok(AiToolOutput {
        call_id: call.call_id.clone(),
        output,
    })
}

fn definition(name: &str, description: &str, parameters: Value) -> AiToolDefinition {
    AiToolDefinition {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters,
    }
}

#[cfg(test)]
mod tests {
    use super::{execute_read_only_tool, ReadOnlyToolContext};
    use crate::application::ai::provider::AiToolCall;
    use serde_json::json;

    #[test]
    fn mutation_tools_are_rejected_at_the_execution_boundary() {
        let context = ReadOnlyToolContext {
            life_state: None,
            active_goals: vec![],
            available_actions: vec![],
            next_best_action: None,
            recent_outcomes: vec![],
        };
        let call = AiToolCall {
            call_id: "call_1".to_owned(),
            name: "propose_create_action".to_owned(),
            arguments: json!({}),
        };
        assert!(execute_read_only_tool(&context, &call).is_err());
    }
}
