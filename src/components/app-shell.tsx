import type { ReactNode } from "react";

export function AppShell({
  navigation,
  children,
}: {
  navigation: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="min-h-screen bg-[var(--app-bg)] text-[var(--app-fg)]">
      <div className="grid min-h-screen grid-cols-1 lg:grid-cols-[280px_1fr]">
        <aside className="border-b border-border/50 bg-card/60 p-6 lg:border-b-0 lg:border-r">
          {navigation}
        </aside>
        <main className="min-w-0 p-6 lg:p-8">{children}</main>
      </div>
    </div>
  );
}
