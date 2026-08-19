import { useEffect, useState, type FormEvent } from "react";
import type { ActionExecution, OutcomeInput } from "../types/domain";
import { formatElapsed, formatTime } from "../utils/format";
import { InlineError } from "./AsyncState";

type ExecutionFlow = "idle" | "complete" | "abandon";

type ExecutionControlsProps = {
  execution: ActionExecution;
  actionTitle: string;
  goalTitle?: string;
  pending?: boolean;
  onComplete: (input: OutcomeInput) => Promise<boolean>;
  onAbandon: (input: OutcomeInput) => Promise<boolean>;
};

function validScale(value: number): boolean {
  return Number.isFinite(value) && value >= 0 && value <= 10;
}

function OutcomeField({
  id,
  label,
  value,
  onChange,
  optional = false,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  optional?: boolean;
}) {
  return (
    <label className="outcome-field" htmlFor={id}>
      <span>{label} <small>0–10{optional ? ", optional" : ""}</small></span>
      <input
        id={id}
        max="10"
        min="0"
        onChange={(event) => onChange(event.currentTarget.value)}
        required={!optional}
        step="0.1"
        type="number"
        value={value}
      />
    </label>
  );
}

export function ExecutionControls({
  execution,
  actionTitle,
  goalTitle,
  pending = false,
  onComplete,
  onAbandon,
}: ExecutionControlsProps) {
  const [flow, setFlow] = useState<ExecutionFlow>("idle");
  const [resultQuality, setResultQuality] = useState("");
  const [energyAfter, setEnergyAfter] = useState("");
  const [difficulty, setDifficulty] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    setFlow("idle");
    setResultQuality("");
    setEnergyAfter("");
    setDifficulty("");
    setValidationError(null);
  }, [execution.id, execution.execution_id, execution.started_at]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  function valuesFor(flowType: Exclude<ExecutionFlow, "idle">): OutcomeInput | null {
    const after = Number(energyAfter);
    const effort = Number(difficulty);
    const quality = Number(resultQuality);

    if (!validScale(after) || !validScale(effort)) {
      setValidationError("Energy after and difficulty must be between 0 and 10.");
      return null;
    }

    if (flowType === "complete" && !validScale(quality)) {
      setValidationError("Result quality must be between 0 and 10.");
      return null;
    }

    setValidationError(null);
    return {
      energy_after: after,
      difficulty: effort,
      result_quality: flowType === "complete" ? quality : null,
    };
  }

  async function submit(event: FormEvent<HTMLFormElement>, flowType: Exclude<ExecutionFlow, "idle">) {
    event.preventDefault();
    const input = valuesFor(flowType);
    if (!input) {
      return;
    }

    const saved = flowType === "complete"
      ? await onComplete(input)
      : await onAbandon(input);

    if (saved) {
      setFlow("idle");
    }
  }

  return (
    <section className="execution-card" aria-live="polite">
      <div className="execution-card__topline">
        <span className="focus-mode-label">Focus mode</span>
        <span>Started {formatTime(execution.started_at)}</span>
      </div>
      <h2>{actionTitle}</h2>
      <p className="execution-card__elapsed">{formatElapsed(execution.started_at, now)}</p>
      <dl className="focus-mode-metrics">
        <div><dt>Goal</dt><dd>{goalTitle ?? "Unlinked action"}</dd></div>
        <div><dt>Starting energy</dt><dd>{execution.energy_before.toFixed(1)} / 10</dd></div>
      </dl>
      <p className="muted">Stay with the current action. Capture the outcome when you finish.</p>

      {flow === "idle" && (
        <div className="button-row">
          <button className="button" disabled={pending} onClick={() => setFlow("complete")} type="button">
            Complete
          </button>
          <button className="button button--ghost" disabled={pending} onClick={() => setFlow("abandon")} type="button">
            Abandon
          </button>
        </div>
      )}

      {flow === "complete" && (
        <form className="outcome-form" onSubmit={(event) => void submit(event, "complete")}>
          <h3>How did it go?</h3>
          <div className="form-grid form-grid--three">
            <OutcomeField id="result-quality" label="Result quality" onChange={setResultQuality} value={resultQuality} />
            <OutcomeField id="energy-after-complete" label="Energy after" onChange={setEnergyAfter} value={energyAfter} />
            <OutcomeField id="difficulty-complete" label="Difficulty" onChange={setDifficulty} value={difficulty} />
          </div>
          {validationError && <InlineError>{validationError}</InlineError>}
          <div className="button-row">
            <button className="button" disabled={pending} type="submit">{pending ? "Saving…" : "Save outcome"}</button>
            <button className="button button--ghost" disabled={pending} onClick={() => setFlow("idle")} type="button">Cancel</button>
          </div>
        </form>
      )}

      {flow === "abandon" && (
        <form className="outcome-form" onSubmit={(event) => void submit(event, "abandon")}>
          <h3>Leave this action intentionally</h3>
          <p className="muted">This records an abandoned outcome, not a failure.</p>
          <div className="form-grid form-grid--two">
            <OutcomeField id="energy-after-abandon" label="Energy after" onChange={setEnergyAfter} value={energyAfter} />
            <OutcomeField id="difficulty-abandon" label="Difficulty" onChange={setDifficulty} value={difficulty} />
          </div>
          {validationError && <InlineError>{validationError}</InlineError>}
          <div className="button-row">
            <button className="button button--danger" disabled={pending} type="submit">{pending ? "Saving…" : "Confirm abandon"}</button>
            <button className="button button--ghost" disabled={pending} onClick={() => setFlow("idle")} type="button">Cancel</button>
          </div>
        </form>
      )}
    </section>
  );
}
