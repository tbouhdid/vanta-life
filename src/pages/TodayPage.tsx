import { useEffect, useRef, useState } from "react";
import { EmptyState, InlineError, PageHeader } from "../components/AsyncState";
import { ExecutionControls } from "../components/ExecutionControls";
import { LifeStateForm } from "../components/LifeStateForm";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { ActionItem, ActionOutcome, AiDecisionReview, BootstrapData, DecisionResponse, LifeStateInput, OutcomeInput } from "../types/domain";
import type { AppPage } from "../types/navigation";
import { actionTitle, decisionId, formatDateTime, formatNumber, goalForAction, greetingForNow } from "../utils/format";

type TodayPageProps = {
  data: BootstrapData;
  onRefresh: () => Promise<boolean>;
  onNavigate: (page: AppPage) => void;
};

function lifeStateInput(data: BootstrapData): LifeStateInput {
  return data.life_state
    ? {
      energy: data.life_state.energy,
      focus: data.life_state.focus,
      stress: data.life_state.stress,
      sleep_hours: data.life_state.sleep_hours,
      available_minutes: data.life_state.available_minutes,
    }
    : {
      energy: 5,
      focus: 5,
      stress: 5,
      sleep_hours: 7,
      available_minutes: data.settings.default_available_minutes ?? 120,
    };
}

function isOpenAction(action: ActionItem): boolean {
  return action.status === "available";
}

function executionActionTitle(data: BootstrapData): string {
  const execution = data.active_execution;
  if (!execution) {
    return "Action";
  }

  return execution.action_title ?? data.actions.find((action) => action.id === execution.action_id)?.title ?? "Action";
}

function todayMessage(data: BootstrapData): string {
  if (!data.life_state) return "Set your current state, then let VANTA find a move that fits the day.";
  if (data.active_execution) return "Your attention is already committed. Finish deliberately, then reassess.";
  if (data.life_state.energy <= 3) return "Low capacity noted. Precision matters more than volume today.";
  if (data.life_state.available_minutes < 45) return "Time is constrained. VANTA will keep the next decision practical.";
  return "Your current state, goals, and constraints are ready for a deliberate next move.";
}

function OutcomeSummary({ outcome, actions }: { outcome: ActionOutcome; actions: ActionItem[] }) {
  const label = outcome.completed ? "Completed" : "Abandoned";
  const timestamp = outcome.created_at ?? outcome.ended_at;

  return (
    <article className="outcome-summary">
      <div>
        <span className={outcome.completed ? "status-pill status-pill--complete" : "status-pill"}>{label}</span>
        <h3>{actionTitle(outcome, actions)}</h3>
      </div>
      <div className="outcome-summary__metrics">
        <span>{outcome.actual_duration_minutes} min</span>
        {outcome.result_quality !== null && <span>Quality {formatNumber(outcome.result_quality)}</span>}
        <span>{formatDateTime(timestamp)}</span>
      </div>
    </article>
  );
}

export function TodayPage({ data, onRefresh, onNavigate }: TodayPageProps) {
  const [showCheckIn, setShowCheckIn] = useState(false);
  const [decision, setDecision] = useState<DecisionResponse | null>(data.decision);
  const [skippedActionIds, setSkippedActionIds] = useState<string[]>([]);
  const [aiReview, setAiReview] = useState<AiDecisionReview | null>(null);
  const [showDecisionDetail, setShowDecisionDetail] = useState(false);
  const automaticCalculationAttempted = useRef(false);
  const { error, pending, run } = useAsyncAction();

  useEffect(() => {
    if (data.decision) {
      setDecision(data.decision);
    }
  }, [data.decision]);

  async function calculate(excludedActionIds = skippedActionIds): Promise<DecisionResponse | null> {
    const result = await run(() => vantaApi.calculateDecision({
      excluded_action_ids: excludedActionIds.length > 0 ? excludedActionIds : undefined,
    }));

    if (result.ok) {
      setDecision(result.value);
      setShowDecisionDetail(false);
      return result.value;
    }

    return null;
  }

  useEffect(() => {
    const hasCandidates = data.actions.some(isOpenAction);
    if (
      !automaticCalculationAttempted.current
      && !data.active_execution
      && !decision
      && data.life_state
      && hasCandidates
    ) {
      automaticCalculationAttempted.current = true;
      void calculate();
    }
  }, [data.active_execution, data.actions, data.life_state, decision]);

  async function saveCheckIn(input: LifeStateInput) {
    const result = await run(async () => {
      await vantaApi.saveLifeState(input);
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("Your check-in was saved, but the page could not refresh.");
      }
    });

    if (result.ok) {
      setDecision(null);
      automaticCalculationAttempted.current = false;
      setShowCheckIn(false);
    }
  }

  async function skipRecommendedAction() {
    const nextAction = decision?.next_best_action;
    if (!nextAction) {
      return;
    }

    const exclusions = [...new Set([...skippedActionIds, nextAction.action_id])];
    setSkippedActionIds(exclusions);
    await calculate(exclusions);
  }

  async function requestAiReview() {
    const id = decisionId(decision);
    if (!id) return;
    const result = await run(() => vantaApi.consultDecisionWithAi(id));
    if (result.ok) setAiReview(result.value.ai_contextual_note);
  }

  async function startRecommendedAction() {
    const nextAction = decision?.next_best_action;
    if (!nextAction || !nextAction.feasible) {
      return;
    }

    const result = await run(async () => {
      await vantaApi.startRecommendedAction({
        action_id: nextAction.action_id,
        decision_id: decisionId(decision),
      });
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("The action started, but the page could not refresh.");
      }
    });

    if (result.ok) {
      setDecision(null);
      setAiReview(null);
    }
  }

  async function finishExecution(input: OutcomeInput): Promise<boolean> {
    const result = await run(async () => {
      await vantaApi.completeActiveAction(input);
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("The outcome was saved, but the page could not refresh.");
      }
    });

    if (result.ok) {
      setDecision(null);
      automaticCalculationAttempted.current = false;
    }

    return result.ok;
  }

  async function abandonExecution(input: OutcomeInput): Promise<boolean> {
    const result = await run(async () => {
      await vantaApi.abandonActiveAction(input);
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("The outcome was saved, but the page could not refresh.");
      }
    });

    if (result.ok) {
      setDecision(null);
      automaticCalculationAttempted.current = false;
    }

    return result.ok;
  }

  async function completeTodayAction(action: ActionItem) {
    await run(async () => {
      await vantaApi.completeActionItem(action.id);
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("The action was completed, but the page could not refresh.");
      }
    });
  }

  const activeGoals = data.goals.filter((goal) => goal.active && !goal.completed_at);
  const availableActions = data.actions.filter(isOpenAction).slice(0, 5);
  const nextAction = decision?.next_best_action ?? null;
  const selectedAction = nextAction ? data.actions.find((action) => action.id === nextAction.action_id) : undefined;
  const selectedGoal = selectedAction ? goalForAction(selectedAction, data.goals) : undefined;
  const executionGoal = data.active_execution
    ? data.goals.find((goal) => goal.id === data.actions.find((action) => action.id === data.active_execution?.action_id)?.goal_id)?.title
    : undefined;

  return (
    <div className="page today-page">
      <PageHeader eyebrow="Today" title={`${greetingForNow()}, ${data.profile?.name ?? "there"}.`}>
        <button className="button button--ghost" disabled={pending} onClick={() => setShowCheckIn((current) => !current)} type="button">
          {showCheckIn ? "Close check-in" : "Check in"}
        </button>
      </PageHeader>
      <p className="today-hero__message">{todayMessage(data)}</p>

      {error && <InlineError>{error}</InlineError>}

      {showCheckIn && (
        <section className="section-stack today-checkin">
          <div className="section-heading-row">
            <h2 className="section-heading">Current life state</h2>
            <span className="muted">A new check-in replaces the decision context.</span>
          </div>
          <LifeStateForm
            initialValue={lifeStateInput(data)}
            isSubmitting={pending}
            onCancel={() => setShowCheckIn(false)}
            onSubmit={saveCheckIn}
            submitLabel="Save check-in"
          />
        </section>
      )}

      <section className="today-overview">
        <article className="life-state-summary">
          <div className="section-heading-row">
            <h2>Life state</h2>
            <span className="muted">{data.life_state ? `Updated ${formatDateTime(data.life_state.timestamp)}` : "No check-in today"}</span>
          </div>
          {data.life_state ? (
            <dl className="life-metrics">
              <div><dt>Energy</dt><dd>{formatNumber(data.life_state.energy)}</dd></div>
              <div><dt>Focus</dt><dd>{formatNumber(data.life_state.focus)}</dd></div>
              <div><dt>Stress</dt><dd>{formatNumber(data.life_state.stress)}</dd></div>
              <div><dt>Recovery</dt><dd>{formatNumber(data.life_state.sleep_hours)}<small>h sleep</small></dd></div>
            </dl>
          ) : (
            <EmptyState title="No life state yet.">
              A one-minute check-in gives VANTA the context it needs to make a useful recommendation.
            </EmptyState>
          )}
        </article>

        <article className="active-goals-summary">
          <div className="section-heading-row">
            <h2>Strategic direction</h2>
            <button className="text-button" onClick={() => onNavigate("goals")} type="button">View goals</button>
          </div>
          {activeGoals.length === 0 ? (
            <p className="muted">No active goal yet. Give the next decision a direction.</p>
          ) : (
            <ul className="goal-chip-list">
              {activeGoals.map((goal) => <li key={goal.id}><strong>{goal.title}</strong><span>{formatNumber(goal.priority)}</span></li>)}
            </ul>
          )}
        </article>
      </section>

      <section className="section-stack">
        <div className="section-heading-row">
          <h2 className="section-heading">Next best action</h2>
          {!data.active_execution && <button className="text-button" disabled={pending} onClick={() => void calculate()} type="button">Recalculate</button>}
        </div>

        {data.active_execution ? (
          <ExecutionControls
            actionTitle={executionActionTitle(data)}
            execution={data.active_execution}
            goalTitle={executionGoal}
            onAbandon={abandonExecution}
            onComplete={finishExecution}
            pending={pending}
          />
        ) : !data.life_state ? (
          <EmptyState title="Check in before deciding." action={<button className="button" onClick={() => setShowCheckIn(true)} type="button">Start check-in</button>}>
            VANTA needs your current energy, focus, stress, sleep, and available time before it can rank actions.
          </EmptyState>
        ) : availableActions.length === 0 ? (
          <EmptyState title="Create an action to get a recommendation." action={<button className="button" onClick={() => onNavigate("actions")} type="button">Create action</button>}>
            The decision engine only ranks actions you have made available.
          </EmptyState>
        ) : nextAction ? (
          <article className="next-action-card">
            <div className="next-action-card__label">
              <span>Next best action</span>
              {aiReview && <small>VANTA Intelligence reviewed this decision</small>}
            </div>
            <div className="next-action-card__header">
              <div>
                <h2>{nextAction.action_title}</h2>
                <p>{nextAction.reason}</p>
              </div>
              <div className="score-orb"><span>Score</span><strong aria-label={`Score ${nextAction.score.toFixed(2)}`}>{nextAction.score.toFixed(2)}</strong></div>
            </div>
            <dl className="recommendation-metrics">
              <div><dt>Duration</dt><dd>{selectedAction?.duration_minutes ?? "—"} min</dd></div>
              <div><dt>Goal</dt><dd>{selectedGoal?.title ?? "Unlinked"}</dd></div>
              <div><dt>Energy fit</dt><dd>{formatNumber(selectedAction?.energy_required)} required</dd></div>
              <div><dt>Confidence</dt><dd>{nextAction.feasible ? "Fits today" : "Review needed"}</dd></div>
            </dl>
            <div className="decision-cta">
              <button className="button" disabled={pending || !nextAction.feasible} onClick={() => void startRecommendedAction()} type="button">{pending ? "Preparing..." : "Start focus"}</button>
              <button className="button button--ghost" onClick={() => setShowDecisionDetail((current) => !current)} type="button">Why this?</button>
              <button className="button button--quiet" disabled={pending} onClick={() => void skipRecommendedAction()} type="button">Other options</button>
              <button className="text-button" disabled={pending || !data.ai_status.available || !data.settings.contextual_review_enabled || !decisionId(decision)} onClick={() => void requestAiReview()} type="button">Request AI context</button>
            </div>
            {showDecisionDetail && nextAction.components && (
              <dl className="decision-detail" aria-label="Decision score components">
                <div><dt>Impact</dt><dd>{formatNumber(nextAction.components.impact_score, 2)}</dd></div>
                <div><dt>Urgency</dt><dd>{formatNumber(nextAction.components.urgency_score, 2)}</dd></div>
                <div><dt>Goal alignment</dt><dd>{formatNumber(nextAction.components.goal_alignment_score, 2)}</dd></div>
                <div><dt>Energy fit</dt><dd>{formatNumber(nextAction.components.energy_fit_score, 2)}</dd></div>
                <div><dt>Time fit</dt><dd>{formatNumber(nextAction.components.time_fit_score, 2)}</dd></div>
              </dl>
            )}
            <div className="button-row">
              <button className="button" disabled={pending || !nextAction.feasible} onClick={() => void startRecommendedAction()} type="button">{pending ? "Working…" : "Start"}</button>
              <button className="button button--ghost" disabled={pending} onClick={() => void skipRecommendedAction()} type="button">Skip</button>
              <button className="button button--ghost" disabled={pending} onClick={() => void calculate()} type="button">Recalculate</button>
              <button className="text-button" disabled={pending || !data.ai_status.available || !data.settings.contextual_review_enabled || !decisionId(decision)} onClick={() => void requestAiReview()} type="button">Ask AI for context</button>
            </div>
            {aiReview && (
              <aside className="ai-contextual-note">
                <strong>VANTA Intelligence <small>Advisory only</small></strong>
                <p>{aiReview.explanation}</p>
                {aiReview.contextual_factors.length > 0 && <p className="muted">Factors: {aiReview.contextual_factors.join(" · ")}</p>}
                {aiReview.ranking_override_recommended && <p className="muted">The deterministic recommendation remains the default; no ranking was changed.</p>}
              </aside>
            )}
          </article>
        ) : (
          <EmptyState title="No recommendation yet." action={<button className="button" disabled={pending} onClick={() => void calculate()} type="button">Calculate next action</button>}>
            VANTA can now rank your available actions against your current life state.
          </EmptyState>
        )}
      </section>

      <section className="section-stack">
        <div className="section-heading-row">
          <h2 className="section-heading">Today’s actions</h2>
          <button className="text-button" onClick={() => onNavigate("actions")} type="button">View all</button>
        </div>
        {availableActions.length === 0 ? (
          <p className="muted">No available actions.</p>
        ) : (
          <div className="today-action-list">
            {availableActions.map((action) => {
              const goal = goalForAction(action, data.goals);
              return (
                <article className="today-action-row" key={action.id}>
                  <div>
                    <h3>{action.title}</h3>
                    <p>{goal?.title ?? "No goal"} · {action.duration_minutes} min · Energy {formatNumber(action.energy_required)}</p>
                  </div>
                  <button className="text-button" disabled={pending || data.active_execution?.action_id === action.id} onClick={() => void completeTodayAction(action)} type="button">Complete</button>
                </article>
              );
            })}
          </div>
        )}
      </section>

      <section className="section-stack">
        <div className="section-heading-row">
          <h2 className="section-heading">Recent outcomes</h2>
          <button className="text-button" onClick={() => onNavigate("history")} type="button">View history</button>
        </div>
        {data.recent_outcomes.length === 0 ? (
          <p className="muted">Complete or abandon an action to start building an outcome history.</p>
        ) : (
          <div className="outcome-list">
            {data.recent_outcomes.slice(0, 5).map((outcome, index) => (
              <OutcomeSummary actions={data.actions} key={outcome.id ?? `${outcome.execution_id ?? "outcome"}-${index}`} outcome={outcome} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
