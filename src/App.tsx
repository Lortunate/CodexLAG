import { useState } from "react";
import { AppShell } from "./components/app-shell";
import { AccountsPage } from "./features/accounts/accounts-page";
import { KeysPage } from "./features/keys/keys-page";
import { LogsPage } from "./features/logs/logs-page";
import { OverviewPage } from "./features/overview/overview-page";
import { PoliciesPage } from "./features/policies/policies-page";
import { RelaysPage } from "./features/relays/relays-page";
import { cn } from "./lib/utils";

type PageId =
  | "overview"
  | "accounts"
  | "relays"
  | "keys"
  | "policies"
  | "logs";

const sections: { id: PageId; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "accounts", label: "Official Accounts" },
  { id: "relays", label: "Relays" },
  { id: "keys", label: "Platform Keys" },
  { id: "policies", label: "Policies" },
  { id: "logs", label: "Logs & Usage" },
];

export default function App() {
  const [activePage, setActivePage] = useState<PageId>("overview");

  let activeContent = <OverviewPage />;

  switch (activePage) {
    case "accounts":
      activeContent = <AccountsPage />;
      break;
    case "relays":
      activeContent = <RelaysPage />;
      break;
    case "keys":
      activeContent = <KeysPage />;
      break;
    case "policies":
      activeContent = <PoliciesPage />;
      break;
    case "logs":
      activeContent = <LogsPage />;
      break;
    case "overview":
    default:
      activeContent = <OverviewPage />;
      break;
  }

  return (
    <AppShell
      navigation={
        <nav aria-label="Primary" className="space-y-2">
          <div className="mb-8">
            <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">
              Local control plane
            </p>
            <h1 className="text-2xl font-semibold">CodexLAG</h1>
          </div>
          {sections.map((section) => (
            <button
              key={section.id}
              type="button"
              className={cn(
                "w-full rounded-xl px-4 py-3 text-left transition-colors",
                section.id === activePage
                  ? "bg-primary text-primary-foreground"
                  : "bg-transparent hover:bg-muted",
              )}
              aria-pressed={section.id === activePage ? "true" : "false"}
              onClick={() => setActivePage(section.id)}
            >
              {section.label}
            </button>
          ))}
        </nav>
      }
    >
      {activeContent}
    </AppShell>
  );
}
