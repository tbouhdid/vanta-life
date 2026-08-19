import { useCallback, useEffect, useState } from "react";
import { EmptyState, ErrorState, LoadingState, PageHeader } from "../components/AsyncState";
import { vantaApi } from "../services/bridge";
import type { HistoryData, HistoryEntry } from "../types/domain";
import { formatTime, statusLabel } from "../utils/format";

function entryLabel(entry: HistoryEntry): string {
  switch (entry.kind) {
    case "life_state":
      return "State update";
    case "decision":
      return "Decision";
    case "execution":
      return "Action";
    case "outcome":
      return "Outcome";
    default:
      return statusLabel(entry.kind);
  }
}

export function HistoryPage() {
  const [history, setHistory] = useState<HistoryData | null>(null);
  const [state, setState] = useState<"loading" | "ready" | "error">("loading");
  const [error, setError] = useState<string | null>(null);

  const loadHistory = useCallback(async () => {
    setState("loading");
    setError(null);

    try {
      const nextHistory = await vantaApi.getHistory();
      setHistory(nextHistory);
      setState("ready");
    } catch (caughtError) {
      setState("error");
      setError(caughtError instanceof Error ? caughtError.message : "History could not be loaded.");
    }
  }, []);

  useEffect(() => {
    void loadHistory();
  }, [loadHistory]);

  return (
    <div className="page">
      <PageHeader eyebrow="Learning loop" title="History">
        <button className="button button--ghost" disabled={state === "loading"} onClick={() => void loadHistory()} type="button">Refresh</button>
      </PageHeader>
      <p className="page-intro">A local timeline of state updates, decisions, action, and outcome — the evidence VANTA uses to learn with you over time.</p>

      {state === "loading" && <LoadingState title="Loading your local history…" />}
      {state === "error" && (
        <ErrorState action={<button className="button" onClick={() => void loadHistory()} type="button">Try again</button>}>
          {error ?? "History could not be loaded."}
        </ErrorState>
      )}
      {state === "ready" && history?.days.length === 0 && (
        <EmptyState title="No history yet.">
          Your check-ins, decisions, and action outcomes will appear here as you use VANTA Life.
        </EmptyState>
      )}
      {state === "ready" && history && history.days.length > 0 && (
        <div className="history-days">
          {history.days.map((day) => (
            <section className="history-day" key={day.date}>
              <h2>{day.date}</h2>
              <div className="history-timeline">
                {day.entries.map((entry) => (
                  <article className="history-entry" key={`${entry.kind}-${entry.id}`}>
                    <div className="history-entry__time">{formatTime(entry.timestamp)}</div>
                    <div className="history-entry__marker" aria-hidden="true" />
                    <div className="history-entry__body">
                      <div className="tag-row">
                        <span className="metric-pill">{entryLabel(entry)}</span>
                        {entry.status && <span className="status-pill">{statusLabel(entry.status)}</span>}
                      </div>
                      <h3>{entry.title}</h3>
                      {entry.detail && <p>{entry.detail}</p>}
                    </div>
                  </article>
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
