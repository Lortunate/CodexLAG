import type { ReactNode } from "react";

export function AppShell({
  navigation,
  children,
}: {
  navigation: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="workbench-shell">
      <div className="workbench-shell__grid">
        <aside className="workbench-shell__rail">{navigation}</aside>
        <main className="workbench-shell__main">
          <div className="workbench-shell__main-inner">{children}</div>
        </main>
      </div>
    </div>
  );
}
