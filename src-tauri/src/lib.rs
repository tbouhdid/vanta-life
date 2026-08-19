use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

mod application;
mod core;
mod storage;

use application::{
    dto::{
        AbandonExecutionInput, ActionItemInput, AiStatus, BootstrapData, CalculateDecisionInput,
        ChatReply, ChatSendInput, CompleteExecutionInput, CompleteOnboardingInput,
        DecisionConsultationInput, DecisionConsultationResponse, EntityIdInput, GoalInput,
        LifeStateInput, MemoryAnalysisResponse, SetOpenAiApiKeyInput, SettingsUpdateResult,
        StartRecommendedActionInput, ToggleGoalActiveInput, UpdateActionItemInput, UpdateGoalInput,
        UpdateProfileInput, UpdateSettingsInput,
    },
    error::AppError,
    services::AppService,
};
use core::{
    action_execution::ActionExecution, analytics::AnalyticsSummary,
    candidate_action::CandidateAction, chat::ChatMessage, decision_result::DecisionResponse,
    goal::Goal, history::HistoryResponse, life_state::LifeState, outcome::ActionOutcome,
    profile::Profile,
};

struct AppState {
    service: Mutex<AppService>,
}

#[derive(Serialize)]
struct CoreStatus {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

fn invoke_service<T>(
    state: State<'_, AppState>,
    operation: impl FnOnce(&mut AppService) -> Result<T, AppError>,
) -> Result<T, String> {
    let mut service = state
        .service
        .lock()
        .map_err(|_| "VANTA's local storage is temporarily unavailable.".to_owned())?;
    operation(&mut service).map_err(|error| error.to_string())
}

#[tauri::command]
fn core_status() -> CoreStatus {
    CoreStatus {
        status: "online",
        service: "VANTA Core",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[tauri::command]
fn get_bootstrap(state: State<'_, AppState>) -> Result<BootstrapData, String> {
    invoke_service(state, |service| service.bootstrap())
}

#[tauri::command]
fn complete_onboarding(
    input: CompleteOnboardingInput,
    state: State<'_, AppState>,
) -> Result<BootstrapData, String> {
    invoke_service(state, |service| service.complete_onboarding(input))
}

#[tauri::command]
fn update_profile(
    input: UpdateProfileInput,
    state: State<'_, AppState>,
) -> Result<Profile, String> {
    invoke_service(state, |service| service.update_profile(input))
}

#[tauri::command]
fn update_settings(
    input: UpdateSettingsInput,
    state: State<'_, AppState>,
) -> Result<SettingsUpdateResult, String> {
    invoke_service(state, |service| service.update_settings(input))
}

#[tauri::command]
fn save_life_state(input: LifeStateInput, state: State<'_, AppState>) -> Result<LifeState, String> {
    invoke_service(state, |service| service.save_life_state(input))
}

#[tauri::command]
fn create_goal(input: GoalInput, state: State<'_, AppState>) -> Result<Goal, String> {
    invoke_service(state, |service| service.create_goal(input))
}

#[tauri::command]
fn update_goal(input: UpdateGoalInput, state: State<'_, AppState>) -> Result<Goal, String> {
    invoke_service(state, |service| service.update_goal(input))
}

#[tauri::command]
fn toggle_goal_active(
    input: ToggleGoalActiveInput,
    state: State<'_, AppState>,
) -> Result<Goal, String> {
    invoke_service(state, |service| service.toggle_goal_active(input))
}

#[tauri::command]
fn complete_goal(input: EntityIdInput, state: State<'_, AppState>) -> Result<Goal, String> {
    invoke_service(state, |service| service.complete_goal(input))
}

#[tauri::command]
fn delete_goal(input: EntityIdInput, state: State<'_, AppState>) -> Result<(), String> {
    invoke_service(state, |service| service.delete_goal(input))
}

#[tauri::command]
fn create_action_item(
    input: ActionItemInput,
    state: State<'_, AppState>,
) -> Result<CandidateAction, String> {
    invoke_service(state, |service| service.create_action_item(input))
}

#[tauri::command]
fn update_action_item(
    input: UpdateActionItemInput,
    state: State<'_, AppState>,
) -> Result<CandidateAction, String> {
    invoke_service(state, |service| service.update_action_item(input))
}

#[tauri::command]
fn complete_action_item(
    input: EntityIdInput,
    state: State<'_, AppState>,
) -> Result<CandidateAction, String> {
    invoke_service(state, |service| service.complete_action_item(input))
}

#[tauri::command]
fn delete_action(input: EntityIdInput, state: State<'_, AppState>) -> Result<(), String> {
    invoke_service(state, |service| service.delete_action(input))
}

#[tauri::command]
fn archive_action(
    input: EntityIdInput,
    state: State<'_, AppState>,
) -> Result<CandidateAction, String> {
    invoke_service(state, |service| service.archive_action(input))
}

#[tauri::command]
fn calculate_decision(
    input: Option<CalculateDecisionInput>,
    state: State<'_, AppState>,
) -> Result<DecisionResponse, String> {
    invoke_service(state, |service| {
        service.calculate_decision(input.unwrap_or_default())
    })
}

#[tauri::command]
fn start_recommended_action(
    input: Option<StartRecommendedActionInput>,
    state: State<'_, AppState>,
) -> Result<ActionExecution, String> {
    invoke_service(state, |service| service.start_recommended_action(input))
}

#[tauri::command]
fn get_active_execution(state: State<'_, AppState>) -> Result<Option<ActionExecution>, String> {
    invoke_service(state, |service| service.active_execution())
}

#[tauri::command]
fn complete_active_action(
    input: CompleteExecutionInput,
    state: State<'_, AppState>,
) -> Result<ActionOutcome, String> {
    invoke_service(state, |service| service.complete_active_action(input))
}

#[tauri::command]
fn abandon_active_action(
    input: AbandonExecutionInput,
    state: State<'_, AppState>,
) -> Result<ActionOutcome, String> {
    invoke_service(state, |service| service.abandon_active_action(input))
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> Result<HistoryResponse, String> {
    invoke_service(state, |service| service.history())
}

#[tauri::command]
fn get_analytics(state: State<'_, AppState>) -> Result<AnalyticsSummary, String> {
    invoke_service(state, |service| service.analytics())
}

#[tauri::command]
fn get_ai_status(state: State<'_, AppState>) -> Result<AiStatus, String> {
    invoke_service(state, |service| Ok(service.ai_status()))
}

#[tauri::command]
fn set_openai_api_key(
    input: SetOpenAiApiKeyInput,
    state: State<'_, AppState>,
) -> Result<AiStatus, String> {
    invoke_service(state, |service| service.set_openai_api_key(input))
}

#[tauri::command]
fn clear_openai_api_key(state: State<'_, AppState>) -> Result<AiStatus, String> {
    invoke_service(state, |service| service.clear_openai_api_key())
}

#[tauri::command]
fn get_chat_messages(state: State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
    invoke_service(state, |service| service.chat_messages())
}

#[tauri::command]
fn send_chat_message(
    input: ChatSendInput,
    state: State<'_, AppState>,
) -> Result<ChatReply, String> {
    invoke_service(state, |service| service.send_chat_message(input))
}

#[tauri::command]
fn consult_decision_with_ai(
    input: DecisionConsultationInput,
    state: State<'_, AppState>,
) -> Result<DecisionConsultationResponse, String> {
    invoke_service(state, |service| service.consult_decision_with_ai(input))
}

#[tauri::command]
fn analyze_latest_outcome_memory(
    state: State<'_, AppState>,
) -> Result<MemoryAnalysisResponse, String> {
    invoke_service(state, |service| service.analyze_latest_outcome_memory())
}

// Compatibility commands retained for the initial prototype bridge. They now
// read persistent data and no longer use mocks or volatile session state.
#[tauri::command]
fn get_life_state(state: State<'_, AppState>) -> Result<Option<LifeState>, String> {
    invoke_service(state, |service| Ok(service.bootstrap()?.life_state))
}

#[tauri::command]
fn get_goals(state: State<'_, AppState>) -> Result<Vec<Goal>, String> {
    invoke_service(state, |service| Ok(service.bootstrap()?.goals))
}

#[tauri::command]
fn get_candidate_actions(state: State<'_, AppState>) -> Result<Vec<CandidateAction>, String> {
    invoke_service(state, |service| Ok(service.bootstrap()?.actions))
}

#[tauri::command]
fn get_decision(state: State<'_, AppState>) -> Result<DecisionResponse, String> {
    invoke_service(state, |service| {
        service.calculate_decision(CalculateDecisionInput::default())
    })
}

#[tauri::command]
fn get_session_outcomes(state: State<'_, AppState>) -> Result<Vec<ActionOutcome>, String> {
    invoke_service(state, |service| service.recent_outcomes())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let database_path = data_dir.join("vanta-life.sqlite3");
            let service = AppService::open(&database_path)?;
            app.manage(AppState {
                service: Mutex::new(service),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_status,
            get_bootstrap,
            complete_onboarding,
            update_profile,
            update_settings,
            save_life_state,
            create_goal,
            update_goal,
            toggle_goal_active,
            complete_goal,
            delete_goal,
            create_action_item,
            update_action_item,
            complete_action_item,
            delete_action,
            archive_action,
            calculate_decision,
            start_recommended_action,
            get_active_execution,
            complete_active_action,
            abandon_active_action,
            get_history,
            get_analytics,
            get_ai_status,
            set_openai_api_key,
            clear_openai_api_key,
            get_chat_messages,
            send_chat_message,
            consult_decision_with_ai,
            analyze_latest_outcome_memory,
            get_life_state,
            get_goals,
            get_candidate_actions,
            get_decision,
            get_session_outcomes
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("VANTA Life could not start: {error}");
    }
}
