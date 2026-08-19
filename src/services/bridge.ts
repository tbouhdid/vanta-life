import { invoke } from "@tauri-apps/api/core";
import type {
  ActionExecution,
  ActionInput,
  ActionItem,
  ActionOutcome,
  AnalyticsSummary,
  BootstrapData,
  AiStatus,
  ChatMessage,
  ChatReply,
  DecisionConsultationResponse,
  CalculateDecisionInput,
  CompleteOnboardingInput,
  DecisionResponse,
  Goal,
  GoalInput,
  HistoryData,
  LifeState,
  LifeStateInput,
  OutcomeInput,
  Profile,
  MemoryAnalysisResponse,
  StartRecommendedActionInput,
  UpdateActionInput,
  UpdateGoalInput,
  UpdateProfileInput,
  UpdateSettingsInput,
  UpdateSettingsResponse,
} from "../types/domain";

export class BridgeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BridgeError";
  }
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "VANTA Life could not complete that request. Please try again.";
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new BridgeError(errorMessage(error));
  }
}

/**
 * The only frontend boundary for Tauri commands. Pages consume domain-shaped
 * data and never import `invoke` directly.
 */
export const vantaApi = {
  getBootstrap: () => call<BootstrapData>("get_bootstrap"),

  completeOnboarding: (input: CompleteOnboardingInput) =>
    call<BootstrapData>("complete_onboarding", { input }),

  saveLifeState: (input: LifeStateInput) =>
    call<LifeState>("save_life_state", { input }),

  createGoal: (input: GoalInput) => call<Goal>("create_goal", { input }),

  updateGoal: (input: UpdateGoalInput) => call<Goal>("update_goal", { input }),

  toggleGoalActive: (id: string, active: boolean) =>
    call<Goal>("toggle_goal_active", { input: { id, active } }),

  completeGoal: (id: string) => call<Goal>("complete_goal", { input: { id } }),

  deleteGoal: (id: string) => call<void>("delete_goal", { input: { id } }),

  createActionItem: (input: ActionInput) =>
    call<ActionItem>("create_action_item", { input }),

  updateAction: (input: UpdateActionInput) =>
    call<ActionItem>("update_action_item", { input }),

  completeActionItem: (id: string) =>
    call<ActionItem>("complete_action_item", { input: { id } }),

  deleteAction: (id: string) => call<void>("delete_action", { input: { id } }),

  archiveAction: (id: string) => call<ActionItem>("archive_action", { input: { id } }),

  calculateDecision: (input: CalculateDecisionInput = {}) =>
    call<DecisionResponse>("calculate_decision", { input }),

  startRecommendedAction: (input: StartRecommendedActionInput) =>
    call<ActionExecution>("start_recommended_action", { input }),

  completeActiveAction: (input: OutcomeInput) =>
    call<ActionOutcome>("complete_active_action", { input }),

  abandonActiveAction: (input: OutcomeInput) =>
    call<ActionOutcome>("abandon_active_action", { input }),

  getHistory: () => call<HistoryData>("get_history"),

  getAnalytics: () => call<AnalyticsSummary>("get_analytics"),

  updateSettings: (input: UpdateSettingsInput) =>
    call<UpdateSettingsResponse>("update_settings", { input }),

  updateProfile: (input: UpdateProfileInput) =>
    call<Profile>("update_profile", { input }),

  getAiStatus: () => call<AiStatus>("get_ai_status"),

  setOpenAiApiKey: (apiKey: string) =>
    call<AiStatus>("set_openai_api_key", { input: { api_key: apiKey } }),

  clearOpenAiApiKey: () => call<AiStatus>("clear_openai_api_key"),

  getChatMessages: () => call<ChatMessage[]>("get_chat_messages"),

  sendChatMessage: (content: string) =>
    call<ChatReply>("send_chat_message", { input: { content } }),

  consultDecisionWithAi: (decisionId: string) =>
    call<DecisionConsultationResponse>("consult_decision_with_ai", {
      input: { decision_id: decisionId },
    }),

  analyzeLatestOutcomeMemory: () =>
    call<MemoryAnalysisResponse>("analyze_latest_outcome_memory"),
};
