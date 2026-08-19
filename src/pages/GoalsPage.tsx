import { useState } from "react";
import { EmptyState, InlineError, PageHeader } from "../components/AsyncState";
import { GoalForm } from "../components/GoalForm";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { BootstrapData, Goal, GoalInput } from "../types/domain";
import { formatDate, formatNumber } from "../utils/format";

type GoalsPageProps = {
  data: BootstrapData;
  onRefresh: () => Promise<boolean>;
};

function asInput(goal: Goal): GoalInput {
  return {
    title: goal.title,
    description: goal.description,
    priority: goal.priority,
  };
}

function goalProgress(goal: Goal, data: BootstrapData) {
  const related = data.actions.filter((action) => action.goal_id === goal.id);
  const completed = related.filter((action) => action.status === "completed" || action.completed_at).length;
  return { total: related.length, completed, percentage: related.length > 0 ? Math.round((completed / related.length) * 100) : 0 };
}

export function GoalsPage({ data, onRefresh }: GoalsPageProps) {
  const [showCreate, setShowCreate] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const { error, pending, run } = useAsyncAction();

  async function refreshAfter(work: () => Promise<unknown>): Promise<boolean> {
    const result = await run(async () => {
      await work();
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("The change was saved, but the page could not refresh.");
      }
    });

    return result.ok;
  }

  async function createGoal(input: GoalInput) {
    if (await refreshAfter(() => vantaApi.createGoal(input))) {
      setShowCreate(false);
    }
  }

  async function updateGoal(goal: Goal, input: GoalInput) {
    if (await refreshAfter(() => vantaApi.updateGoal({ id: goal.id, ...input }))) {
      setEditingId(null);
    }
  }

  async function toggleGoal(goal: Goal) {
    await refreshAfter(() => vantaApi.toggleGoalActive(goal.id, !goal.active));
  }

  async function completeGoal(goal: Goal) {
    await refreshAfter(() => vantaApi.completeGoal(goal.id));
  }

  async function deleteGoal(goal: Goal) {
    const approved = window.confirm(`Delete “${goal.title}”? Its associated actions will remain available but unlinked.`);
    if (approved) {
      await refreshAfter(() => vantaApi.deleteGoal(goal.id));
    }
  }

  const openGoals = data.goals.filter((goal) => !goal.completed_at);
  const completedGoals = data.goals.filter((goal) => goal.completed_at);

  return (
    <div className="page">
      <PageHeader eyebrow="Direction" title="Goals">
        <button className="button" onClick={() => { setEditingId(null); setShowCreate((current) => !current); }} type="button">
          {showCreate ? "Close" : "New goal"}
        </button>
      </PageHeader>

      <p className="page-intro">Goals give the decision engine a sense of direction. Keep several active when life genuinely has several priorities.</p>
      {error && <InlineError>{error}</InlineError>}

      {showCreate && (
        <section className="section-stack">
          <h2 className="section-heading">Create a goal</h2>
          <GoalForm
            isSubmitting={pending}
            onCancel={() => setShowCreate(false)}
            onSubmit={createGoal}
            submitLabel="Create goal"
          />
        </section>
      )}

      <section className="section-stack">
        <div className="section-heading-row">
          <h2 className="section-heading">Current goals</h2>
          <span className="count-label">{openGoals.length}</span>
        </div>
        {openGoals.length === 0 && (
          <EmptyState title="No current goals yet.">
            Create one to help VANTA connect your actions to what you want to improve.
          </EmptyState>
        )}
        <div className="entity-list">
          {openGoals.map((goal) => {
            const progress = goalProgress(goal, data);
            return <article className={goal.active ? "entity-card entity-card--active goal-card" : "entity-card goal-card"} key={goal.id}>
              {editingId === goal.id ? (
                <GoalForm
                  initialValue={asInput(goal)}
                  isSubmitting={pending}
                  onCancel={() => setEditingId(null)}
                  onSubmit={(input) => updateGoal(goal, input)}
                  submitLabel="Save changes"
                />
              ) : (
                <>
                  <div className="entity-card__header">
                    <div>
                      <div className="tag-row">
                        <span className={goal.active ? "status-pill status-pill--active" : "status-pill"}>{goal.active ? "Active" : "Inactive"}</span>
                        <span className="metric-pill">Priority {formatNumber(goal.priority)}</span>
                      </div>
                      <h3>{goal.title}</h3>
                    </div>
                    <button aria-label={`Edit ${goal.title}`} className="icon-button" disabled={pending} onClick={() => { setShowCreate(false); setEditingId(goal.id); }} type="button">✎</button>
                  </div>
                  {goal.description && <p>{goal.description}</p>}
                  <div className="goal-progress" aria-label={`${progress.completed} of ${progress.total} linked actions completed`}>
                    <div className="goal-progress__meta"><span>Linked actions</span><strong>{progress.total ? `${progress.completed} / ${progress.total} complete` : "No actions linked"}</strong></div>
                    <span className="goal-progress__track"><i style={{ width: `${progress.percentage}%` }} /></span>
                  </div>
                  <p className="muted">Created {formatDate(goal.created_at)}</p>
                  <div className="card-actions">
                    <button className="text-button" disabled={pending} onClick={() => void toggleGoal(goal)} type="button">{goal.active ? "Deactivate" : "Activate"}</button>
                    <button className="text-button" disabled={pending} onClick={() => void completeGoal(goal)} type="button">Complete</button>
                    <button className="text-button text-button--danger" disabled={pending} onClick={() => void deleteGoal(goal)} type="button">Delete</button>
                  </div>
                </>
              )}
            </article>;
          })}
        </div>
      </section>

      {completedGoals.length > 0 && (
        <section className="section-stack">
          <div className="section-heading-row">
            <h2 className="section-heading">Completed</h2>
            <span className="count-label">{completedGoals.length}</span>
          </div>
          <div className="entity-list">
            {completedGoals.map((goal) => (
              <article className="entity-card entity-card--muted" key={goal.id}>
                <div className="entity-card__header">
                  <div>
                    <span className="status-pill status-pill--complete">Completed</span>
                    <h3>{goal.title}</h3>
                  </div>
                  <button aria-label={`Delete ${goal.title}`} className="icon-button icon-button--danger" disabled={pending} onClick={() => void deleteGoal(goal)} type="button">×</button>
                </div>
                {goal.description && <p>{goal.description}</p>}
                <p className="muted">Completed {formatDate(goal.completed_at)}</p>
              </article>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
