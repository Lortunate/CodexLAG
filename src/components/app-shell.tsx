import type { ReactNode } from "react";

export function AppShell({
  navigation,
  activePageLabel,
  children,
}: {
  navigation: ReactNode;
  activePageLabel: string;
  children: ReactNode;
}) {
  return (
    <div className="workbench-shell">
      <div className="workbench-shell__grid">
        <aside className="workbench-shell__rail">{navigation}</aside>
        <main className="workbench-shell__main">
          <div className="workbench-shell__main-inner">
            <header className="workbench-topbar" aria-label="Workspace header">
              <div className="workbench-topbar__context">
                <p className="workbench-topbar__eyebrow">CodexLAG operator workbench</p>
                <div className="workbench-topbar__title-block">
                  <h2 className="workbench-topbar__title">{activePageLabel}</h2>
                  <p className="workbench-topbar__description">
                    Local desktop control surface with durable navigation and dense operational context.
                  </p>
                </div>
              </div>
              <dl className="workbench-topbar__summary" aria-label="Workspace framing">
                <div>
                  <dt>Surface</dt>
                  <dd>{activePageLabel}</dd>
                </div>
                <div>
                  <dt>Session</dt>
                  <dd>Desktop operator</dd>
                </div>
                <div>
                  <dt>Scope</dt>
                  <dd>Accounts, relays, keys, policy, diagnostics</dd>
                </div>
              </dl>
            </header>
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
