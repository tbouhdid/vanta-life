use std::{collections::HashSet, path::Path, sync::Arc};

use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::ai::{
        context::{AiContextSnapshot, ContextBuilder},
        openai::OpenAiProvider,
        provider::{AiError, AiInputMessage, AiProvider, AiProviderRequest, AiStructuredOutput},
        secret_store::{KeyringSecretStore, SecretStore},
        system_instructions::VANTA_SYSTEM_INSTRUCTIONS,
        tools::{execute_read_only_tool, read_only_tool_definitions, ReadOnlyToolContext},
    },
    core::{
        action_execution::{start_action, ActionExecution},
        analytics::{summarize_outcomes, AnalyticsSummary},
        candidate_action::{ActionStatus, CandidateAction},
        chat::{ChatMessage, ChatRole},
        decision_engine,
        decision_result::{DecisionResponse, DecisionResult},
        goal::Goal,
        history::{HistoryDay, HistoryResponse},
        life_state::LifeState,
        memory::{
            repeated_evidence_memory, CandidateMemory, MemoryInference, MemoryObservation,
            StoredMemory,
        },
        outcome::{abandon_action, complete_action, ActionOutcome},
        profile::Profile,
        settings::{LocalSettings, API_CONFIGURATION_NOT_CONFIGURED},
    },
    storage::{SqliteRepository, StoredDecision},
};

#[cfg(test)]
use crate::application::ai::secret_store::InMemorySecretStore;

use super::{
    dto::{
        AbandonExecutionInput, ActionItemInput, AiDecisionReview, AiStatus, BootstrapData,
        CalculateDecisionInput, ChatReply, ChatSendInput, CompleteExecutionInput,
        CompleteOnboardingInput, DecisionConsultationInput, DecisionConsultationResponse,
        EntityIdInput, GoalInput, LifeStateInput, MemoryAnalysisResponse, SetOpenAiApiKeyInput,
        SettingsUpdateResult, StartRecommendedActionInput, ToggleGoalActiveInput,
        UpdateActionItemInput, UpdateGoalInput, UpdateProfileInput, UpdateSettingsInput,
    },
    error::AppError,
};

/// The small application boundary used by Tauri commands. It coordinates
/// domain functions and storage but deliberately keeps the decision engine
/// unaware of SQLite.
pub struct AppService {
    repository: SqliteRepository,
    ai_provider: Arc<dyn AiProvider>,
    secret_store: Arc<dyn SecretStore>,
}

impl AppService {
    pub fn open(database_path: &Path) -> Result<Self, AppError> {
        Ok(Self::with_dependencies(
            SqliteRepository::open(database_path)?,
            Arc::new(OpenAiProvider::default()),
            Arc::new(KeyringSecretStore),
        ))
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, AppError> {
        Ok(Self::with_dependencies(
            SqliteRepository::open_in_memory()?,
            Arc::new(super::ai::provider::UnavailableAiProvider),
            Arc::new(InMemorySecretStore::default()),
        ))
    }

    fn with_dependencies(
        repository: SqliteRepository,
        ai_provider: Arc<dyn AiProvider>,
        secret_store: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            repository,
            ai_provider,
            secret_store,
        }
    }

    pub fn bootstrap(&mut self) -> Result<BootstrapData, AppError> {
        Ok(BootstrapData {
            profile: self.repository.get_profile()?,
            settings: self.repository.get_settings()?,
            life_state: self.repository.latest_life_state()?,
            goals: self.repository.list_goals()?,
            actions: self.repository.list_actions()?,
            // Decisions are intentionally calculated on demand. Returning a
            // stale persisted choice on startup would be misleading after a
            // new check-in or action edit.
            decision: None,
            active_execution: self.repository.get_active_execution()?,
            recent_outcomes: self.repository.recent_outcomes(12)?,
            ai_status: self.ai_status(),
        })
    }

    pub fn complete_onboarding(
        &mut self,
        input: CompleteOnboardingInput,
    ) -> Result<BootstrapData, AppError> {
        if self.repository.get_profile()?.is_some() {
            return Err(AppError::Conflict(
                "Onboarding has already been completed for this local profile.".to_owned(),
            ));
        }

        Profile::validate_name(&input.name).map_err(AppError::Validation)?;
        Goal::validate_fields(&input.goal.title, input.goal.priority)
            .map_err(AppError::Validation)?;

        let timestamp = now();
        let profile = Profile {
            id: new_id(),
            name: input.name.trim().to_owned(),
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
            onboarding_completed: true,
            default_available_minutes: input.life_state.available_minutes,
        };
        let goal = make_goal(input.goal, timestamp.clone());
        let life_state = make_life_state(input.life_state, timestamp.clone())?;
        let mut actions = Vec::with_capacity(input.actions.len());

        for action_input in input.actions {
            if let Some(goal_id) = normalized_optional_id(action_input.goal_id.as_deref()) {
                if goal_id != goal.id {
                    return Err(AppError::Validation(
                        "Onboarding actions can only be linked to the first goal.".to_owned(),
                    ));
                }
            }
            actions.push(make_action(
                action_input,
                timestamp.clone(),
                Some(goal.id.clone()),
            )?);
        }

        let settings = LocalSettings::default();
        self.repository
            .complete_onboarding(&profile, &settings, &goal, &life_state, &actions)?;
        self.bootstrap()
    }

    pub fn update_profile(&mut self, input: UpdateProfileInput) -> Result<Profile, AppError> {
        Profile::validate_name(&input.name).map_err(AppError::Validation)?;
        let profile = self.require_profile()?;
        Ok(self
            .repository
            .update_profile_name(&profile.id, input.name.trim(), &now())?)
    }

    pub fn update_settings(
        &mut self,
        input: UpdateSettingsInput,
    ) -> Result<SettingsUpdateResult, AppError> {
        let profile = self.require_profile()?;
        if let Some(name) = input.name.as_deref() {
            Profile::validate_name(name).map_err(AppError::Validation)?;
        }
        let settings = LocalSettings {
            start_week_day: input.start_week_day.trim().to_lowercase(),
            default_available_minutes: input.default_available_minutes,
            ai_enabled: input.ai_enabled,
            api_configuration_status: self.repository.get_settings()?.api_configuration_status,
            contextual_review_enabled: input.contextual_review_enabled,
            activity_awareness_enabled: input.activity_awareness_enabled,
            notifications_enabled: input.notifications_enabled,
            intervention_cooldown_minutes: input.intervention_cooldown_minutes,
            start_with_windows: input.start_with_windows,
        };
        settings.validate().map_err(AppError::Validation)?;

        let profile = if let Some(name) = input.name.as_deref() {
            self.repository
                .update_profile_name(&profile.id, name.trim(), &now())?
        } else {
            profile
        };
        let settings = self.repository.upsert_settings(&settings)?;
        Ok(SettingsUpdateResult { profile, settings })
    }

    pub fn save_life_state(&mut self, input: LifeStateInput) -> Result<LifeState, AppError> {
        self.require_profile()?;
        let life_state = make_life_state(input, now())?;
        self.repository.insert_life_state(&life_state)?;
        Ok(life_state)
    }

    pub fn create_goal(&mut self, input: GoalInput) -> Result<Goal, AppError> {
        self.require_profile()?;
        Goal::validate_fields(&input.title, input.priority).map_err(AppError::Validation)?;
        let goal = make_goal(input, now());
        self.repository.insert_goal(&goal)?;
        Ok(goal)
    }

    pub fn update_goal(&mut self, input: UpdateGoalInput) -> Result<Goal, AppError> {
        self.require_profile()?;
        Goal::validate_fields(&input.title, input.priority).map_err(AppError::Validation)?;
        let existing = self
            .repository
            .get_goal(&input.id)?
            .ok_or_else(|| AppError::NotFound("Goal not found.".to_owned()))?;
        let goal = Goal {
            id: existing.id,
            title: input.title.trim().to_owned(),
            description: input.description.trim().to_owned(),
            priority: input.priority,
            active: existing.active,
            created_at: existing.created_at,
            updated_at: now(),
            completed_at: existing.completed_at,
        };
        Ok(self.repository.update_goal(&goal)?)
    }

    pub fn toggle_goal_active(&mut self, input: ToggleGoalActiveInput) -> Result<Goal, AppError> {
        self.require_profile()?;
        Ok(self.repository.set_goal_active(&input.id, input.active)?)
    }

    pub fn complete_goal(&mut self, input: EntityIdInput) -> Result<Goal, AppError> {
        self.require_profile()?;
        Ok(self.repository.complete_goal(&input.id, &now())?)
    }

    pub fn delete_goal(&mut self, input: EntityIdInput) -> Result<(), AppError> {
        self.require_profile()?;
        self.repository.delete_goal(&input.id)?;
        Ok(())
    }

    pub fn create_action_item(
        &mut self,
        input: ActionItemInput,
    ) -> Result<CandidateAction, AppError> {
        self.require_profile()?;
        let goal_id = normalized_optional_id(input.goal_id.as_deref());
        self.ensure_goal_exists(goal_id.as_deref())?;
        let action = make_action(input, now(), goal_id)?;
        self.repository.insert_action(&action)?;
        Ok(action)
    }

    pub fn update_action_item(
        &mut self,
        input: UpdateActionItemInput,
    ) -> Result<CandidateAction, AppError> {
        self.require_profile()?;
        let existing = self
            .repository
            .get_action(&input.id)?
            .ok_or_else(|| AppError::NotFound("Action not found.".to_owned()))?;
        let goal_id = normalized_optional_id(input.goal_id.as_deref());
        self.ensure_goal_exists(goal_id.as_deref())?;
        CandidateAction::validate_fields(
            &input.title,
            input.impact,
            input.urgency,
            input.goal_alignment,
            input.energy_required,
            input.duration_minutes,
        )
        .map_err(AppError::Validation)?;

        let action = CandidateAction {
            id: existing.id,
            title: input.title.trim().to_owned(),
            description: input.description.trim().to_owned(),
            goal_id,
            impact: input.impact,
            urgency: input.urgency,
            goal_alignment: input.goal_alignment,
            energy_required: input.energy_required,
            duration_minutes: input.duration_minutes,
            status: existing.status,
            created_at: existing.created_at,
            updated_at: now(),
        };
        Ok(self.repository.update_action(&action)?)
    }

    pub fn complete_action_item(
        &mut self,
        input: EntityIdInput,
    ) -> Result<CandidateAction, AppError> {
        self.require_profile()?;
        if self
            .repository
            .get_active_execution()?
            .is_some_and(|execution| execution.action_id == input.id)
        {
            return Err(AppError::Conflict(
                "Complete or abandon the in-progress action before marking it complete.".to_owned(),
            ));
        }
        Ok(self.repository.complete_action_item(&input.id)?)
    }

    pub fn delete_action(&mut self, input: EntityIdInput) -> Result<(), AppError> {
        self.require_profile()?;
        if self
            .repository
            .get_active_execution()?
            .is_some_and(|execution| execution.action_id == input.id)
        {
            return Err(AppError::Conflict(
                "An in-progress action cannot be deleted.".to_owned(),
            ));
        }
        self.repository.delete_action(&input.id)?;
        Ok(())
    }

    pub fn archive_action(&mut self, input: EntityIdInput) -> Result<CandidateAction, AppError> {
        self.require_profile()?;
        if self
            .repository
            .get_active_execution()?
            .is_some_and(|execution| execution.action_id == input.id)
        {
            return Err(AppError::Conflict(
                "An in-progress action cannot be archived.".to_owned(),
            ));
        }
        Ok(self.repository.archive_action(&input.id)?)
    }

    pub fn calculate_decision(
        &mut self,
        input: CalculateDecisionInput,
    ) -> Result<DecisionResponse, AppError> {
        self.require_profile()?;
        let life_state = self.repository.latest_life_state()?.ok_or_else(|| {
            AppError::Validation("A life-state check-in is required first.".to_owned())
        })?;
        let active_goal_ids = self
            .repository
            .list_goals()?
            .into_iter()
            .filter(|goal| goal.active)
            .map(|goal| goal.id)
            .collect::<HashSet<_>>();
        let excluded_action_ids = input
            .excluded_action_ids
            .into_iter()
            .collect::<HashSet<_>>();
        let candidate_actions = self
            .repository
            .list_actions()?
            .into_iter()
            .filter(|action| action.status == ActionStatus::Available)
            .filter(|action| !excluded_action_ids.contains(&action.id))
            .filter(|action| {
                action
                    .goal_id
                    .as_ref()
                    .is_none_or(|goal_id| active_goal_ids.contains(goal_id))
            })
            .collect::<Vec<_>>();

        let ranking = decision_engine::rank_actions(&life_state, &candidate_actions);
        let next_best_action = decision_engine::select_next_best_action(&ranking);
        let timestamp = now();
        let id = new_id();
        let stored = StoredDecision {
            id: id.clone(),
            timestamp: timestamp.clone(),
            selected_action_id: next_best_action
                .as_ref()
                .map(|action| action.action_id.clone()),
            score: next_best_action.as_ref().map_or(0.0, |action| action.score),
            feasible: next_best_action.is_some(),
            reason: next_best_action
                .as_ref()
                .map(|action| action.reason.clone())
                .unwrap_or_else(|| {
                    "No feasible action is available for the current life state.".to_owned()
                }),
            life_state_snapshot: serde_json::to_string(&life_state)
                .map_err(|error| AppError::Serialization(error.to_string()))?,
            ranking_snapshot: serde_json::to_string(&ranking)
                .map_err(|error| AppError::Serialization(error.to_string()))?,
        };
        self.repository.insert_decision(&stored)?;

        Ok(DecisionResponse {
            id,
            timestamp,
            next_best_action,
            ranking,
        })
    }

    pub fn start_recommended_action(
        &mut self,
        input: Option<StartRecommendedActionInput>,
    ) -> Result<ActionExecution, AppError> {
        self.require_profile()?;
        if self.repository.get_active_execution()?.is_some() {
            return Err(AppError::Conflict(
                "An action is already in progress.".to_owned(),
            ));
        }

        let requested_action_id = input.as_ref().map(|value| value.action_id.clone());
        let decision = match input.and_then(|value| value.decision_id) {
            Some(decision_id) => {
                let stored = self
                    .repository
                    .get_decision(&decision_id)?
                    .ok_or_else(|| AppError::NotFound("Decision not found.".to_owned()))?;
                decision_response_from_stored(stored)?
            }
            None => self.calculate_decision(CalculateDecisionInput::default())?,
        };
        let selected = decision.next_best_action.clone().ok_or_else(|| {
            AppError::Conflict("No feasible action is available to start.".to_owned())
        })?;

        if let Some(action_id) = requested_action_id.as_deref() {
            if action_id != selected.action_id {
                return Err(AppError::Conflict(
                    "The requested action is not the selected recommendation for this decision."
                        .to_owned(),
                ));
            }
        }

        let action = self
            .repository
            .get_action(&selected.action_id)?
            .ok_or_else(|| AppError::NotFound("Recommended action not found.".to_owned()))?;
        if action.status != ActionStatus::Available {
            return Err(AppError::Conflict(
                "The recommended action has already been completed.".to_owned(),
            ));
        }
        if let Some(goal_id) = action.goal_id.as_deref() {
            let goal = self
                .repository
                .get_goal(goal_id)?
                .ok_or_else(|| AppError::NotFound("The action's goal was not found.".to_owned()))?;
            if !goal.active {
                return Err(AppError::Conflict(
                    "The recommended action's goal is not active.".to_owned(),
                ));
            }
        }

        let life_state = self.repository.latest_life_state()?.ok_or_else(|| {
            AppError::Validation("A life-state check-in is required first.".to_owned())
        })?;
        let mut execution = start_action(&selected, life_state.energy, Utc::now())?;
        execution.id = new_id();
        execution.decision_id = Some(decision.id);
        self.repository.insert_execution(&execution)?;
        Ok(execution)
    }

    pub fn active_execution(&self) -> Result<Option<ActionExecution>, AppError> {
        Ok(self.repository.get_active_execution()?)
    }

    pub fn complete_active_action(
        &mut self,
        input: CompleteExecutionInput,
    ) -> Result<ActionOutcome, AppError> {
        let execution = self
            .repository
            .get_active_execution()?
            .ok_or_else(|| AppError::NotFound("There is no action in progress.".to_owned()))?;
        let mut outcome = complete_action(&execution, input, Utc::now())?;
        outcome.id = new_id();
        self.repository
            .finish_execution_and_insert_outcome(&outcome)?;
        self.store_deterministic_memory_from_recent_outcomes()?;
        Ok(outcome)
    }

    pub fn abandon_active_action(
        &mut self,
        input: AbandonExecutionInput,
    ) -> Result<ActionOutcome, AppError> {
        let execution = self
            .repository
            .get_active_execution()?
            .ok_or_else(|| AppError::NotFound("There is no action in progress.".to_owned()))?;
        let mut outcome = abandon_action(&execution, input, Utc::now())?;
        outcome.id = new_id();
        self.repository
            .finish_execution_and_insert_outcome(&outcome)?;
        self.store_deterministic_memory_from_recent_outcomes()?;
        Ok(outcome)
    }

    pub fn recent_outcomes(&self) -> Result<Vec<ActionOutcome>, AppError> {
        Ok(self.repository.recent_outcomes(50)?)
    }

    pub fn analytics(&self) -> Result<AnalyticsSummary, AppError> {
        Ok(summarize_outcomes(&self.repository.recent_outcomes(500)?))
    }

    pub fn history(&self) -> Result<HistoryResponse, AppError> {
        let entries = self.repository.history_entries()?;
        let mut days = Vec::<HistoryDay>::new();

        for entry in entries {
            let date = entry
                .timestamp
                .get(..10)
                .unwrap_or(&entry.timestamp)
                .to_owned();
            if let Some(day) = days.last_mut().filter(|day| day.date == date) {
                day.entries.push(entry);
            } else {
                days.push(HistoryDay {
                    date,
                    entries: vec![entry],
                });
            }
        }

        Ok(HistoryResponse { days })
    }

    pub fn ai_status(&self) -> AiStatus {
        let settings = self.repository.get_settings().unwrap_or_default();
        if !settings.ai_enabled {
            return AiStatus {
                enabled: false,
                configured: self
                    .secret_store
                    .get_openai_api_key()
                    .ok()
                    .flatten()
                    .is_some(),
                available: false,
                message:
                    "AI is disabled in Settings. VANTA's deterministic core remains available."
                        .to_owned(),
            };
        }
        match self.secret_store.get_openai_api_key() {
            Ok(Some(_)) => AiStatus {
                enabled: true,
                configured: true,
                available: true,
                message: "OpenAI is configured.".to_owned(),
            },
            Ok(None) => AiStatus {
                enabled: true,
                configured: false,
                available: false,
                message: "Set an OpenAI API key in Settings to enable AI assistance.".to_owned(),
            },
            Err(_) => AiStatus {
                enabled: true,
                configured: false,
                available: false,
                message: "Secure credential storage is unavailable; AI is disabled safely."
                    .to_owned(),
            },
        }
    }

    pub fn set_openai_api_key(
        &mut self,
        input: SetOpenAiApiKeyInput,
    ) -> Result<AiStatus, AppError> {
        self.require_profile()?;
        let key = input.api_key.trim();
        if key.len() < 12 {
            return Err(AppError::Validation(
                "Enter a valid OpenAI API key.".to_owned(),
            ));
        }
        self.secret_store
            .set_openai_api_key(key)
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let mut settings = self.repository.get_settings()?;
        settings.api_configuration_status = "configured".to_owned();
        self.repository.upsert_settings(&settings)?;
        Ok(self.ai_status())
    }

    pub fn clear_openai_api_key(&mut self) -> Result<AiStatus, AppError> {
        self.require_profile()?;
        self.secret_store
            .delete_openai_api_key()
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let mut settings = self.repository.get_settings()?;
        settings.api_configuration_status = API_CONFIGURATION_NOT_CONFIGURED.to_owned();
        self.repository.upsert_settings(&settings)?;
        Ok(self.ai_status())
    }

    pub fn chat_messages(&self) -> Result<Vec<ChatMessage>, AppError> {
        self.require_profile()?;
        Ok(self.repository.recent_chat_messages(60)?)
    }

    pub fn send_chat_message(&mut self, input: ChatSendInput) -> Result<ChatReply, AppError> {
        self.require_profile()?;
        let content = input.content.trim();
        if content.is_empty() || content.chars().count() > 4_000 {
            return Err(AppError::Validation(
                "Chat messages must contain between 1 and 4000 characters.".to_owned(),
            ));
        }
        let user_message = ChatMessage {
            id: new_id(),
            role: ChatRole::User,
            content: content.to_owned(),
            timestamp: now(),
        };
        self.repository.insert_chat_message(&user_message)?;
        let status = self.ai_status();
        if !status.available {
            return Ok(ChatReply {
                user_message,
                assistant_message: None,
                status,
                tool_calls_used: 0,
            });
        }
        let api_key = match self.secret_store.get_openai_api_key() {
            Ok(Some(key)) => key,
            Ok(None) => {
                return Ok(ChatReply {
                    user_message,
                    assistant_message: None,
                    status: unavailable_status("OpenAI is not configured."),
                    tool_calls_used: 0,
                })
            }
            Err(_) => {
                return Ok(ChatReply {
                    user_message,
                    assistant_message: None,
                    status: unavailable_status("Secure credential storage is unavailable."),
                    tool_calls_used: 0,
                })
            }
        };
        let snapshot = self.ai_context_snapshot(None)?;
        let context = ContextBuilder::build(&snapshot);
        let history = self.repository.recent_chat_messages(12)?;
        let request = AiProviderRequest {
            instructions: format!(
                "{VANTA_SYSTEM_INSTRUCTIONS}\n\nVANTA CONTEXT\n{}",
                context.text
            ),
            input: history
                .into_iter()
                .map(|message| AiInputMessage {
                    role: message.role.as_str().to_owned(),
                    content: message.content,
                })
                .collect(),
            tools: read_only_tool_definitions(),
            structured_output: None,
        };
        let tool_context = ReadOnlyToolContext {
            life_state: snapshot.life_state.clone(),
            active_goals: snapshot.active_goals.clone(),
            available_actions: snapshot.available_actions.clone(),
            next_best_action: snapshot.latest_decision.clone(),
            recent_outcomes: snapshot.recent_outcomes.clone(),
        };
        let mut response = match self.ai_provider.create_response(&api_key, &request) {
            Ok(response) => response,
            Err(error) => {
                return Ok(ChatReply {
                    user_message,
                    assistant_message: None,
                    status: provider_failure_status(error),
                    tool_calls_used: 0,
                })
            }
        };
        let mut tool_calls_used = 0;
        for _ in 0..3 {
            if response.tool_calls.is_empty() {
                break;
            }
            tool_calls_used += response.tool_calls.len();
            let outputs = response
                .tool_calls
                .iter()
                .map(|call| match execute_read_only_tool(&tool_context, call) {
                    Ok(output) => output,
                    Err(error) => super::ai::provider::AiToolOutput {
                        call_id: call.call_id.clone(),
                        output: serde_json::json!({"error": error.to_string()}),
                    },
                })
                .collect::<Vec<_>>();
            response = match self.ai_provider.continue_with_tools(
                &api_key,
                &response.response_id,
                &outputs,
            ) {
                Ok(next) => next,
                Err(error) => {
                    return Ok(ChatReply {
                        user_message,
                        assistant_message: None,
                        status: provider_failure_status(error),
                        tool_calls_used,
                    })
                }
            };
        }
        if response.text.trim().is_empty() {
            return Ok(ChatReply {
                user_message,
                assistant_message: None,
                status: unavailable_status(
                    "AI returned no usable response; the deterministic core is unchanged.",
                ),
                tool_calls_used,
            });
        }
        let assistant_message = ChatMessage {
            id: new_id(),
            role: ChatRole::Assistant,
            content: response.text.trim().to_owned(),
            timestamp: now(),
        };
        self.repository.insert_chat_message(&assistant_message)?;
        Ok(ChatReply {
            user_message,
            assistant_message: Some(assistant_message),
            status: self.ai_status(),
            tool_calls_used,
        })
    }

    pub fn consult_decision_with_ai(
        &mut self,
        input: DecisionConsultationInput,
    ) -> Result<DecisionConsultationResponse, AppError> {
        self.require_profile()?;
        if !self.repository.get_settings()?.contextual_review_enabled {
            return Err(AppError::Validation(
                "Enable contextual decision review in Settings before requesting it.".to_owned(),
            ));
        }
        let stored = self
            .repository
            .get_decision(&input.decision_id)?
            .ok_or_else(|| AppError::NotFound("Decision not found.".to_owned()))?;
        let decision = decision_response_from_stored(stored)?;
        let status = self.ai_status();
        if !status.available {
            return Ok(DecisionConsultationResponse {
                deterministic_next_best_action: decision.next_best_action,
                ai_contextual_note: None,
                status,
            });
        }
        let api_key = match self.secret_store.get_openai_api_key() {
            Ok(Some(key)) => key,
            _ => {
                return Ok(DecisionConsultationResponse {
                    deterministic_next_best_action: decision.next_best_action,
                    ai_contextual_note: None,
                    status: unavailable_status("OpenAI is not configured.".to_owned()),
                })
            }
        };
        let snapshot = self.ai_context_snapshot(Some(decision.clone()))?;
        let request = AiProviderRequest {
            instructions: format!("{VANTA_SYSTEM_INSTRUCTIONS}\n\nReview the deterministic ranking only as an advisory note. Do not make a final decision.\n\nVANTA CONTEXT\n{}", ContextBuilder::build(&snapshot).text),
            input: vec![AiInputMessage { role: "user".to_owned(), content: "Provide a contextual review of the deterministic ranking.".to_owned() }],
            tools: vec![],
            structured_output: Some(decision_review_schema()),
        };
        let response = match self.ai_provider.create_response(&api_key, &request) {
            Ok(response) => response,
            Err(error) => {
                return Ok(DecisionConsultationResponse {
                    deterministic_next_best_action: decision.next_best_action,
                    ai_contextual_note: None,
                    status: provider_failure_status(error),
                })
            }
        };
        let review = match parse_decision_review(&response.text, &decision) {
            Ok(review) => Some(review),
            Err(error) => {
                return Ok(DecisionConsultationResponse {
                    deterministic_next_best_action: decision.next_best_action,
                    ai_contextual_note: None,
                    status: unavailable_status(format!("AI review was not accepted: {error}")),
                })
            }
        };
        Ok(DecisionConsultationResponse {
            deterministic_next_best_action: decision.next_best_action,
            ai_contextual_note: review,
            status: self.ai_status(),
        })
    }

    pub fn analyze_latest_outcome_memory(&mut self) -> Result<MemoryAnalysisResponse, AppError> {
        self.require_profile()?;
        let status = self.ai_status();
        if !status.available {
            return Ok(MemoryAnalysisResponse {
                candidate_memory: None,
                status,
            });
        }
        let recent = self.repository.recent_outcomes(1)?;
        let Some(outcome) = recent.first() else {
            return Ok(MemoryAnalysisResponse {
                candidate_memory: None,
                status: unavailable_status(
                    "Record an outcome before asking for a memory analysis.".to_owned(),
                ),
            });
        };
        let api_key = match self.secret_store.get_openai_api_key() {
            Ok(Some(key)) => key,
            _ => {
                return Ok(MemoryAnalysisResponse {
                    candidate_memory: None,
                    status: unavailable_status("OpenAI is not configured.".to_owned()),
                })
            }
        };
        let snapshot = self.ai_context_snapshot(None)?;
        let request = AiProviderRequest {
            instructions: format!("{VANTA_SYSTEM_INSTRUCTIONS}\n\nGenerate one cautious candidate memory from the outcome below. Keep observations and inferences distinct. This is only a proposal and must not claim it was saved.\n\nVANTA CONTEXT\n{}", ContextBuilder::build(&snapshot).text),
            input: vec![AiInputMessage { role: "user".to_owned(), content: format!("Latest outcome: {} was {} after {} minutes.", outcome.action_title, if outcome.completed { "completed" } else { "abandoned" }, outcome.actual_duration_minutes) }],
            tools: vec![],
            structured_output: Some(memory_schema()),
        };
        let response = match self.ai_provider.create_response(&api_key, &request) {
            Ok(response) => response,
            Err(error) => {
                return Ok(MemoryAnalysisResponse {
                    candidate_memory: None,
                    status: provider_failure_status(error),
                })
            }
        };
        let candidate_memory = match parse_candidate_memory(&response.text) {
            Ok(candidate) => Some(candidate),
            Err(error) => {
                return Ok(MemoryAnalysisResponse {
                    candidate_memory: None,
                    status: unavailable_status(format!(
                        "AI memory proposal was not accepted: {error}"
                    )),
                })
            }
        };
        Ok(MemoryAnalysisResponse {
            candidate_memory,
            status: self.ai_status(),
        })
    }

    fn ai_context_snapshot(
        &self,
        decision: Option<DecisionResponse>,
    ) -> Result<AiContextSnapshot, AppError> {
        let latest_decision = match decision {
            Some(decision) => Some(decision),
            None => self
                .repository
                .latest_decision()?
                .map(decision_response_from_stored)
                .transpose()?,
        };
        Ok(AiContextSnapshot {
            profile: self.repository.get_profile()?,
            life_state: self.repository.latest_life_state()?,
            active_goals: self
                .repository
                .list_goals()?
                .into_iter()
                .filter(|goal| goal.active && goal.completed_at.is_none())
                .collect(),
            active_execution: self.repository.get_active_execution()?,
            available_actions: self
                .repository
                .list_actions()?
                .into_iter()
                .filter(|action| action.status == ActionStatus::Available)
                .collect(),
            latest_decision,
            recent_outcomes: self.repository.recent_outcomes(6)?,
            memories: self.repository.relevant_memories(6)?,
        })
    }

    fn require_profile(&self) -> Result<Profile, AppError> {
        self.repository.get_profile()?.ok_or_else(|| {
            AppError::Validation("Complete onboarding before using VANTA Life.".to_owned())
        })
    }

    fn ensure_goal_exists(&self, goal_id: Option<&str>) -> Result<(), AppError> {
        if let Some(goal_id) = goal_id {
            if self.repository.get_goal(goal_id)?.is_none() {
                return Err(AppError::NotFound("Goal not found.".to_owned()));
            }
        }
        Ok(())
    }

    fn store_deterministic_memory_from_recent_outcomes(&mut self) -> Result<(), AppError> {
        let outcomes = self.repository.recent_outcomes(50)?;
        let Some(candidate) = repeated_evidence_memory(&outcomes) else {
            return Ok(());
        };
        let memory = StoredMemory {
            id: new_id(),
            category: "observed_pattern".to_owned(),
            statement: candidate.proposed_content,
            source: candidate.observation.source,
            confidence: candidate.confidence,
            importance: candidate.importance,
            created_at: now(),
            last_used_at: None,
            active: true,
        };
        self.repository.insert_memory_if_new(&memory)?;
        Ok(())
    }
}

fn make_life_state(input: LifeStateInput, timestamp: String) -> Result<LifeState, AppError> {
    let life_state = LifeState {
        id: new_id(),
        timestamp,
        energy: input.energy,
        focus: input.focus,
        stress: input.stress,
        sleep_hours: input.sleep_hours,
        available_minutes: input.available_minutes,
        optional_note: normalized_optional_text(input.optional_note.as_deref()),
    };
    life_state.validate().map_err(AppError::Validation)?;
    Ok(life_state)
}

fn make_goal(input: GoalInput, timestamp: String) -> Goal {
    Goal {
        id: new_id(),
        title: input.title.trim().to_owned(),
        description: input.description.trim().to_owned(),
        priority: input.priority,
        active: true,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        completed_at: None,
    }
}

fn make_action(
    input: ActionItemInput,
    timestamp: String,
    default_goal_id: Option<String>,
) -> Result<CandidateAction, AppError> {
    CandidateAction::validate_fields(
        &input.title,
        input.impact,
        input.urgency,
        input.goal_alignment,
        input.energy_required,
        input.duration_minutes,
    )
    .map_err(AppError::Validation)?;

    Ok(CandidateAction {
        id: new_id(),
        title: input.title.trim().to_owned(),
        description: input.description.trim().to_owned(),
        goal_id: normalized_optional_id(input.goal_id.as_deref()).or(default_goal_id),
        impact: input.impact,
        urgency: input.urgency,
        goal_alignment: input.goal_alignment,
        energy_required: input.energy_required,
        duration_minutes: input.duration_minutes,
        status: ActionStatus::Available,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    })
}

fn decision_response_from_stored(stored: StoredDecision) -> Result<DecisionResponse, AppError> {
    let ranking =
        serde_json::from_str::<Vec<DecisionResult>>(&stored.ranking_snapshot).map_err(|error| {
            AppError::Serialization(format!("Stored decision ranking is invalid: {error}"))
        })?;
    let next_best_action = stored.selected_action_id.as_ref().and_then(|selected_id| {
        ranking
            .iter()
            .find(|result| &result.action_id == selected_id)
            .cloned()
    });
    Ok(DecisionResponse {
        id: stored.id,
        timestamp: stored.timestamp,
        next_best_action,
        ranking,
    })
}

fn normalized_optional_id(value: Option<&str>) -> Option<String> {
    value.and_then(|id| {
        let id = id.trim();
        (!id.is_empty()).then(|| id.to_owned())
    })
}

fn normalized_optional_text(value: Option<&str>) -> Option<String> {
    value.and_then(|text| {
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_owned())
    })
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn unavailable_status(message: impl Into<String>) -> AiStatus {
    AiStatus {
        enabled: true,
        configured: false,
        available: false,
        message: message.into(),
    }
}

fn provider_failure_status(error: AiError) -> AiStatus {
    AiStatus {
        enabled: true,
        configured: true,
        available: false,
        message: format!("AI could not respond: {error}"),
    }
}

fn decision_review_schema() -> AiStructuredOutput {
    AiStructuredOutput {
        name: "vanta_decision_review".to_owned(),
        schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "ranking_override_recommended": {"type": "boolean"},
                "preferred_action_id": {"type": ["string", "null"]},
                "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                "contextual_factors": {"type": "array", "items": {"type": "string"}, "maxItems": 6},
                "explanation": {"type": "string", "maxLength": 800}
            },
            "required": ["ranking_override_recommended", "preferred_action_id", "confidence", "contextual_factors", "explanation"]
        }),
    }
}

fn memory_schema() -> AiStructuredOutput {
    AiStructuredOutput {
        name: "vanta_candidate_memory".to_owned(),
        schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "observation": {"type": "string", "maxLength": 500},
                "inference": {"type": ["string", "null"], "maxLength": 500},
                "proposed_content": {"type": "string", "maxLength": 500},
                "importance": {"type": "number", "minimum": 0, "maximum": 10},
                "confidence": {"type": "number", "minimum": 0, "maximum": 1}
            },
            "required": ["observation", "inference", "proposed_content", "importance", "confidence"]
        }),
    }
}

fn parse_decision_review(
    text: &str,
    decision: &DecisionResponse,
) -> Result<AiDecisionReview, String> {
    let mut review = serde_json::from_str::<AiDecisionReview>(text)
        .map_err(|error| format!("invalid structured response: {error}"))?;
    if !review.confidence.is_finite() || !(0.0..=1.0).contains(&review.confidence) {
        return Err("confidence must be between 0 and 1".to_owned());
    }
    if review.explanation.trim().is_empty() {
        return Err("explanation is required".to_owned());
    }
    if let Some(action_id) = review.preferred_action_id.as_ref() {
        if !decision
            .ranking
            .iter()
            .any(|item| &item.action_id == action_id)
        {
            return Err(
                "preferred_action_id is not present in the deterministic ranking".to_owned(),
            );
        }
    }
    review.contextual_factors.truncate(6);
    Ok(review)
}

#[derive(serde::Deserialize)]
struct CandidateMemoryWire {
    observation: String,
    inference: Option<String>,
    proposed_content: String,
    importance: f32,
    confidence: f32,
}

fn parse_candidate_memory(text: &str) -> Result<CandidateMemory, String> {
    let wire = serde_json::from_str::<CandidateMemoryWire>(text)
        .map_err(|error| format!("invalid structured response: {error}"))?;
    if wire.observation.trim().is_empty() || wire.proposed_content.trim().is_empty() {
        return Err("observation and proposed content are required".to_owned());
    }
    if !wire.importance.is_finite()
        || !(0.0..=10.0).contains(&wire.importance)
        || !wire.confidence.is_finite()
        || !(0.0..=1.0).contains(&wire.confidence)
    {
        return Err("importance/confidence are outside their allowed range".to_owned());
    }
    Ok(CandidateMemory {
        observation: MemoryObservation {
            content: wire.observation,
            source: "outcome_analysis".to_owned(),
        },
        inference: wire
            .inference
            .filter(|value| !value.trim().is_empty())
            .map(|content| MemoryInference {
                content,
                confidence: wire.confidence,
            }),
        proposed_content: wire.proposed_content,
        importance: wire.importance,
        confidence: wire.confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ai::{provider::AiProviderResponse, secret_store::SecretStore};
    use std::sync::Arc;
    use uuid::Uuid;

    struct MockAiProvider {
        response: AiProviderResponse,
    }

    impl AiProvider for MockAiProvider {
        fn create_response(
            &self,
            _api_key: &str,
            _request: &AiProviderRequest,
        ) -> Result<AiProviderResponse, AiError> {
            Ok(self.response.clone())
        }

        fn continue_with_tools(
            &self,
            _api_key: &str,
            _previous_response_id: &str,
            _outputs: &[super::super::ai::provider::AiToolOutput],
        ) -> Result<AiProviderResponse, AiError> {
            Err(AiError::Provider(
                "No follow-up response was configured for this test.".to_owned(),
            ))
        }
    }

    fn service_with_ai_response(text: &str) -> AppService {
        let secrets = Arc::new(InMemorySecretStore::default());
        secrets
            .set_openai_api_key("test-openai-key-not-real")
            .expect("test secret should be stored");
        let mut service = AppService::with_dependencies(
            SqliteRepository::open_in_memory().expect("in-memory storage should open"),
            Arc::new(MockAiProvider {
                response: AiProviderResponse {
                    response_id: "resp_test".to_owned(),
                    text: text.to_owned(),
                    tool_calls: vec![],
                },
            }),
            secrets,
        );
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");
        service
            .update_settings(UpdateSettingsInput {
                name: None,
                start_week_day: "monday".to_owned(),
                default_available_minutes: 120,
                ai_enabled: true,
                contextual_review_enabled: true,
                activity_awareness_enabled: false,
                notifications_enabled: false,
                intervention_cooldown_minutes: 90,
                start_with_windows: false,
            })
            .expect("AI should be enabled for this test");
        service
    }

    fn onboarding_input() -> CompleteOnboardingInput {
        CompleteOnboardingInput {
            name: "Ada".to_owned(),
            goal: GoalInput {
                title: "Ship alpha".to_owned(),
                description: "Make VANTA useful.".to_owned(),
                priority: 9.0,
            },
            life_state: LifeStateInput {
                energy: 7.0,
                focus: 7.0,
                stress: 3.0,
                sleep_hours: 7.5,
                available_minutes: 120,
                optional_note: None,
            },
            actions: vec![ActionItemInput {
                title: "Implement persistence".to_owned(),
                description: "Store user data locally.".to_owned(),
                goal_id: None,
                impact: 9.0,
                urgency: 8.0,
                goal_alignment: 10.0,
                energy_required: 5.0,
                duration_minutes: 60,
            }],
        }
    }

    #[test]
    fn goal_and_action_crud_persist_through_the_service() {
        let mut service = AppService::open_in_memory().expect("in-memory storage should open");
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");

        let goal = service
            .create_goal(GoalInput {
                title: "Health".to_owned(),
                description: "Move daily.".to_owned(),
                priority: 6.0,
            })
            .expect("goal should be created");
        let action = service
            .create_action_item(ActionItemInput {
                title: "Walk".to_owned(),
                description: "Twenty minutes.".to_owned(),
                goal_id: Some(goal.id.clone()),
                impact: 6.0,
                urgency: 4.0,
                goal_alignment: 8.0,
                energy_required: 2.0,
                duration_minutes: 20,
            })
            .expect("action should be created");

        let updated_goal = service
            .update_goal(UpdateGoalInput {
                id: goal.id.clone(),
                title: "Health first".to_owned(),
                description: "Move daily.".to_owned(),
                priority: 7.0,
            })
            .expect("goal should update");
        assert_eq!(updated_goal.title, "Health first");

        let completed = service
            .complete_action_item(EntityIdInput {
                id: action.id.clone(),
            })
            .expect("action should complete");
        assert_eq!(completed.status, ActionStatus::Completed);

        service
            .delete_action(EntityIdInput { id: action.id })
            .expect("action should delete");
        service
            .delete_goal(EntityIdInput { id: goal.id })
            .expect("goal should delete");
    }

    #[test]
    fn calculated_decision_is_persisted_and_can_start_an_execution() {
        let mut service = AppService::open_in_memory().expect("in-memory storage should open");
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");

        let decision = service
            .calculate_decision(CalculateDecisionInput::default())
            .expect("decision should be calculated");
        let action_id = decision
            .next_best_action
            .as_ref()
            .expect("seed action should be feasible")
            .action_id
            .clone();
        assert!(service
            .repository
            .get_decision(&decision.id)
            .expect("decision lookup should succeed")
            .is_some());
        let execution = service
            .start_recommended_action(Some(StartRecommendedActionInput {
                action_id,
                decision_id: Some(decision.id),
            }))
            .expect("recommended action should start");
        assert_eq!(execution.status.as_str(), "in_progress");
        assert!(service
            .active_execution()
            .expect("active execution should load")
            .is_some());
    }

    #[test]
    fn completing_an_execution_persists_its_outcome_and_completes_the_action() {
        let mut service = AppService::open_in_memory().expect("in-memory storage should open");
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");
        let decision = service
            .calculate_decision(CalculateDecisionInput::default())
            .expect("decision should calculate");
        let action_id = decision
            .next_best_action
            .as_ref()
            .expect("seed action should be feasible")
            .action_id
            .clone();
        service
            .start_recommended_action(Some(StartRecommendedActionInput {
                action_id: action_id.clone(),
                decision_id: Some(decision.id),
            }))
            .expect("execution should start");

        let outcome = service
            .complete_active_action(CompleteExecutionInput {
                result_quality: 8.0,
                energy_after: 5.0,
                difficulty: 4.0,
            })
            .expect("outcome should persist");
        assert!(outcome.completed);
        assert_eq!(outcome.energy_after, Some(5.0));
        assert_eq!(outcome.difficulty, Some(4.0));
        assert!(service
            .active_execution()
            .expect("active execution should load")
            .is_none());
        assert_eq!(
            service
                .repository
                .get_action(&action_id)
                .expect("action lookup should succeed")
                .expect("action should still exist")
                .status,
            ActionStatus::Completed
        );
        assert_eq!(
            service
                .recent_outcomes()
                .expect("outcomes should load")
                .len(),
            1
        );
    }

    #[test]
    fn chat_falls_back_without_an_available_provider_and_persists_the_user_message() {
        let mut service = AppService::open_in_memory().expect("in-memory storage should open");
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");

        let reply = service
            .send_chat_message(ChatSendInput {
                content: "What should I do now?".to_owned(),
            })
            .expect("offline chat fallback should not fail");

        assert!(reply.assistant_message.is_none());
        assert!(!reply.status.available);
        assert_eq!(
            service.chat_messages().expect("messages should load").len(),
            1
        );
    }

    #[test]
    fn invalid_structured_decision_review_is_rejected() {
        let mut service = AppService::open_in_memory().expect("in-memory storage should open");
        service
            .complete_onboarding(onboarding_input())
            .expect("onboarding should persist");
        let decision = service
            .calculate_decision(CalculateDecisionInput::default())
            .expect("decision should calculate");

        assert!(parse_decision_review("{\"confidence\": 2}", &decision).is_err());
        assert!(parse_decision_review(
            "{\"ranking_override_recommended\":true,\"preferred_action_id\":\"not-in-ranking\",\"confidence\":0.8,\"contextual_factors\":[],\"explanation\":\"Review\"}",
            &decision,
        ).is_err());
    }

    #[test]
    fn ai_override_is_advisory_and_never_changes_the_deterministic_choice() {
        let mut service = service_with_ai_response(
            "{\"ranking_override_recommended\":true,\"preferred_action_id\":null,\"confidence\":0.82,\"contextual_factors\":[\"User reported limited energy\"],\"explanation\":\"Consider a shorter task if the user prefers, but keep the deterministic result.\"}",
        );
        let decision = service
            .calculate_decision(CalculateDecisionInput::default())
            .expect("decision should calculate");
        let expected = decision.next_best_action.clone();

        let consultation = service
            .consult_decision_with_ai(DecisionConsultationInput {
                decision_id: decision.id,
            })
            .expect("consultation should be returned");

        assert!(
            consultation
                .ai_contextual_note
                .expect("review should be accepted")
                .ranking_override_recommended
        );
        assert_eq!(consultation.deterministic_next_best_action, expected);
    }

    #[test]
    fn persisted_bootstrap_data_survives_reopening_a_file_database() {
        let path = test_database_path();
        {
            let mut service = AppService::open(&path).expect("file database should open");
            service
                .complete_onboarding(onboarding_input())
                .expect("onboarding should persist");
            service
                .save_life_state(LifeStateInput {
                    energy: 5.0,
                    focus: 4.0,
                    stress: 6.0,
                    sleep_hours: 6.5,
                    available_minutes: 45,
                    optional_note: None,
                })
                .expect("check-in should persist");
        }

        let mut reopened = AppService::open(&path).expect("file database should reopen");
        let bootstrap = reopened
            .bootstrap()
            .expect("bootstrap should load persisted data");
        assert_eq!(bootstrap.profile.expect("profile should exist").name, "Ada");
        assert_eq!(bootstrap.goals.len(), 1);
        assert_eq!(bootstrap.actions.len(), 1);
        assert_eq!(
            bootstrap
                .life_state
                .expect("life state should exist")
                .available_minutes,
            45
        );
        drop(reopened);
        remove_test_database(&path);
    }

    #[test]
    fn active_execution_is_loaded_after_reopening_a_file_database() {
        let path = test_database_path();
        let execution_id;
        let started_at;
        {
            let mut service = AppService::open(&path).expect("file database should open");
            service
                .complete_onboarding(onboarding_input())
                .expect("onboarding should persist");
            let decision = service
                .calculate_decision(CalculateDecisionInput::default())
                .expect("decision should calculate");
            let action_id = decision
                .next_best_action
                .as_ref()
                .expect("seed action should be feasible")
                .action_id
                .clone();
            let execution = service
                .start_recommended_action(Some(StartRecommendedActionInput {
                    action_id,
                    decision_id: Some(decision.id),
                }))
                .expect("execution should start");
            execution_id = execution.id;
            started_at = execution.started_at;
        }

        let reopened = AppService::open(&path).expect("file database should reopen");
        let execution = reopened
            .active_execution()
            .expect("active execution should load")
            .expect("execution should survive restart");
        assert_eq!(execution.id, execution_id);
        assert_eq!(execution.started_at, started_at);
        assert_eq!(
            execution.status,
            crate::core::action_execution::ExecutionStatus::InProgress
        );
        drop(reopened);
        remove_test_database(&path);
    }

    fn test_database_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vanta-life-test-{}.sqlite", Uuid::new_v4()))
    }

    fn remove_test_database(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }
}
