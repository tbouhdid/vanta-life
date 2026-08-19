use serde::{Deserialize, Serialize};

use super::outcome::ActionOutcome;

/// A directly observed fact. Observations are deliberately kept separate from
/// model-generated inferences and from memories approved for persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryObservation {
    pub content: String,
    pub source: String,
}

/// A hypothesis based on observations. It is not a fact about the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryInference {
    pub content: String,
    pub confidence: f32,
}

/// A non-persistent proposal. Saving it requires an explicit future user
/// confirmation flow; this Alpha never saves candidate memories automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateMemory {
    pub observation: MemoryObservation,
    pub inference: Option<MemoryInference>,
    pub proposed_content: String,
    pub importance: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredMemory {
    pub id: String,
    pub category: String,
    pub statement: String,
    pub source: String,
    pub confidence: f32,
    pub importance: f32,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub active: bool,
}

/// A conservative, deterministic proposal derived from repeated recorded
/// outcomes. It deliberately stores an observation, not a causal claim.
pub fn repeated_evidence_memory(outcomes: &[ActionOutcome]) -> Option<CandidateMemory> {
    let matching = outcomes
        .iter()
        .filter(|outcome| {
            outcome.completed
                && outcome.energy_before >= 7.0
                && outcome.result_quality.is_some_and(|quality| quality >= 7.0)
        })
        .count();

    if matching < 3 {
        return None;
    }

    let confidence = (0.45 + matching as f32 * 0.08).min(0.8);
    let observation = format!(
        "{matching} recorded completed sessions started with energy at least 7/10 and had result quality at least 7/10."
    );
    Some(CandidateMemory {
        observation: MemoryObservation {
            content: observation.clone(),
            source: "deterministic_outcome_analysis".to_owned(),
        },
        inference: Some(MemoryInference {
            content:
                "High-energy sessions appear to coincide with stronger recorded completion quality."
                    .to_owned(),
            confidence,
        }),
        // This statement is intentionally observational: it does not claim
        // causation from a small personal sample.
        proposed_content: observation,
        importance: 6.0,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::repeated_evidence_memory;
    use crate::core::outcome::ActionOutcome;

    fn outcome(energy_before: f32, quality: Option<f32>) -> ActionOutcome {
        let timestamp = Utc::now();
        ActionOutcome {
            id: "outcome".to_owned(),
            execution_id: "execution".to_owned(),
            action_id: "action".to_owned(),
            action_title: "Action".to_owned(),
            decision_id: None,
            decision_score: 0.7,
            recommended: true,
            accepted: true,
            started_at: timestamp,
            ended_at: timestamp,
            created_at: timestamp,
            actual_duration_minutes: 20,
            completed: true,
            abandoned: false,
            result_quality: quality,
            energy_before,
            energy_after: Some(energy_before - 1.0),
            difficulty: Some(5.0),
        }
    }

    #[test]
    fn insufficient_data_does_not_create_a_memory() {
        assert!(
            repeated_evidence_memory(&[outcome(8.0, Some(8.0)), outcome(7.0, Some(7.0))]).is_none()
        );
    }

    #[test]
    fn repeated_evidence_creates_a_bounded_memory() {
        let memory = repeated_evidence_memory(&[
            outcome(8.0, Some(8.0)),
            outcome(7.0, Some(7.0)),
            outcome(9.0, Some(9.0)),
        ])
        .expect("three observations should qualify");
        assert!(memory.proposed_content.contains("3 recorded"));
        assert!((0.0..=1.0).contains(&memory.confidence));
    }
}
