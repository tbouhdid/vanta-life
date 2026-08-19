import { useEffect, useState, type FormEvent } from "react";
import type { LifeStateInput } from "../types/domain";
import { InlineError } from "./AsyncState";

type LifeStateFormProps = {
  initialValue: LifeStateInput;
  submitLabel: string;
  isSubmitting?: boolean;
  onSubmit: (input: LifeStateInput) => Promise<void> | void;
  onCancel?: () => void;
};

function validScale(value: number): boolean {
  return Number.isFinite(value) && value >= 0 && value <= 10;
}

function numberValue(value: string): number {
  return Number(value);
}

export function LifeStateForm({
  initialValue,
  submitLabel,
  isSubmitting = false,
  onSubmit,
  onCancel,
}: LifeStateFormProps) {
  const [values, setValues] = useState<LifeStateInput>(initialValue);
  const [validationError, setValidationError] = useState<string | null>(null);

  useEffect(() => {
    setValues(initialValue);
    setValidationError(null);
  }, [initialValue]);

  function update<K extends keyof LifeStateInput>(key: K, value: LifeStateInput[K]) {
    setValues((current) => ({ ...current, [key]: value }));
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (![values.energy, values.focus, values.stress].every(validScale)) {
      setValidationError("Energy, focus, and stress must be between 0 and 10.");
      return;
    }

    if (!Number.isFinite(values.sleep_hours) || values.sleep_hours < 0 || values.sleep_hours > 24) {
      setValidationError("Sleep must be between 0 and 24 hours.");
      return;
    }

    if (!Number.isFinite(values.available_minutes) || values.available_minutes < 0) {
      setValidationError("Available time cannot be negative.");
      return;
    }

    setValidationError(null);
    await onSubmit(values);
  }

  return (
    <form className="form-card" onSubmit={(event) => void submit(event)}>
      <div className="form-grid form-grid--metrics">
        <label>
          <span>Energy <small>0–10</small></span>
          <input
            max="10"
            min="0"
            onChange={(event) => update("energy", numberValue(event.currentTarget.value))}
            required
            step="0.1"
            type="number"
            value={values.energy}
          />
        </label>
        <label>
          <span>Focus <small>0–10</small></span>
          <input
            max="10"
            min="0"
            onChange={(event) => update("focus", numberValue(event.currentTarget.value))}
            required
            step="0.1"
            type="number"
            value={values.focus}
          />
        </label>
        <label>
          <span>Stress <small>0–10</small></span>
          <input
            max="10"
            min="0"
            onChange={(event) => update("stress", numberValue(event.currentTarget.value))}
            required
            step="0.1"
            type="number"
            value={values.stress}
          />
        </label>
        <label>
          <span>Sleep <small>hours</small></span>
          <input
            max="24"
            min="0"
            onChange={(event) => update("sleep_hours", numberValue(event.currentTarget.value))}
            required
            step="0.1"
            type="number"
            value={values.sleep_hours}
          />
        </label>
        <label>
          <span>Available time <small>minutes</small></span>
          <input
            min="0"
            onChange={(event) => update("available_minutes", numberValue(event.currentTarget.value))}
            required
            step="5"
            type="number"
            value={values.available_minutes}
          />
        </label>
      </div>
      <label>
        <span>Context note <small>optional</small></span>
        <textarea
          maxLength={1200}
          onChange={(event) => update("optional_note", event.currentTarget.value || null)}
          placeholder="Anything relevant about today?"
          rows={2}
          value={values.optional_note ?? ""}
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
