import { useState } from "react";
import { ActionForm, blankAction } from "../components/ActionForm";
import { GoalForm } from "../components/GoalForm";
import { LifeStateForm } from "../components/LifeStateForm";
import { InlineError } from "../components/AsyncState";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { ActionInput, BootstrapData, GoalInput, LifeStateInput, LocalSettings } from "../types/domain";

type OnboardingStep = "welcome" | "identity" | "goals" | "state" | "actions" | "finish";

type OnboardingPageProps = {
  settings: LocalSettings;
  onCompleted: (data: BootstrapData) => void;
};

const initialGoal: GoalInput = { title: "", description: "", priority: 7 };

function initialLifeState(settings: LocalSettings): LifeStateInput {
  return {
    energy: 5,
    focus: 5,
    stress: 5,
    sleep_hours: 7,
    available_minutes: settings.default_available_minutes ?? 120,
    optional_note: null,
  };
}

function stepNumber(step: OnboardingStep): number {
  return ["identity", "goals", "state", "actions", "finish"].indexOf(step) + 1;
}

export function OnboardingPage({ settings, onCompleted }: OnboardingPageProps) {
  const [step, setStep] = useState<OnboardingStep>("welcome");
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | null>(null);
  const [goal, setGoal] = useState<GoalInput>(initialGoal);
  const [lifeState, setLifeState] = useState<LifeStateInput>(() => initialLifeState(settings));
  const [actions, setActions] = useState<ActionInput[]>([]);
  const [actionFormKey, setActionFormKey] = useState(0);
  const { error, pending, run } = useAsyncAction();

  function nextFromName() {
    if (!name.trim()) {
      setNameError("Tell VANTA what to call you.");
      return;
    }
    setNameError(null);
    setStep("goals");
  }

  async function finish() {
    const result = await run(() => vantaApi.completeOnboarding({
      name: name.trim(), goal, life_state: lifeState, actions,
    }));
    if (result.ok) onCompleted(result.value);
  }

  const numberedStep = stepNumber(step);

  return (
    <main className="onboarding-shell">
      <section className="onboarding-card">
        <div className="brand brand--onboarding" aria-label="VANTA Life">
          <span className="brand__mark">V</span>
          <span>VANTA <em>Life</em></span>
        </div>
        {step !== "welcome" && (
          <div className="onboarding-progress" aria-label={`Step ${numberedStep} of 5`}>
            {[1, 2, 3, 4, 5].map((item) => <span className={item <= numberedStep ? "onboarding-progress__dot onboarding-progress__dot--active" : "onboarding-progress__dot"} key={item} />)}
          </div>
        )}

        {step === "welcome" && (
          <div className="onboarding-step">
            <p className="eyebrow">Personal Decision OS</p>
            <h1>Your life. Better decisions.</h1>
            <p className="lead">VANTA turns your current capacity, strategic goals, and available actions into one considered next move. Your data stays on this device.</p>
            <div className="form-actions form-actions--end"><button className="button" onClick={() => setStep("identity")} type="button">Begin</button></div>
          </div>
        )}

        {step === "identity" && (
          <div className="onboarding-step">
            <p className="eyebrow">Step 1 of 5 · Identity</p>
            <h1>What should VANTA call you?</h1>
            <label className="onboarding-name-field">
              <span>Your name</span>
              <input autoFocus maxLength={80} onChange={(event) => setName(event.currentTarget.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); nextFromName(); } }} placeholder="Your first name" value={name} />
            </label>
            {nameError && <InlineError>{nameError}</InlineError>}
            <div className="form-actions"><button className="button button--ghost" onClick={() => setStep("welcome")} type="button">Back</button><button className="button" onClick={nextFromName} type="button">Continue</button></div>
          </div>
        )}

        {step === "goals" && (
          <div className="onboarding-step">
            <p className="eyebrow">Step 2 of 5 · Direction</p>
            <h1>What are you trying to improve?</h1>
            <p className="lead">Start with one strategic goal. You can add or refine goals later.</p>
            <GoalForm initialValue={goal} onCancel={() => setStep("identity")} onSubmit={(value) => { setGoal(value); setStep("state"); }} submitLabel="Continue" />
          </div>
        )}

        {step === "state" && (
          <div className="onboarding-step">
            <p className="eyebrow">Step 3 of 5 · Current state</p>
            <h1>How does today feel?</h1>
            <p className="lead">A quick, honest check-in lets VANTA respect the constraints of this day.</p>
            <LifeStateForm initialValue={lifeState} onCancel={() => setStep("goals")} onSubmit={(value) => { setLifeState(value); setStep("actions"); }} submitLabel="Continue" />
          </div>
        )}

        {step === "actions" && (
          <div className="onboarding-step">
            <p className="eyebrow">Step 4 of 5 · Options</p>
            <h1>Create your first actions.</h1>
            <p className="lead">Add concrete options VANTA can rank. You may skip this and add them later.</p>
            <ActionForm goals={[]} initialValue={{ ...blankAction, goal_id: null }} key={actionFormKey} onSubmit={(value) => { setActions((current) => [...current, value]); setActionFormKey((current) => current + 1); }} submitLabel="Add action" />
            {actions.length > 0 && <div className="onboarding-action-list" aria-label="First actions">
              {actions.map((action, index) => <div className="onboarding-action-row" key={`${action.title}-${index}`}>
                <span><strong>{action.title}</strong> <small>{action.duration_minutes} min</small></span>
                <button className="text-button text-button--danger" onClick={() => setActions((current) => current.filter((_, actionIndex) => actionIndex !== index))} type="button">Remove</button>
              </div>)}
            </div>}
            <div className="form-actions"><button className="button button--ghost" onClick={() => setStep("state")} type="button">Back</button><button className="button" onClick={() => setStep("finish")} type="button">{actions.length ? "Continue" : "Skip for now"}</button></div>
          </div>
        )}

        {step === "finish" && (
          <div className="onboarding-step">
            <p className="eyebrow">Step 5 of 5 · Ready</p>
            <h1>VANTA has enough context to begin.</h1>
            <p className="lead">Your first state, goal, and available actions will be saved locally. VANTA can then calculate your first Next Best Action.</p>
            {error && <InlineError>{error}</InlineError>}
            <div className="form-actions"><button className="button button--ghost" disabled={pending} onClick={() => setStep("actions")} type="button">Back</button><button className="button" disabled={pending} onClick={() => void finish()} type="button">{pending ? "Preparing VANTA…" : "Finish onboarding"}</button></div>
          </div>
        )}
      </section>
    </main>
  );
}
