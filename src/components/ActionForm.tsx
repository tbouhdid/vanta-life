import { useEffect, useState, type FormEvent } from "react";
import type { ActionInput, Goal } from "../types/domain";
import { InlineError } from "./AsyncState";

export const blankAction: ActionInput = {
  title: "",
  description: "",
  goal_id: null,
  impact: 5,
  urgency: 5,
  goal_alignment: 5,
  energy_required: 5,
  duration_minutes: 30,
};

type ActionFormProps = {
  goals: Goal[];
  initialValue?: ActionInput;
  submitLabel: string;
  isSubmitting?: boolean;
  onSubmit: (input: ActionInput) => Promise<void> | void;
  onCancel?: () => void;
};

function validScale(value: number): boolean {
  return Number.isFinite(value) && value >= 0 && value <= 10;
}

export function ActionForm({
  goals,
  initialValue = blankAction,
  submitLabel,
  isSubmitting = false,
  onSubmit,
  onCancel,
}: ActionFormProps) {
  const [values, setValues] = useState<ActionInput>(initialValue);
  const [validationError, setValidationError] = useState<string | null>(null);

  useEffect(() => {
    setValues(initialValue);
    setValidationError(null);
  }, [initialValue]);

  function updateMetric(key: "impact" | "urgency" | "goal_alignment" | "energy_required", value: number) {
    setValues((current) => ({ ...current, [key]: value }));
  }

  function updateField<K extends keyof ActionInput>(key: K, value: ActionInput[K]) {
    setValues((current) => ({ ...current, [key]: value }));
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const metrics = [values.impact, values.urgency, values.goal_alignment, values.energy_required];

    if (!values.title.trim()) {
      setValidationError("Give this action a title.");
      return;
    }

    if (!metrics.every(validScale)) {
      setValidationError("Impact, urgency, alignment, and energy must be between 0 and 10.");
      return;
    }

    if (!Number.isFinite(values.duration_minutes) || values.duration_minutes <= 0) {
      setValidationError("Duration must be greater than zero minutes.");
      return;
    }

    setValidationError(null);
    await onSubmit({ ...values, title: values.title.trim(), description: values.description.trim() });
  }

  return (
    <form className="form-card" onSubmit={(event) => void submit(event)}>
      <div className="form-grid form-grid--two">
        <label>
          <span>Action title</span>
          <input
            autoFocus
            maxLength={160}
            onChange={(event) => updateField("title", event.currentTarget.value)}
            placeholder="What could you do next?"
            required
            value={values.title}
          />
        </label>
        <label>
          <span>Goal <small>optional</small></span>
          <select
            onChange={(event) => updateField("goal_id", event.currentTarget.value || null)}
            value={values.goal_id ?? ""}
          >
            <option value="">No associated goal</option>
            {goals.map((goal) => (
              <option key={goal.id} value={goal.id}>{goal.title}</option>
            ))}
          </select>
        </label>
      </div>
      <label>
        <span>Description <small>optional</small></span>
        <textarea
          maxLength={1200}
          onChange={(event) => updateField("description", event.currentTarget.value)}
          placeholder="What does done look like?"
          rows={3}
          value={values.description}
        />
      </label>
      <div className="form-grid form-grid--metrics">
        <label>
          <span>Impact <small>0–10</small></span>
          <input max="10" min="0" onChange={(event) => updateMetric("impact", Number(event.currentTarget.value))} required step="0.1" type="number" value={values.impact} />
        </label>
        <label>
          <span>Urgency <small>0–10</small></span>
          <input max="10" min="0" onChange={(event) => updateMetric("urgency", Number(event.currentTarget.value))} required step="0.1" type="number" value={values.urgency} />
        </label>
        <label>
          <span>Goal alignment <small>0–10</small></span>
          <input max="10" min="0" onChange={(event) => updateMetric("goal_alignment", Number(event.currentTarget.value))} required step="0.1" type="number" value={values.goal_alignment} />
        </label>
        <label>
          <span>Energy required <small>0–10</small></span>
          <input max="10" min="0" onChange={(event) => updateMetric("energy_required", Number(event.currentTarget.value))} required step="0.1" type="number" value={values.energy_required} />
        </label>
        <label>
          <span>Duration <small>minutes</small></span>
          <input min="1" onChange={(event) => updateField("duration_minutes", Number(event.currentTarget.value))} required step="5" type="number" value={values.duration_minutes} />
        </label>
      </div>
      {validationError && <InlineError>{validationError}</InlineError>}
      <div className="form-actions">
        {onCancel && (
          <button className="button button--ghost" disabled={isSubmitting} onClick={onCancel} type="button">
            Cancel
          </button>
        )}
        <button className="button" disabled={isSubmitting} type="submit">
          {isSubmitting ? "Saving…" : submitLabel}
        </button>
      </div>
    </form>
  );
}
