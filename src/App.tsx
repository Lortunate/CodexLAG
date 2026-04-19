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
        <div className="workbench-shell__rail-inner">
          <nav aria-label="Primary" className="workbench-nav">
            <div className="workbench-brand">
              <p className="workbench-brand__eyebrow">Local control plane</p>
              <h1 className="workbench-brand__title">CodexLAG</h1>
            </div>
            {sections.map((section, index) => (
              <button
                key={section.id}
                type="button"
                className={cn(
                  "workbench-nav__item",
                  section.id === activePage && "is-active",
                )}
                aria-label={section.label}
                aria-pressed={section.id === activePage ? "true" : "false"}
                onClick={() => setActivePage(section.id)}
              >
                <span className="workbench-nav__label-row">
                  <span className="workbench-nav__index">{String(index + 1).padStart(2, "0")}</span>
                  <span className="workbench-nav__label">{section.label}</span>
                </span>
              </button>
            ))}
          </nav>
        </div>
      }
    >
      {activeContent}
    </AppShell>
  );
}
