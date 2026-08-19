import { useEffect, useState, type FormEvent } from "react";
import type { GoalInput } from "../types/domain";
import { InlineError } from "./AsyncState";

const blankGoal: GoalInput = {
  title: "",
  description: "",
  priority: 5,
};

type GoalFormProps = {
  initialValue?: GoalInput;
  submitLabel: string;
  isSubmitting?: boolean;
  onSubmit: (input: GoalInput) => Promise<void> | void;
  onCancel?: () => void;
};

export function GoalForm({
  initialValue = blankGoal,
  submitLabel,
  isSubmitting = false,
  onSubmit,
  onCancel,
}: GoalFormProps) {
  const [values, setValues] = useState<GoalInput>(initialValue);
  const [validationError, setValidationError] = useState<string | null>(null);

  useEffect(() => {
    setValues(initialValue);
    setValidationError(null);
  }, [initialValue]);

  function update<K extends keyof GoalInput>(key: K, value: GoalInput[K]) {
    setValues((current) => ({ ...current, [key]: value }));
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!values.title.trim()) {
      setValidationError("Give this goal a title.");
      return;
    }

    if (!Number.isFinite(values.priority) || values.priority < 0 || values.priority > 10) {
      setValidationError("Priority must be between 0 and 10.");
      return;
    }

    setValidationError(null);
    await onSubmit({ ...values, title: values.title.trim(), description: values.description.trim() });
  }

  return (
    <form className="form-card" onSubmit={(event) => void submit(event)}>
      <label>
        <span>Goal title</span>
        <input
          autoFocus
          maxLength={160}
          onChange={(event) => update("title", event.currentTarget.value)}
          placeholder="What are you trying to improve?"
          required
          value={values.title}
        />
      </label>
      <label>
        <span>Description <small>optional</small></span>
        <textarea
          maxLength={1200}
          onChange={(event) => update("description", event.currentTarget.value)}
          placeholder="Why does this matter now?"
          rows={3}
          value={values.description}
        />
      </label>
      <label className="form-field--compact">
        <span>Priority <small>0–10</small></span>
        <input
          max="10"
          min="0"
          onChange={(event) => update("priority", Number(event.currentTarget.value))}
          required
          step="0.1"
          type="number"
          value={values.priority}
        />
      </label>
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
