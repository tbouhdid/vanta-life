import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { AppSidebar } from "./components/AppSidebar";
import { AppTopbar } from "./components/AppTopbar";
import { ErrorState, InlineError, LoadingState } from "./components/AsyncState";
import { OnboardingErrorBoundary } from "./components/OnboardingErrorBoundary";
import { vantaApi } from "./services/bridge";
import type { BootstrapData } from "./types/domain";
import type { AppPage } from "./types/navigation";
import { ActionsPage } from "./pages/ActionsPage";
import { ChatPage } from "./pages/ChatPage";
import { GoalsPage } from "./pages/GoalsPage";
import { HistoryPage } from "./pages/HistoryPage";
import { InsightsPage } from "./pages/InsightsPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { SettingsPage } from "./pages/SettingsPage";
import { TodayPage } from "./pages/TodayPage";
import "./App.css";

type AppLoadState = "loading" | "ready" | "error";

function messageFrom(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "VANTA Life could not load your local data.";
}

function App() {
  const [loadState, setLoadState] = useState<AppLoadState>("loading");
  const [data, setData] = useState<BootstrapData | null>(null);
  const dataRef = useRef<BootstrapData | null>(null);
  const [appError, setAppError] = useState<string | null>(null);
  const [activePage, setActivePage] = useState<AppPage>("today");

  useEffect(() => {
    dataRef.current = data;
  }, [data]);

  const refresh = useCallback(async (): Promise<boolean> => {
    try {
      const nextData = await vantaApi.getBootstrap();
      setData(nextData);
      setAppError(null);
      setLoadState("ready");
      return true;
    } catch (error) {
      setAppError(messageFrom(error));
      if (!dataRef.current) {
        setLoadState("error");
      }
      return false;
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  function navigate(page: AppPage) {
    setActivePage(page);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function completeOnboarding(nextData: BootstrapData) {
    setData(nextData);
    setAppError(null);
    setLoadState("ready");
    setActivePage("today");
  }

  if (loadState === "loading" && !data) {
    return (
      <main className="app-loading-shell">
        <LoadingState title="Opening VANTA Life…" />
      </main>
    );
  }

  if (loadState === "error" && !data) {
    return (
      <main className="app-loading-shell">
        <ErrorState action={<button className="button" onClick={() => { setLoadState("loading"); void refresh(); }} type="button">Try again</button>}>
          {appError ?? "Your local data could not be loaded."}
        </ErrorState>
      </main>
    );
  }

  if (!data) {
    return null;
  }

  if (!data.profile || !data.profile.onboarding_completed) {
    return (
      <OnboardingErrorBoundary>
        <OnboardingPage onCompleted={completeOnboarding} settings={data.settings} />
      </OnboardingErrorBoundary>
    );
  }

  let page: ReactNode;
  switch (activePage) {
    case "goals":
      page = <GoalsPage data={data} onRefresh={refresh} />;
      break;
    case "actions":
      page = <ActionsPage data={data} onRefresh={refresh} />;
      break;
    case "history":
      page = <HistoryPage />;
      break;
    case "insights":
      page = <InsightsPage />;
      break;
    case "chat":
      page = <ChatPage data={data} />;
      break;
    case "settings":
      page = <SettingsPage data={data} onRefresh={refresh} />;
      break;
    case "today":
    default:
      page = <TodayPage data={data} onNavigate={navigate} onRefresh={refresh} />;
      break;
  }

  return (
    <div className="app-shell">
      <AppSidebar activePage={activePage} name={data.profile.name} onNavigate={navigate} />
      <main className="app-content">
        <AppTopbar aiStatus={data.ai_status} />
        {appError && <InlineError>{appError}</InlineError>}
        {page}
      </main>
    </div>
  );
}

export default App;
