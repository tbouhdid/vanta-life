import type { ReactNode } from "react";

type StateMessageProps = {
  title?: string;
  children: ReactNode;
  action?: ReactNode;
};

export function LoadingState({ title = "Loading your life state…" }: { title?: string }) {
  return (
    <div className="state-card state-card--loading" role="status" aria-live="polite">
      <span className="loading-dot" aria-hidden="true" />
      <div>
        <p>{title}</p>
        <span aria-hidden="true" className="skeleton-lines"><i /><i /><i /></span>
      </div>
    </div>
  );
}

export function EmptyState({ title, children, action }: StateMessageProps) {
  return (
    <div className="state-card state-card--empty">
      {title && <h3>{title}</h3>}
      <p>{children}</p>
      {action && <div className="state-card__action">{action}</div>}
    </div>
  );
}

export function ErrorState({ title = "Something went wrong", children, action }: StateMessageProps) {
  return (
    <div className="state-card state-card--error" role="alert">
      <h2>{title}</h2>
      <p>{children}</p>
      {action && <div className="state-card__action">{action}</div>}
    </div>
  );
}

export function InlineError({ children }: { children: ReactNode }) {
  return (
    <p className="inline-error" role="alert">
      {children}
    </p>
  );
}

export function PageHeader({
  eyebrow,
  title,
  children,
}: {
  eyebrow?: string;
  title: string;
  children?: ReactNode;
}) {
  return (
    <header className="page-header">
      <div>
        {eyebrow && <p className="eyebrow">{eyebrow}</p>}
        <h1>{title}</h1>
      </div>
      {children && <div className="page-header__actions">{children}</div>}
    </header>
  );
}
