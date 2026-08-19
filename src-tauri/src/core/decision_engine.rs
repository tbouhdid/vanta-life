use super::{
    candidate_action::CandidateAction,
    decision_result::{DecisionResult, DecisionScoreComponents},
    life_state::LifeState,
};

const ATTRIBUTE_MAX: f32 = 10.0;
const IMPACT_WEIGHT: f32 = 0.30;
const URGENCY_WEIGHT: f32 = 0.20;
const GOAL_ALIGNMENT_WEIGHT: f32 = 0.30;
const ENERGY_FIT_WEIGHT: f32 = 0.10;
const TIME_FIT_WEIGHT: f32 = 0.10;

pub fn evaluate_action(life_state: &LifeState, action: &CandidateAction) -> DecisionResult {
    let components = DecisionScoreComponents {
        impact_score: normalize_attribute(action.impact),
        urgency_score: normalize_attribute(action.urgency),
        goal_alignment_score: normalize_attribute(action.goal_alignment),
        energy_fit_score: calculate_energy_fit(life_state.energy, action.energy_required),
        time_fit_score: calculate_time_fit(life_state.available_minutes, action.duration_minutes),
    };
    let feasible = action.duration_minutes <= life_state.available_minutes;
    let score = (components.impact_score * IMPACT_WEIGHT
        + components.urgency_score * URGENCY_WEIGHT
        + components.goal_alignment_score * GOAL_ALIGNMENT_WEIGHT
        + components.energy_fit_score * ENERGY_FIT_WEIGHT
        + components.time_fit_score * TIME_FIT_WEIGHT)
        .clamp(0.0, 1.0);

    DecisionResult {
        action_id: action.id.clone(),
        action_title: action.title.clone(),
        score,
        feasible,
        reason: build_reason(&components, feasible),
        components,
    }
}

pub fn rank_actions(
    life_state: &LifeState,
    candidate_actions: &[CandidateAction],
) -> Vec<DecisionResult> {
    let mut ranking = candidate_actions
        .iter()
        .map(|action| evaluate_action(life_state, action))
        .collect::<Vec<_>>();

    ranking.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    ranking
}

pub fn select_next_best_action(ranking: &[DecisionResult]) -> Option<DecisionResult> {
    ranking.iter().find(|result| result.feasible).cloned()
}

fn normalize_attribute(value: f32) -> f32 {
    if value.is_finite() {
        (value / ATTRIBUTE_MAX).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

// A sufficient energy level receives a full fit. Otherwise, the fit declines
// linearly according to the proportion of required energy that is available.
fn calculate_energy_fit(available_energy: f32, required_energy: f32) -> f32 {
    let available = normalize_attribute(available_energy);
    let required = normalize_attribute(required_energy);

    if required == 0.0 || available >= required {
        1.0
    } else {
        available / required
    }
}

fn calculate_time_fit(available_minutes: u32, duration_minutes: u32) -> f32 {
    if duration_minutes <= available_minutes {
        1.0
    } else {
        available_minutes as f32 / duration_minutes as f32
    }
}

fn build_reason(components: &DecisionScoreComponents, feasible: bool) -> String {
    let value_summary = if components.impact_score >= 0.7 && components.goal_alignment_score >= 0.7
    {
        "High impact and strong goal alignment."
    } else {
        "Balanced impact and goal alignment."
    };
    let energy_summary = if components.energy_fit_score >= 1.0 {
        "Compatible with current energy."
    } else {
        "Requires more energy than currently available."
    };
    let time_summary = if feasible {
        "Fits within available time."
    } else {
        "Exceeds available time."
    };

    format!("{value_summary} {energy_summary} {time_summary}")
}

#[cfg(test)]
mod tests {
    use super::{evaluate_action, rank_actions, select_next_best_action};
    use crate::core::{candidate_action::CandidateAction, life_state::LifeState};

    fn life_state(energy: f32, available_minutes: u32) -> LifeState {
        LifeState {
            id: "life-state-test".to_owned(),
            timestamp: "2026-08-12T00:00:00Z".to_owned(),
            energy,
            focus: 5.0,
            stress: 5.0,
            sleep_hours: 7.0,
            available_minutes,
            optional_note: None,
        }
    }

    fn action(
        id: &str,
        impact: f32,
        goal_alignment: f32,
        energy_required: f32,
        duration_minutes: u32,
    ) -> CandidateAction {
        CandidateAction {
            id: id.to_owned(),
            title: id.to_owned(),
            description: "Test action".to_owned(),
            goal_id: Some("test-goal".to_owned()),
            impact,
            urgency: 5.0,
            goal_alignment,
            energy_required,
            duration_minutes,
            status: super::super::candidate_action::ActionStatus::Available,
            created_at: "2026-08-12T00:00:00Z".to_owned(),
            updated_at: "2026-08-12T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn high_impact_and_goal_alignment_outscore_a_weaker_action() {
        let state = life_state(8.0, 120);
        let strong_action = action("strong", 9.0, 9.0, 4.0, 30);
        let weak_action = action("weak", 3.0, 3.0, 4.0, 30);

        let strong_result = evaluate_action(&state, &strong_action);
        let weak_result = evaluate_action(&state, &weak_action);

        assert!(strong_result.score > weak_result.score);
    }

    #[test]
    fn action_exceeding_available_time_is_not_feasible() {
        let state = life_state(8.0, 30);
        let long_action = action("long", 8.0, 8.0, 4.0, 45);

        let result = evaluate_action(&state, &long_action);

        assert!(!result.feasible);
    }

    #[test]
    fn next_best_action_excludes_non_feasible_actions() {
        let state = life_state(8.0, 30);
        let non_feasible_action = action("non-feasible", 10.0, 10.0, 4.0, 60);
        let feasible_action = action("feasible", 5.0, 5.0, 4.0, 30);
        let ranking = rank_actions(&state, &[non_feasible_action, feasible_action]);

        let selected = select_next_best_action(&ranking);

        assert_eq!(
            selected.as_ref().map(|result| result.action_id.as_str()),
            Some("feasible")
        );
    }

    #[test]
    fn final_scores_stay_within_zero_and_one() {
        let state = life_state(7.0, 90);
        let actions = vec![
            action("low", 0.0, 0.0, 0.0, 15),
            action("high", 10.0, 10.0, 10.0, 90),
            action("long", 8.0, 8.0, 8.0, 180),
        ];

        let ranking = rank_actions(&state, &actions);

        assert!(ranking
            .iter()
            .all(|result| (0.0..=1.0).contains(&result.score)));
    }

    #[test]
    fn energy_mismatch_reduces_energy_fit_score() {
        let sufficient_energy_state = life_state(8.0, 120);
        let insufficient_energy_state = life_state(4.0, 120);
        let demanding_action = action("demanding", 7.0, 7.0, 8.0, 30);

        let sufficient_result = evaluate_action(&sufficient_energy_state, &demanding_action);
        let insufficient_result = evaluate_action(&insufficient_energy_state, &demanding_action);

        assert_eq!(sufficient_result.components.energy_fit_score, 1.0);
        assert!(
            insufficient_result.components.energy_fit_score
                < sufficient_result.components.energy_fit_score
        );
    }
}
