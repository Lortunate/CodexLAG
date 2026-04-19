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
                <p className="workbench-topbar__eyebrow">Workspace</p>
                <div className="workbench-topbar__title-row">
                  <h2 className="workbench-topbar__title">{activePageLabel}</h2>
                  <span className="workbench-topbar__chip">Desktop operator</span>
                </div>
              </div>
              <div className="workbench-topbar__meta">
                <span className="workbench-topbar__meta-label">Single-machine control plane</span>
              </div>
            </header>
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
