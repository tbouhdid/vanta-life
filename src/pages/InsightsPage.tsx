import { useCallback, useEffect, useState } from "react";
import { EmptyState, ErrorState, LoadingState, PageHeader } from "../components/AsyncState";
import { vantaApi } from "../services/bridge";
import type { AnalyticsSummary } from "../types/domain";

function metric(value: number | null, suffix = ""): string {
  return value === null ? "—" : `${value.toFixed(1)}${suffix}`;
}

export function InsightsPage() {
  const [data, setData] = useState<AnalyticsSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const load = useCallback(async () => {
    setLoading(true); setError(null);
    try { setData(await vantaApi.getAnalytics()); }
    catch (caught) { setError(caught instanceof Error ? caught.message : "Insights could not be loaded."); }
    finally { setLoading(false); }
  }, []);
  useEffect(() => { void load(); }, [load]);

  return <div className="page">
    <PageHeader eyebrow="Evidence, not guesses" title="Insights"><button className="button button--ghost" disabled={loading} onClick={() => void load()} type="button">Refresh</button></PageHeader>
    <p className="page-intro">VANTA only surfaces measures supported by recorded outcomes. It does not infer patterns from an empty history.</p>
    {loading && <LoadingState title="Reading local outcome history…" />}
    {error && <ErrorState action={<button className="button" onClick={() => void load()} type="button">Retry</button>}>{error}</ErrorState>}
    {!loading && !error && data?.sample_size === 0 && <EmptyState title="Not enough data yet.">Complete or abandon actions with an outcome to build a meaningful personal record.</EmptyState>}
    {!loading && !error && data && data.sample_size > 0 && <section className="today-overview" aria-label="Personal analytics">
      <article className="life-state-summary"><h2>Completion rate</h2><p className="insight-value">{metric(data.completion_rate === null ? null : data.completion_rate * 100, "%")}</p><p className="muted">Across {data.sample_size} recorded outcomes.</p></article>
      <article className="active-goals-summary"><h2>Average quality</h2><p className="insight-value">{metric(data.average_result_quality, " / 10")}</p><p className="muted">Only outcomes you rated are included.</p></article>
      <article className="life-state-summary"><h2>Average duration</h2><p className="insight-value">{metric(data.average_duration_minutes, " min")}</p><p className="muted">Measured from actual execution timestamps.</p></article>
      <article className="active-goals-summary"><h2>Energy change</h2><p className="insight-value">{metric(data.average_energy_before)} → {metric(data.average_energy_after)}</p><p className="muted">Before and after recorded actions.</p></article>
    </section>}
  </div>;
}
