import { appPages, pageLabels, type AppPage } from "../types/navigation";

type AppSidebarProps = {
  activePage: AppPage;
  name: string;
  onNavigate: (page: AppPage) => void;
};

function NavIcon({ page }: { page: AppPage }) {
  const common = { fill: "none", stroke: "currentColor", strokeLinecap: "round" as const, strokeLinejoin: "round" as const, strokeWidth: 1.65 };
  const path = (() => {
    switch (page) {
      case "today": return <><path d="M12 3v3M12 18v3M4.2 12h3M16.8 12h3M6.5 6.5l2.1 2.1M15.4 15.4l2.1 2.1M17.5 6.5l-2.1 2.1M8.6 15.4l-2.1 2.1" {...common} /><circle cx="12" cy="12" r="3.5" {...common} /></>;
      case "chat": return <><path d="M5 18.2 4 21l3.5-1.5A8.5 8.5 0 1 0 3.5 12c0 2.3.9 4.4 2.4 6.2Z" {...common} /><path d="M8.2 12h.1M12 12h.1M15.8 12h.1" {...common} /></>;
      case "goals": return <><circle cx="12" cy="12" r="8" {...common} /><circle cx="12" cy="12" r="3" {...common} /><path d="m15 9-4.5 4.5" {...common} /></>;
      case "actions": return <><rect x="4" y="5" width="16" height="14" rx="2" {...common} /><path d="M8 10h8M8 14h5" {...common} /></>;
      case "history": return <><path d="M4 12a8 8 0 1 0 2.3-5.7L4 8.5" {...common} /><path d="M4 4.5v4h4M12 7v5l3 2" {...common} /></>;
      case "insights": return <><path d="M5 19V9M12 19V5M19 19v-7" {...common} /><path d="M3 19h18" {...common} /></>;
      case "settings": return <><circle cx="12" cy="12" r="3" {...common} /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.2 2.2-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5v.2h-3.2v-.2a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1-2.2-2.2.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H5v-3.2h.2a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1 2.2-2.2.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5V4h3.2v.2a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1 2.2 2.2-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.5 1h.2V14h-.2a1.7 1.7 0 0 0-1.5 1Z" {...common} /></>;
    }
  })();
  return <svg aria-hidden="true" className="nav-item__icon" viewBox="0 0 24 24">{path}</svg>;
}

function NavButton({ activePage, page, onNavigate }: { activePage: AppPage; page: AppPage; onNavigate: (page: AppPage) => void }) {
  return (
    <button
      aria-current={activePage === page ? "page" : undefined}
      className={activePage === page ? "nav-item nav-item--active" : "nav-item"}
      onClick={() => onNavigate(page)}
      type="button"
    >
      <NavIcon page={page} />
      <span>{pageLabels[page]}</span>
    </button>
  );
}

export function AppSidebar({ activePage, name, onNavigate }: AppSidebarProps) {
  const primaryPages = appPages.filter((page) => page !== "settings");

  return (
    <aside className="sidebar">
      <div className="brand" aria-label="VANTA Life — Personal Decision OS">
        <span className="brand__mark" aria-hidden="true">V</span>
        <span className="brand__wordmark">VANTA <em>Life</em><small>Personal Decision OS</small></span>
      </div>

      <nav className="sidebar__nav" aria-label="Main navigation">
        {primaryPages.map((page) => <NavButton activePage={activePage} key={page} onNavigate={onNavigate} page={page} />)}
      </nav>

      <div className="sidebar__footer">
        <nav aria-label="Application settings"><NavButton activePage={activePage} onNavigate={onNavigate} page="settings" /></nav>
        <div className="sidebar__profile">
          <span className="avatar" aria-hidden="true">{name.slice(0, 1).toUpperCase()}</span>
          <span><strong>{name}</strong><small>Private workspace</small></span>
        </div>
      </div>
    </aside>
  );
}
