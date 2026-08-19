import { Component, type ErrorInfo, type ReactNode } from "react";

type AppErrorBoundaryProps = { children: ReactNode };
type AppErrorBoundaryState = { error: Error | null };

/** Keeps an unexpected rendering error from becoming an empty application window. */
export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("VANTA render error", error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;
    const detail = this.state.error.stack ?? this.state.error.message;
    return (
      <main className="app-loading-shell">
        <section className="state-card state-card--error" role="alert">
          <p className="eyebrow">VANTA Life</p>
          <h1>VANTA encountered a problem loading this view.</h1>
          <p>Nothing has been changed in your local data. You can retry this view or return to Today.</p>
          {import.meta.env.DEV && <pre className="error-diagnostic">{detail}</pre>}
          <div className="form-actions form-actions--start">
            <button className="button button--ghost" onClick={() => this.setState({ error: null })} type="button">Retry</button>
            <button className="button" onClick={() => window.location.reload()} type="button">Return to Today</button>
          </div>
        </section>
      </main>
    );
  }
}
