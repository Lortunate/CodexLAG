import type { ReactNode } from "react";

export function AppShell({
  navigation,
  children,
}: {
  navigation: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="app-shell">
      <div className="app-shell__frame">
        <aside className="app-shell__sidebar">
          {navigation}
        </aside>
        <main className="app-shell__main">
          <div className="app-shell__main-inner">{children}</div>
        </main>
      </div>
    </div>
  );
}
