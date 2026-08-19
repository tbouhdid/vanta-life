import { useEffect, useState, type FormEvent } from "react";
import { disable, enable } from "@tauri-apps/plugin-autostart";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { InlineError, PageHeader } from "../components/AsyncState";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { BootstrapData, CandidateMemory } from "../types/domain";

type SettingsPageProps = {
  data: BootstrapData;
  onRefresh: () => Promise<boolean>;
};

export function SettingsPage({ data, onRefresh }: SettingsPageProps) {
  const [name, setName] = useState(data.profile?.name ?? "");
  const [startWeekDay, setStartWeekDay] = useState(data.settings.start_week_day || "monday");
  const [defaultAvailableMinutes, setDefaultAvailableMinutes] = useState(data.settings.default_available_minutes ?? 120);
  const [aiEnabled, setAiEnabled] = useState(data.settings.ai_enabled);
  const [contextualReviewEnabled, setContextualReviewEnabled] = useState(data.settings.contextual_review_enabled);
  const [notificationsEnabled, setNotificationsEnabled] = useState(data.settings.notifications_enabled);
  const [startWithWindows, setStartWithWindows] = useState(data.settings.start_with_windows);
  const [apiKey, setApiKey] = useState("");
  const [candidateMemory, setCandidateMemory] = useState<CandidateMemory | null>(null);
  const [validationError, setValidationError] = useState<string | null>(null);
  const { error, pending, run } = useAsyncAction();

  useEffect(() => {
    setName(data.profile?.name ?? "");
    setStartWeekDay(data.settings.start_week_day || "monday");
    setDefaultAvailableMinutes(data.settings.default_available_minutes ?? 120);
    setAiEnabled(data.settings.ai_enabled);
    setContextualReviewEnabled(data.settings.contextual_review_enabled);
    setNotificationsEnabled(data.settings.notifications_enabled);
    setStartWithWindows(data.settings.start_with_windows);
  }, [data.profile?.name, data.settings]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!name.trim()) {
      setValidationError("Your name cannot be empty.");
      return;
    }

    if (!Number.isFinite(defaultAvailableMinutes) || defaultAvailableMinutes < 0) {
      setValidationError("Default available time cannot be negative.");
      return;
    }

    setValidationError(null);
    const result = await run(async () => {
      await vantaApi.updateSettings({
        name: name.trim(),
        start_week_day: startWeekDay,
        default_available_minutes: defaultAvailableMinutes,
        ai_enabled: aiEnabled,
        contextual_review_enabled: contextualReviewEnabled,
        activity_awareness_enabled: data.settings.activity_awareness_enabled,
        notifications_enabled: notificationsEnabled,
        intervention_cooldown_minutes: data.settings.intervention_cooldown_minutes,
        start_with_windows: startWithWindows,
      });
      if (startWithWindows) await enable(); else await disable();
      const refreshed = await onRefresh();
      if (!refreshed) {
        throw new Error("Settings were saved, but the page could not refresh.");
      }
    });

    if (result.ok) {
      setValidationError(null);
    }
  }

  async function saveApiKey() {
    const key = apiKey.trim();
    if (!key) {
      setValidationError("Enter an API key before saving it.");
      return;
    }
    setValidationError(null);
    const result = await run(async () => {
      await vantaApi.setOpenAiApiKey(key);
      setApiKey("");
      if (!await onRefresh()) throw new Error("The key was stored, but Settings could not refresh.");
    });
    if (result.ok) setValidationError(null);
  }

  async function removeApiKey() {
    const result = await run(async () => {
      await vantaApi.clearOpenAiApiKey();
      if (!await onRefresh()) throw new Error("The key was removed, but Settings could not refresh.");
    });
    if (result.ok) setValidationError(null);
  }

  async function analyzeMemory() {
    const result = await run(() => vantaApi.analyzeLatestOutcomeMemory());
    if (result.ok) setCandidateMemory(result.value.candidate_memory);
  }

  async function testNotification() {
    const result = await run(async () => {
      let granted = await isPermissionGranted();
      if (!granted) granted = await requestPermission() === "granted";
      if (!granted) throw new Error("Windows notifications were not permitted.");
      sendNotification({ title: "VANTA — Focus check", body: "Your local decision system is ready when you are." });
    });
    if (result.ok) setValidationError(null);
  }

  const apiStatus = data.ai_status.configured ? "Configured" : "Not configured";

  return (
    <div className="page">
      <PageHeader eyebrow="Local preferences" title="Settings" />
      <p className="page-intro">These settings live on this device. API credentials are never stored in the local SQLite database.</p>

      <form className="settings-form" onSubmit={(event) => void submit(event)}>
        <section className="section-stack">
          <h2 className="section-heading">Profile</h2>
          <div className="form-card">
            <label className="form-field--compact">
              <span>Your name</span>
              <input maxLength={80} onChange={(event) => setName(event.currentTarget.value)} value={name} />
            </label>
          </div>
        </section>

        <section className="section-stack">
          <h2 className="section-heading">Application</h2>
          <div className="form-card form-grid form-grid--two">
            <label>
              <span>Week starts on</span>
              <select onChange={(event) => setStartWeekDay(event.currentTarget.value)} value={startWeekDay}>
                <option value="monday">Monday</option>
                <option value="sunday">Sunday</option>
              </select>
            </label>
            <label>
              <span>Default available time <small>minutes</small></span>
              <input min="0" onChange={(event) => setDefaultAvailableMinutes(Number(event.currentTarget.value))} step="5" type="number" value={defaultAvailableMinutes} />
            </label>
          </div>
        </section>

        <section className="section-stack">
          <h2 className="section-heading">VANTA Intelligence</h2>
          <div className="form-card settings-card">
            <label className="toggle-row">
              <span>
                <strong>AI enabled</strong>
                <small>Use OpenAI only when you explicitly chat, request a decision review, or analyze an outcome.</small>
              </span>
              <input checked={aiEnabled} onChange={(event) => setAiEnabled(event.currentTarget.checked)} type="checkbox" />
            </label>
            <label className="toggle-row">
              <span><strong>Contextual decision review</strong><small>Allow an explicitly requested AI review alongside the deterministic ranking. It never overrides the core decision.</small></span>
              <input checked={contextualReviewEnabled} onChange={(event) => setContextualReviewEnabled(event.currentTarget.checked)} type="checkbox" />
            </label>
            <div className="settings-status">
              <span>OpenAI</span>
              <strong>{apiStatus}</strong>
            </div>
            <label className="form-field--compact">
              <span>Set OpenAI API key</span>
              <input autoComplete="off" onChange={(event) => setApiKey(event.currentTarget.value)} placeholder="sk-…" type="password" value={apiKey} />
              <small>Stored only in this device’s secure credential store; it is never shown again or written to SQLite.</small>
            </label>
            <div className="button-row">
              <button className="button button--ghost" disabled={pending || !apiKey.trim()} onClick={() => void saveApiKey()} type="button">Set key</button>
              {data.ai_status.configured && <button className="text-button" disabled={pending} onClick={() => void removeApiKey()} type="button">Remove key</button>}
            </div>
            <div className="settings-status settings-status--note">
              <span>Candidate memory</span>
              <button className="text-button" disabled={pending || !data.ai_status.available} onClick={() => void analyzeMemory()} type="button">Analyze latest outcome</button>
            </div>
            {candidateMemory && (
              <div className="candidate-memory">
                <strong>Candidate memory — not saved</strong>
                <p><span>Observation:</span> {candidateMemory.observation.content}</p>
                {candidateMemory.inference && <p><span>Inference:</span> {candidateMemory.inference.content} ({Math.round(candidateMemory.inference.confidence * 100)}% confidence)</p>}
                <p><span>Proposed memory:</span> {candidateMemory.proposed_content}</p>
              </div>
            )}
          </div>
        </section>

        <section className="section-stack">
          <h2 className="section-heading">Notifications</h2>
          <div className="form-card settings-card">
            <label className="toggle-row"><span><strong>Windows notifications</strong><small>VANTA asks Windows permission only when you use a notification. No recurring notification is scheduled automatically.</small></span><input checked={notificationsEnabled} onChange={(event) => setNotificationsEnabled(event.currentTarget.checked)} type="checkbox" /></label>
            <button className="button button--ghost" disabled={pending || !notificationsEnabled} onClick={() => void testNotification()} type="button">Send test notification</button>
          </div>
        </section>

        <section className="section-stack">
          <h2 className="section-heading">Startup</h2>
          <div className="form-card settings-card">
            <label className="toggle-row"><span><strong>Start VANTA Life with Windows</strong><small>Optional and off by default. This creates or removes the local Windows startup entry when settings are saved.</small></span><input checked={startWithWindows} onChange={(event) => setStartWithWindows(event.currentTarget.checked)} type="checkbox" /></label>
          </div>
        </section>

        <section className="section-stack">
          <h2 className="section-heading">Data</h2>
          <div className="form-card data-card">
            <div>
              <strong>Local-first workspace</strong>
              <p>Profile, life state, goals, decisions, outcomes, and chat history remain on this device in local storage.</p>
            </div>
            <span className="status-pill">No cloud sync</span>
          </div>
        </section>

        {(validationError || error) && <InlineError>{validationError ?? error}</InlineError>}
        <div className="form-actions">
          <button className="button" disabled={pending} type="submit">{pending ? "Saving…" : "Save settings"}</button>
        </div>
      </form>
    </div>
  );
}
