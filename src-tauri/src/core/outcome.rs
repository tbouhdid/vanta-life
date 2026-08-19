use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::action_execution::{
    validate_scale_value, AbandonActionInput, ActionExecution, ActionExecutionError,
    CompleteActionInput, ExecutionStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub id: String,
    pub execution_id: String,
    pub action_id: String,
    pub action_title: String,
    pub decision_id: Option<String>,
    pub decision_score: f32,
    pub recommended: bool,
    pub accepted: bool,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub actual_duration_minutes: u64,
    pub completed: bool,
    pub abandoned: bool,
    pub result_quality: Option<f32>,
    pub energy_before: f32,
    pub energy_after: Option<f32>,
    pub difficulty: Option<f32>,
}

pub fn complete_action(
    execution: &ActionExecution,
    input: CompleteActionInput,
    ended_at: DateTime<Utc>,
) -> Result<ActionOutcome, ActionExecutionError> {
    validate_scale_value("result_quality", input.result_quality)?;
    validate_scale_value("energy_after", input.energy_after)?;
    validate_scale_value("difficulty", input.difficulty)?;

    finalize_action(
        execution,
        ended_at,
        Some(input.result_quality),
        Some(input.energy_after),
        Some(input.difficulty),
        ExecutionStatus::Completed,
    )
}

pub fn abandon_action(
    execution: &ActionExecution,
    input: AbandonActionInput,
    ended_at: DateTime<Utc>,
) -> Result<ActionOutcome, ActionExecutionError> {
    validate_scale_value("energy_after", input.energy_after)?;
    validate_scale_value("difficulty", input.difficulty)?;

    finalize_action(
        execution,
        ended_at,
        None,
        Some(input.energy_after),
        Some(input.difficulty),
        ExecutionStatus::Abandoned,
    )
}

fn finalize_action(
    execution: &ActionExecution,
    ended_at: DateTime<Utc>,
    result_quality: Option<f32>,
    energy_after: Option<f32>,
    difficulty: Option<f32>,
    final_status: ExecutionStatus,
) -> Result<ActionOutcome, ActionExecutionError> {
    if execution.status != ExecutionStatus::InProgress {
        return Err(ActionExecutionError::ExecutionNotInProgress);
    }

    let actual_duration_minutes = ended_at
        .signed_duration_since(execution.started_at)
        .num_minutes()
        .max(0) as u64;

    Ok(ActionOutcome {
        id: String::new(),
        execution_id: execution.id.clone(),
        action_id: execution.action_id.clone(),
        action_title: execution.action_title.clone(),
        decision_id: execution.decision_id.clone(),
        decision_score: execution.decision_score,
        recommended: true,
        accepted: true,
        started_at: execution.started_at,
        ended_at,
        created_at: ended_at,
        actual_duration_minutes,
        completed: final_status == ExecutionStatus::Completed,
        abandoned: final_status == ExecutionStatus::Abandoned,
        result_quality,
        energy_before: execution.energy_before,
        energy_after,
        difficulty,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{abandon_action, complete_action};
    use crate::core::{
        action_execution::{
            start_action, AbandonActionInput, ActionExecutionError, CompleteActionInput,
            ExecutionStatus,
        },
        decision_result::{DecisionResult, DecisionScoreComponents},
    };

    fn decision() -> DecisionResult {
        DecisionResult {
            action_id: "action-demo".to_owned(),
            action_title: "Finish prospect demo".to_owned(),
            score: 0.91,
            feasible: true,
            reason: "Test decision.".to_owned(),
            components: DecisionScoreComponents {
                impact_score: 0.9,
                urgency_score: 0.8,
                goal_alignment_score: 0.95,
                energy_fit_score: 1.0,
                time_fit_score: 1.0,
            },
        }
    }

    fn started_at() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 12, 13, 0, 0)
            .single()
            .expect("fixed test timestamp should be valid")
    }

    #[test]
    fn start_action_creates_an_in_progress_execution() {
        let execution = start_action(&decision(), 7.0, started_at()).expect("valid start");

        assert_eq!(execution.status, ExecutionStatus::InProgress);
    }

    #[test]
    fn completing_an_action_creates_a_completed_outcome() {
        let execution = start_action(&decision(), 7.0, started_at()).expect("valid start");
        let ended_at = Utc
            .with_ymd_and_hms(2026, 8, 12, 13, 49, 0)
            .single()
            .expect("fixed test timestamp should be valid");

        let outcome = complete_action(
            &execution,
            CompleteActionInput {
                result_quality: 8.0,
                energy_after: 5.0,
                difficulty: 6.0,
            },
            ended_at,
        )
        .expect("valid completion");

        assert!(outcome.completed);
        assert!(!outcome.abandoned);
        assert_eq!(outcome.actual_duration_minutes, 49);
        assert_eq!(outcome.decision_score, execution.decision_score);
    }

    #[test]
    fn abandoning_an_action_creates_an_abandoned_outcome() {
        let execution = start_action(&decision(), 7.0, started_at()).expect("valid start");
        let ended_at = Utc
            .with_ymd_and_hms(2026, 8, 12, 13, 20, 0)
            .single()
            .expect("fixed test timestamp should be valid");

        let outcome = abandon_action(
            &execution,
            AbandonActionInput {
                energy_after: 6.0,
                difficulty: 7.0,
            },
            ended_at,
        )
        .expect("valid abandonment");

        assert!(outcome.abandoned);
        assert!(!outcome.completed);
        assert_eq!(outcome.result_quality, None);
    }

    #[test]
    fn actual_duration_is_never_negative() {
        let execution = start_action(&decision(), 7.0, started_at()).expect("valid start");
        let ended_before_start = Utc
            .with_ymd_and_hms(2026, 8, 12, 12, 30, 0)
            .single()
            .expect("fixed test timestamp should be valid");

        let outcome = complete_action(
            &execution,
            CompleteActionInput {
                result_quality: 8.0,
                energy_after: 5.0,
                difficulty: 6.0,
            },
            ended_before_start,
        )
        .expect("valid completion");

        assert_eq!(outcome.actual_duration_minutes, 0);
    }

    #[test]
    fn values_outside_the_zero_to_ten_scale_are_rejected() {
        let execution = start_action(&decision(), 7.0, started_at()).expect("valid start");
        let completed = complete_action(
            &execution,
            CompleteActionInput {
                result_quality: 22.0,
                energy_after: 5.0,
                difficulty: 6.0,
            },
            started_at(),
        );
        let abandoned = abandon_action(
            &execution,
            AbandonActionInput {
                energy_after: 5.0,
                difficulty: -3.0,
            },
            started_at(),
        );
        let invalid_start = start_action(&decision(), 15.0, started_at());

        assert!(matches!(
            completed,
            Err(ActionExecutionError::InvalidScaleValue { .. })
        ));
        assert!(matches!(
            abandoned,
            Err(ActionExecutionError::InvalidScaleValue { .. })
        ));
        assert!(matches!(
            invalid_start,
            Err(ActionExecutionError::InvalidScaleValue { .. })
        ));
    }
}
