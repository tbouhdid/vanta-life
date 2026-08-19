import { Component, type ErrorInfo, type ReactNode } from "react";

type OnboardingErrorBoundaryProps = {
  children: ReactNode;
};

type OnboardingErrorBoundaryState = {
  error: Error | null;
};

/**
 * Keeps a rendering fault inside onboarding from taking down the entire React
 * tree. The original error remains in the development console and is shown
 * inline only in development builds.
 */
export class OnboardingErrorBoundary extends Component<OnboardingErrorBoundaryProps, OnboardingErrorBoundaryState> {
  state: OnboardingErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): OnboardingErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("VANTA onboarding render error", error, info);
  }

  private retry = () => {
    this.setState({ error: null });
  };

  render() {
    const { error } = this.state;

    if (!error) {
      return this.props.children;
    }

    return (
      <main className="onboarding-shell">
        <section className="onboarding-card onboarding-error" role="alert">
          <p className="eyebrow">Onboarding</p>
          <h1>Something went wrong during onboarding.</h1>
          <p className="lead">Your profile has not been created. You can safely try this step again.</p>
          {import.meta.env.DEV && <pre>{error.stack ?? error.message}</pre>}
          <div className="form-actions form-actions--end">
            <button className="button" onClick={this.retry} type="button">Try again</button>
          </div>
        </section>
      </main>
    );
  }
}
