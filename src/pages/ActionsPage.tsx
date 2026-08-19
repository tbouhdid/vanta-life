import { useState } from "react";
import { ActionForm } from "../components/ActionForm";
import { EmptyState, InlineError, PageHeader } from "../components/AsyncState";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { ActionInput, ActionItem, BootstrapData } from "../types/domain";
import { formatDate, formatNumber, goalForAction, statusLabel } from "../utils/format";

type ActionsPageProps = {
  data: BootstrapData;
  onRefresh: () => Promise<boolean>;
};

function asInput(action: ActionItem): ActionInput {
  return {
    title: action.title,
    description: action.description,
    goal_id: action.goal_id,
    impact: action.impact,
    urgency: action.urgency,
    goal_alignment: action.goal_alignment,
    energy_required: action.energy_required,
    duration_minutes: action.duration_minutes,
  };
}

function isCompleted(action: ActionItem): boolean {
  return action.status === "completed" || Boolean(action.completed_at);
}

type ActionFilter = "available" | "in_progress" | "completed" | "archived";

export function ActionsPage({ data, onRefresh }: ActionsPageProps) {
  const [showCreate, setShowCreate] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [filter, setFilter] = useState<ActionFilter>("available");
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

  async function createAction(input: ActionInput) {
    if (await refreshAfter(() => vantaApi.createActionItem(input))) {
      setShowCreate(false);
    }
  }

  async function updateAction(action: ActionItem, input: ActionInput) {
    if (await refreshAfter(() => vantaApi.updateAction({ id: action.id, ...input }))) {
      setEditingId(null);
    }
  }

  async function completeAction(action: ActionItem) {
    await refreshAfter(() => vantaApi.completeActionItem(action.id));
  }

  async function archiveAction(action: ActionItem) {
    await refreshAfter(() => vantaApi.archiveAction(action.id));
  }

  async function deleteAction(action: ActionItem) {
    const approved = window.confirm(`Delete “${action.title}”? This cannot be undone.`);
    if (approved) {
      await refreshAfter(() => vantaApi.deleteAction(action.id));
    }
  }

  const doneActions = data.actions.filter(isCompleted);
  const archivedActions = data.actions.filter((action) => action.status === "archived");
  const inProgressActions = data.actions.filter((action) => action.status === "in_progress" || data.active_execution?.action_id === action.id);
  const availableActions = data.actions.filter((action) => action.status === "available");
  const visibleActions = filter === "available" ? availableActions : inProgressActions;

  return (
    <div className="page">
      <PageHeader eyebrow="Options" title="Actions">
        <button className="button" onClick={() => { setEditingId(null); setShowCreate((current) => !current); }} type="button">
          {showCreate ? "Close" : "New action"}
        </button>
      </PageHeader>
      <p className="page-intro">Actions are candidates, not obligations. VANTA ranks open actions against your current state and active goals.</p>
      {error && <InlineError>{error}</InlineError>}

      <div className="filter-tabs" aria-label="Filter actions">
        <button aria-pressed={filter === "available"} className={filter === "available" ? "filter-tab filter-tab--active" : "filter-tab"} onClick={() => setFilter("available")} type="button">Available <span>{availableActions.length}</span></button>
        <button aria-pressed={filter === "in_progress"} className={filter === "in_progress" ? "filter-tab filter-tab--active" : "filter-tab"} onClick={() => setFilter("in_progress")} type="button">In progress <span>{inProgressActions.length}</span></button>
        <button aria-pressed={filter === "completed"} className={filter === "completed" ? "filter-tab filter-tab--active" : "filter-tab"} onClick={() => setFilter("completed")} type="button">Completed <span>{doneActions.length}</span></button>
        <button aria-pressed={filter === "archived"} className={filter === "archived" ? "filter-tab filter-tab--active" : "filter-tab"} onClick={() => setFilter("archived")} type="button">Archived <span>{archivedActions.length}</span></button>
      </div>

      {showCreate && (
        <section className="section-stack">
          <h2 className="section-heading">Create an action</h2>
          <ActionForm
            goals={data.goals.filter((goal) => !goal.completed_at)}
            isSubmitting={pending}
            onCancel={() => setShowCreate(false)}
            onSubmit={createAction}
            submitLabel="Create action"
          />
        </section>
      )}

      {(filter === "available" || filter === "in_progress") && <section className="section-stack">
        <div className="section-heading-row">
          <h2 className="section-heading">{filter === "available" ? "Available actions" : "In progress"}</h2>
          <span className="count-label">{visibleActions.length}</span>
        </div>
        {visibleActions.length === 0 && (
          <EmptyState title={filter === "available" ? "No available actions." : "Nothing is currently in focus."}>
            {filter === "available" ? "Add a concrete action to give the decision engine something useful to rank." : "Start the current recommendation to enter Focus Mode."}
          </EmptyState>
        )}
        <div className="entity-list">
          {visibleActions.map((action) => {
            const goal = goalForAction(action, data.goals);
            const inProgress = data.active_execution?.action_id === action.id;

            return (
              <article className={inProgress ? "entity-card entity-card--active" : "entity-card"} key={action.id}>
                {editingId === action.id ? (
                  <ActionForm
                    goals={data.goals.filter((item) => !item.completed_at)}
                    initialValue={asInput(action)}
                    isSubmitting={pending}
                    onCancel={() => setEditingId(null)}
                    onSubmit={(input) => updateAction(action, input)}
                    submitLabel="Save changes"
                  />
                ) : (
                  <>
                    <div className="entity-card__header">
                      <div>
                        <div className="tag-row">
                          {inProgress && <span className="status-pill status-pill--progress">In progress</span>}
                          <span className="metric-pill">{action.duration_minutes} min</span>
                          {goal && <span className="metric-pill">{goal.title}</span>}
                        </div>
                        <h3>{action.title}</h3>
                      </div>
                      <button aria-label={`Edit ${action.title}`} className="icon-button" disabled={pending} onClick={() => { setShowCreate(false); setEditingId(action.id); }} type="button">✎</button>
                    </div>
                    {action.description && <p>{action.description}</p>}
                    <dl className="compact-metrics">
                      <div><dt>Impact</dt><dd>{formatNumber(action.impact)}</dd></div>
                      <div><dt>Urgency</dt><dd>{formatNumber(action.urgency)}</dd></div>
                      <div><dt>Alignment</dt><dd>{formatNumber(action.goal_alignment)}</dd></div>
                      <div><dt>Energy</dt><dd>{formatNumber(action.energy_required)}</dd></div>
                    </dl>
                    <p className="muted">Created {formatDate(action.created_at)}</p>
                    <div className="card-actions">
                      {!inProgress && <button className="text-button" disabled={pending} onClick={() => void completeAction(action)} type="button">Mark completed</button>}
                      {!inProgress && <button className="text-button" disabled={pending} onClick={() => void archiveAction(action)} type="button">Archive</button>}
                      <button className="text-button text-button--danger" disabled={pending} onClick={() => void deleteAction(action)} type="button">Delete</button>
                    </div>
                  </>
                )}
              </article>
            );
          })}
        </div>
      </section>}

      {(filter === "completed" || filter === "archived") && (
        <section className="section-stack">
          <div className="section-heading-row">
            <h2 className="section-heading">{filter === "completed" ? "Completed actions" : "Archived actions"}</h2>
            <span className="count-label">{filter === "completed" ? doneActions.length : archivedActions.length}</span>
          </div>
          {(filter === "completed" ? doneActions : archivedActions).length === 0 ? <EmptyState title={filter === "completed" ? "No completed actions yet." : "No archived actions yet."}>{filter === "completed" ? "Completed actions become part of your local decision history." : "Archive options you do not want VANTA to consider right now."}</EmptyState> : <div className="entity-list">
            {(filter === "completed" ? doneActions : archivedActions).map((action) => (
              <article className="entity-card entity-card--muted" key={action.id}>
                <div className="entity-card__header">
                  <div>
                    <span className="status-pill status-pill--complete">{statusLabel(action.status)}</span>
                    <h3>{action.title}</h3>
                  </div>
                  <button aria-label={`Delete ${action.title}`} className="icon-button icon-button--danger" disabled={pending} onClick={() => void deleteAction(action)} type="button">×</button>
                </div>
                {action.description && <p>{action.description}</p>}
                <p className="muted">{filter === "completed" ? `Completed ${formatDate(action.completed_at)}` : `Archived ${formatDate(action.updated_at)}`}</p>
              </article>
            ))}
          </div>}
        </section>
      )}
    </div>
  );
}
