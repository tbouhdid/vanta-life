use serde::Serialize;

use super::outcome::ActionOutcome;

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticsSummary {
    pub sample_size: usize,
    pub completion_rate: Option<f32>,
    pub average_result_quality: Option<f32>,
    pub average_duration_minutes: Option<f32>,
    pub average_energy_before: Option<f32>,
    pub average_energy_after: Option<f32>,
}

pub fn summarize_outcomes(outcomes: &[ActionOutcome]) -> AnalyticsSummary {
    if outcomes.is_empty() {
        return AnalyticsSummary {
            sample_size: 0,
            completion_rate: None,
            average_result_quality: None,
            average_duration_minutes: None,
            average_energy_before: None,
            average_energy_after: None,
        };
    }

    let count = outcomes.len() as f32;
    let quality = outcomes
        .iter()
        .filter_map(|outcome| outcome.result_quality)
        .collect::<Vec<_>>();
    let energy_after = outcomes
        .iter()
        .filter_map(|outcome| outcome.energy_after)
        .collect::<Vec<_>>();
    AnalyticsSummary {
        sample_size: outcomes.len(),
        completion_rate: Some(
            outcomes.iter().filter(|outcome| outcome.completed).count() as f32 / count,
        ),
        average_result_quality: average(&quality),
        average_duration_minutes: Some(
            outcomes
                .iter()
                .map(|outcome| outcome.actual_duration_minutes as f32)
                .sum::<f32>()
                / count,
        ),
        average_energy_before: Some(
            outcomes
                .iter()
                .map(|outcome| outcome.energy_before)
                .sum::<f32>()
                / count,
        ),
        average_energy_after: average(&energy_after),
    }
}

fn average(values: &[f32]) -> Option<f32> {
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::summarize_outcomes;
    use crate::core::outcome::ActionOutcome;
    use chrono::Utc;

    fn outcome(completed: bool, quality: Option<f32>) -> ActionOutcome {
        let timestamp = Utc::now();
        ActionOutcome {
            id: "outcome".to_owned(),
            execution_id: "execution".to_owned(),
            action_id: "action".to_owned(),
            action_title: "Action".to_owned(),
            decision_id: None,
            decision_score: 0.8,
            recommended: true,
            accepted: true,
            started_at: timestamp,
            ended_at: timestamp,
            created_at: timestamp,
            actual_duration_minutes: 30,
            completed,
            abandoned: !completed,
            result_quality: quality,
            energy_before: 6.0,
            energy_after: Some(5.0),
            difficulty: Some(5.0),
        }
    }

    #[test]
    fn no_data_never_returns_fake_analytics() {
        let summary = summarize_outcomes(&[]);
        assert_eq!(summary.sample_size, 0);
        assert_eq!(summary.completion_rate, None);
    }

    #[test]
    fn aggregates_real_outcomes() {
        let summary = summarize_outcomes(&[outcome(true, Some(8.)), outcome(false, None)]);
        assert_eq!(summary.completion_rate, Some(0.5));
        assert_eq!(summary.average_result_quality, Some(8.0));
    }
}
