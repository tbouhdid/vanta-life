import { useEffect, useRef, useState, type FormEvent } from "react";
import { InlineError, LoadingState, PageHeader } from "../components/AsyncState";
import { useAsyncAction } from "../hooks/useAsyncAction";
import { vantaApi } from "../services/bridge";
import type { AiStatus, BootstrapData, ChatMessage } from "../types/domain";
import { formatTime } from "../utils/format";

const shortcuts = [
  "What should I do now?", "Why did you choose this action?", "How am I performing this week?",
  "Should I continue or change task?", "What did you learn about me?",
];

type ChatPageProps = { data: BootstrapData };

export function ChatPage({ data }: ChatPageProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [loadingHistory, setLoadingHistory] = useState(true);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [responseStatus, setResponseStatus] = useState<AiStatus | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const { error, pending, run } = useAsyncAction();

  useEffect(() => {
    let active = true;
    void vantaApi.getChatMessages().then((items) => {
      if (active) setMessages(items);
    }).catch(() => {
      if (active) setHistoryError("Recent conversation could not be loaded.");
    }).finally(() => {
      if (active) setLoadingHistory(false);
    });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages, pending]);

  async function send(content = draft) {
    const trimmed = content.trim();
    if (!trimmed || pending) return;
    setDraft("");
    const result = await run(() => vantaApi.sendChatMessage(trimmed));
    if (result.ok) {
      setResponseStatus(result.value.status);
      setMessages((current) => [
        ...current,
        result.value.user_message,
        ...(result.value.assistant_message ? [result.value.assistant_message] : []),
      ]);
    } else {
      setDraft(trimmed);
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void send();
  }

  const effectiveStatus = responseStatus ?? data.ai_status;
  const activeGoal = data.goals.find((goal) => goal.active && !goal.completed_at);
  const currentAction = data.active_execution?.action_title
    ?? data.actions.find((action) => action.id === data.active_execution?.action_id)?.title;

  return (
    <div className="page chat-page">
      <PageHeader eyebrow="VANTA" title="Personal Intelligence">
        <span className={`ai-status ${effectiveStatus.available ? "ai-status--ready" : ""}`}>
          {effectiveStatus.available ? "AI ready" : effectiveStatus.configured ? "AI disabled" : "AI not configured"}
        </span>
      </PageHeader>
      <p className="page-intro">A calm space to think with your current context. VANTA can advise; your deterministic decision system remains in control.</p>
      <div className="context-chips" aria-label="Current VANTA context">
        <span>Current state {data.life_state ? `· E ${data.life_state.energy.toFixed(0)} / F ${data.life_state.focus.toFixed(0)}` : "· not checked in"}</span>
        <span>Active goal {activeGoal ? `· ${activeGoal.title}` : "· none"}</span>
        <span>Current action {currentAction ? `· ${currentAction}` : "· none"}</span>
      </div>

      {!effectiveStatus.available && <div className="chat-notice" role="status">{effectiveStatus.message} Your goals, decisions, actions, and history continue to work locally without AI.</div>}

      <section className="chat-shell" aria-label="VANTA conversation">
        <div className="chat-thread">
          {historyError && <InlineError>{historyError}</InlineError>}
          {loadingHistory ? <LoadingState title="Loading recent conversation…" /> : messages.length === 0 ? (
            <div className="chat-empty"><h2>Ask VANTA for context, not generic advice.</h2><p>Start with a question below. VANTA uses your current local context; the deterministic core remains authoritative.</p></div>
          ) : messages.map((message) => (
            <article className={`chat-message chat-message--${message.role}`} key={message.id}>
              <div className="chat-message__meta"><strong>{message.role === "assistant" ? "VANTA" : "You"}</strong><time dateTime={message.timestamp}>{formatTime(message.timestamp)}</time></div>
              <p>{message.content}</p>
            </article>
          ))}
          {pending && <div className="chat-generating" role="status">VANTA is considering your context…</div>}
          <div ref={bottomRef} />
        </div>
        <div className="chat-composer">
          <div className="chat-shortcuts" aria-label="Suggested questions">
            {shortcuts.map((shortcut) => <button disabled={pending} key={shortcut} onClick={() => void send(shortcut)} type="button">{shortcut}</button>)}
          </div>
          <form onSubmit={submit}>
            <label className="sr-only" htmlFor="chat-input">Message VANTA</label>
            <textarea id="chat-input" maxLength={4000} onChange={(event) => setDraft(event.currentTarget.value)} placeholder="Ask VANTA anything about your day, goals or decisions…" rows={3} value={draft} />
            <button className="button" disabled={!draft.trim() || pending} type="submit">{pending ? "Thinking…" : "Send"}</button>
          </form>
          {error && <InlineError>{error}</InlineError>}
        </div>
      </section>
    </div>
  );
}
