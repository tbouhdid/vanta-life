export type IsoTimestamp = string;

export type Profile = {
  id: string;
  name: string;
  created_at: IsoTimestamp;
  updated_at: IsoTimestamp;
  onboarding_completed: boolean;
  default_available_minutes: number;
};

export type LifeState = {
  id?: string;
  timestamp: IsoTimestamp;
  energy: number;
  focus: number;
  stress: number;
  sleep_hours: number;
  available_minutes: number;
  optional_note?: string | null;
};

export type LifeStateInput = Omit<LifeState, "id" | "timestamp">;

export type Goal = {
  id: string;
  title: string;
  description: string;
  priority: number;
  active: boolean;
  created_at: IsoTimestamp;
  updated_at: IsoTimestamp;
  completed_at: IsoTimestamp | null;
};

export type GoalInput = {
  title: string;
  description: string;
  priority: number;
};

export type UpdateGoalInput = GoalInput & {
  id: string;
};

/**
 * Status values intentionally remain open-ended: the persistence layer owns
 * the vocabulary, while the UI understands the common Alpha states.
 */
export type ActionStatus = "available" | "in_progress" | "completed" | "archived";

export type ActionItem = {
  id: string;
  title: string;
  description: string;
  goal_id: string | null;
  impact: number;
  urgency: number;
  goal_alignment: number;
  energy_required: number;
  duration_minutes: number;
  status: ActionStatus;
  created_at: IsoTimestamp;
  updated_at: IsoTimestamp;
  completed_at?: IsoTimestamp | null;
};

export type ActionInput = {
  title: string;
  description: string;
  goal_id: string | null;
  impact: number;
  urgency: number;
  goal_alignment: number;
  energy_required: number;
  duration_minutes: number;
};

export type UpdateActionInput = ActionInput & {
  id: string;
};

export type DecisionScoreComponents = {
  impact_score: number;
  urgency_score: number;
  goal_alignment_score: number;
  energy_fit_score: number;
  time_fit_score: number;
};

export type DecisionResult = {
  action_id: string;
  action_title: string;
  score: number;
  feasible: boolean;
  reason: string;
  components?: DecisionScoreComponents;
};

export type DecisionRecord = {
  id: string;
  timestamp: IsoTimestamp;
  selected_action_id: string | null;
  score: number | null;
  feasible: boolean;
  reason: string;
  life_state_snapshot?: unknown;
  ranking_snapshot?: unknown;
};

/** The decision command returns the persisted decision together with its ranking. */
export type DecisionResponse = {
  id?: string;
  decision_id?: string;
  timestamp?: IsoTimestamp;
  decision?: DecisionRecord;
  next_best_action: DecisionResult | null;
  ranking: DecisionResult[];
};

export type ExecutionStatus = "in_progress" | "completed" | "abandoned" | string;

export type ActionExecution = {
  id?: string;
  execution_id?: string;
  action_id: string;
  action_title?: string;
  decision_id?: string | null;
  decision_score?: number | null;
  started_at: IsoTimestamp;
  ended_at?: IsoTimestamp | null;
  energy_before: number;
  status: ExecutionStatus;
};

export type OutcomeInput = {
  result_quality?: number | null;
  energy_after?: number | null;
  difficulty?: number | null;
};

export type ActionOutcome = {
  id?: string;
  execution_id?: string;
  action_id?: string;
  action_title?: string;
  decision_score?: number | null;
  recommended?: boolean;
  accepted?: boolean;
  started_at?: IsoTimestamp;
  ended_at?: IsoTimestamp;
  actual_duration_minutes: number;
  completed: boolean;
  abandoned: boolean;
  result_quality: number | null;
  energy_before: number;
  energy_after: number | null;
  difficulty: number | null;
  created_at?: IsoTimestamp;
};

export type LocalSettings = {
  start_week_day: string;
  default_available_minutes: number;
  ai_enabled: boolean;
  api_configuration_status: string;
  contextual_review_enabled: boolean;
  activity_awareness_enabled: boolean;
  notifications_enabled: boolean;
  intervention_cooldown_minutes: number;
  start_with_windows: boolean;
};

export type AiStatus = {
  enabled: boolean;
  configured: boolean;
  available: boolean;
  message: string;
};

export type ChatRole = "user" | "assistant" | "system";

export type ChatMessage = {
  id: string;
  role: ChatRole;
  content: string;
  timestamp: IsoTimestamp;
};

export type ChatReply = {
  user_message: ChatMessage;
  assistant_message: ChatMessage | null;
  status: AiStatus;
  tool_calls_used: number;
};

export type AiDecisionReview = {
  ranking_override_recommended: boolean;
  preferred_action_id: string | null;
  confidence: number;
  contextual_factors: string[];
  explanation: string;
};

export type DecisionConsultationResponse = {
  deterministic_next_best_action: DecisionResult | null;
  ai_contextual_note: AiDecisionReview | null;
  status: AiStatus;
};

export type CandidateMemory = {
  observation: { content: string; source: string };
  inference: { content: string; confidence: number } | null;
  proposed_content: string;
  importance: number;
  confidence: number;
};

export type MemoryAnalysisResponse = {
  candidate_memory: CandidateMemory | null;
  status: AiStatus;
};

export type UpdateSettingsInput = {
  name: string;
  start_week_day: string;
  default_available_minutes: number;
  ai_enabled: boolean;
  contextual_review_enabled: boolean;
  activity_awareness_enabled: boolean;
  notifications_enabled: boolean;
  intervention_cooldown_minutes: number;
  start_with_windows: boolean;
};

export type UpdateSettingsResponse = {
  profile: Profile;
  settings: LocalSettings;
};

export type UpdateProfileInput = {
  name: string;
};

export type CompleteOnboardingInput = {
  name: string;
  goal: GoalInput;
  life_state: LifeStateInput;
  actions: ActionInput[];
};

export type HistoryEntry = {
  id: string;
  kind: "life_state" | "decision" | "execution" | "outcome" | string;
  timestamp: IsoTimestamp;
  title: string;
  detail: string;
  status?: string | null;
};

export type HistoryDay = {
  date: string;
  entries: HistoryEntry[];
};

export type HistoryData = {
  days: HistoryDay[];
};

export type AnalyticsSummary = {
  sample_size: number;
  completion_rate: number | null;
  average_result_quality: number | null;
  average_duration_minutes: number | null;
  average_energy_before: number | null;
  average_energy_after: number | null;
};

export type BootstrapData = {
  profile: Profile | null;
  settings: LocalSettings;
  life_state: LifeState | null;
  goals: Goal[];
  actions: ActionItem[];
  decision: DecisionResponse | null;
  active_execution: ActionExecution | null;
  recent_outcomes: ActionOutcome[];
  ai_status: AiStatus;
};

export type CalculateDecisionInput = {
  excluded_action_ids?: string[];
};

export type StartRecommendedActionInput = {
  action_id: string;
  decision_id?: string | null;
};

// Kept as aliases while the old in-memory prototype contracts are replaced.
export type CandidateAction = ActionItem;
export type CompleteActiveActionInput = Required<Pick<OutcomeInput, "result_quality" | "energy_after" | "difficulty">>;
export type AbandonActiveActionInput = Required<Pick<OutcomeInput, "energy_after" | "difficulty">>;
