import { useEffect, useState } from "react";
import type { AiStatus } from "../types/domain";

function formatDate(now: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    weekday: "long",
    month: "long",
    day: "numeric",
  }).format(now);
}

function formatTime(now: Date): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(now);
}

export function AppTopbar({ aiStatus }: { aiStatus: AiStatus }) {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 30_000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <header className="app-topbar">
      <div className="app-topbar__date">
        <span>{formatDate(now)}</span>
        <time dateTime={now.toISOString()}>{formatTime(now)}</time>
      </div>
      <div className="app-topbar__status" title={aiStatus.message}>
        <span aria-hidden="true" className={aiStatus.available ? "status-dot status-dot--ready" : "status-dot"} />
        <span>{aiStatus.available ? "Intelligence ready" : "Local mode"}</span>
      </div>
    </header>
  );
}
