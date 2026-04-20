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

const sections: { id: PageId; label: string; detail: string }[] = [
  { id: "overview", label: "Overview", detail: "Runtime status and default key posture" },
  { id: "accounts", label: "Official Accounts", detail: "Provider sessions, balances, and imports" },
  { id: "relays", label: "Relays", detail: "Relay inventory, health, and capability checks" },
  { id: "keys", label: "Platform Keys", detail: "Issued credentials and mode controls" },
  { id: "policies", label: "Policies", detail: "Routing rules and fallback priorities" },
  { id: "logs", label: "Logs & Usage", detail: "Request history, diagnostics, and export tools" },
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
      activePageLabel={sections.find((section) => section.id === activePage)?.label ?? "Overview"}
      navigation={
        <div className="workbench-shell__rail-inner">
          <div className="workbench-brand">
            <p className="workbench-brand__eyebrow">Local control plane</p>
            <h1 className="workbench-brand__title">CodexLAG</h1>
            <p className="workbench-brand__caption">Operator workbench for accounts, relays, keys, policy, and diagnostics</p>
          </div>
          <div className="workbench-rail-status" aria-label="Current workspace">
            <p className="workbench-rail-status__label">Active workspace</p>
            <p className="workbench-rail-status__value">
              {sections.find((section) => section.id === activePage)?.label ?? "Overview"}
            </p>
            <p className="workbench-rail-status__detail">
              {sections.find((section) => section.id === activePage)?.detail ?? "Runtime status and default key posture"}
            </p>
          </div>
          <nav aria-label="Primary" className="workbench-nav">
            <div className="workbench-nav__section-heading">
              <p className="workbench-nav__section-eyebrow">Workspaces</p>
              <p className="workbench-nav__section-caption">Persistent operator surfaces</p>
            </div>
            {sections.map((section) => (
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
                <span className="workbench-nav__content">
                  <span className="workbench-nav__label">{section.label}</span>
                  <span className="workbench-nav__detail">{section.detail}</span>
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
