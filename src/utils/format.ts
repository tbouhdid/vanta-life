import type { ActionItem, ActionOutcome, DecisionResponse, Goal } from "../types/domain";

export function formatNumber(value: number | null | undefined, digits = 1): string {
  return typeof value === "number" && Number.isFinite(value) ? value.toFixed(digits) : "—";
}

export function formatDate(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }

  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
  }).format(date);
}

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatTime(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "—";
  }

  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatElapsed(startedAt: string, now = Date.now()): string {
  const elapsedSeconds = Math.max(0, Math.floor((now - new Date(startedAt).getTime()) / 1000));
  const hours = Math.floor(elapsedSeconds / 3600);
  const minutes = Math.floor((elapsedSeconds % 3600) / 60);
  const seconds = elapsedSeconds % 60;

  return [hours, minutes, seconds]
    .map((part) => part.toString().padStart(2, "0"))
    .join(":");
}

export function greetingForNow(): string {
  const hour = new Date().getHours();

  if (hour < 12) {
    return "Good morning";
  }

  if (hour < 18) {
    return "Good afternoon";
  }

  return "Good evening";
}

export function statusLabel(status: string): string {
  return status.replace(/_/g, " ");
}

export function goalForAction(action: ActionItem, goals: Goal[]): Goal | undefined {
  return goals.find((goal) => goal.id === action.goal_id);
}

export function actionTitle(outcome: ActionOutcome, actions: ActionItem[]): string {
  if (outcome.action_title) {
    return outcome.action_title;
  }

  return actions.find((action) => action.id === outcome.action_id)?.title ?? "Action";
}

export function decisionId(decision: DecisionResponse | null | undefined): string | null {
  return decision?.id ?? decision?.decision_id ?? decision?.decision?.id ?? null;
}

export function dayKey(value: string | null | undefined): string {
  if (!value) {
    return "Undated";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "Undated";
  }

  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(date);
}
