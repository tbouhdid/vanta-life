use chrono::{DateTime, Utc};

use crate::core::{
    action_execution::{
        AbandonActionInput, ActionExecution, ActionExecutionError, CompleteActionInput,
    },
    outcome::{abandon_action, complete_action, ActionOutcome},
};

/// Volatile session data owned by the application shell. It is intentionally
/// in-memory only and is discarded when the desktop application closes.
#[derive(Default)]
pub struct SessionState {
    active_execution: Option<ActionExecution>,
    outcomes: Vec<ActionOutcome>,
}

#[derive(Debug, PartialEq)]
pub enum SessionStateError {
    ActiveExecutionExists,
    NoActiveExecution,
    Domain(ActionExecutionError),
}

impl std::fmt::Display for SessionStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActiveExecutionExists => {
                write!(formatter, "An action is already in progress.")
            }
            Self::NoActiveExecution => write!(formatter, "There is no action in progress."),
            Self::Domain(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SessionStateError {}

impl From<ActionExecutionError> for SessionStateError {
    fn from(error: ActionExecutionError) -> Self {
        Self::Domain(error)
    }
}

impl SessionState {
    pub fn start_execution(&mut self, execution: ActionExecution) -> Result<(), SessionStateError> {
        if self.active_execution.is_some() {
            return Err(SessionStateError::ActiveExecutionExists);
        }

        self.active_execution = Some(execution);
        Ok(())
    }

    pub fn active_execution(&self) -> Option<ActionExecution> {
        self.active_execution.clone()
    }

    pub fn outcomes(&self) -> Vec<ActionOutcome> {
        self.outcomes.clone()
    }

    pub fn complete_active_action(
        &mut self,
        input: CompleteActionInput,
        ended_at: DateTime<Utc>,
    ) -> Result<ActionOutcome, SessionStateError> {
        let execution = self
            .active_execution
            .as_ref()
            .ok_or(SessionStateError::NoActiveExecution)?;
        let outcome = complete_action(execution, input, ended_at)?;

        self.active_execution = None;
        self.outcomes.push(outcome.clone());

        Ok(outcome)
    }

    pub fn abandon_active_action(
        &mut self,
        input: AbandonActionInput,
        ended_at: DateTime<Utc>,
    ) -> Result<ActionOutcome, SessionStateError> {
        let execution = self
            .active_execution
            .as_ref()
            .ok_or(SessionStateError::NoActiveExecution)?;
        let outcome = abandon_action(execution, input, ended_at)?;

        self.active_execution = None;
        self.outcomes.push(outcome.clone());

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{SessionState, SessionStateError};
    use crate::core::{
        action_execution::{start_action, CompleteActionInput},
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

    fn timestamp() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 12, 13, 0, 0)
            .single()
            .expect("fixed test timestamp should be valid")
    }

    #[test]
    fn completing_without_an_active_execution_is_rejected() {
        let mut session_state = SessionState::default();
        let ended_at = timestamp();

        let result = session_state.complete_active_action(
            CompleteActionInput {
                result_quality: 8.0,
                energy_after: 5.0,
                difficulty: 6.0,
            },
            ended_at,
        );

        assert_eq!(result.unwrap_err(), SessionStateError::NoActiveExecution);
    }

    #[test]
    fn only_one_execution_can_be_active_at_a_time() {
        let mut session_state = SessionState::default();
        let execution = start_action(&decision(), 7.0, timestamp()).expect("valid execution");

        session_state
            .start_execution(execution.clone())
            .expect("first execution should start");
        let result = session_state.start_execution(execution);

        assert_eq!(
            result.unwrap_err(),
            SessionStateError::ActiveExecutionExists
        );
    }
}
