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

const sectionDescriptions: Record<PageId, string> = {
  overview: "Runtime health, default key state, and capability inventory.",
  accounts: "Provider onboarding, auth health, and account-level capability checks.",
  relays: "Relay endpoints, balance visibility, and managed upstream paths.",
  keys: "Issued platform keys, policy bindings, and access mode control.",
  policies: "Routing decisions, candidate rejection reasons, and fallback policy authoring.",
  logs: "Diagnostics, request provenance, and detailed route explanations.",
};

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
              <div>
                <h1 className="workbench-brand__title">CodexLAG</h1>
              </div>
              <p className="workbench-brand__description">
                Single-machine operator workbench for provider auth, routing policy, platform
                keys, and runtime diagnostics.
              </p>
              <span className="workbench-brand__meta">Programmer-friendly dark workbench</span>
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
                <span className="workbench-nav__description">
                  {sectionDescriptions[section.id]}
                </span>
              </button>
            ))}
          </nav>
          <div className="workbench-rail-note">
            <strong>Operator posture</strong>
            Keep degraded providers visible, keep secrets local, and make route consequences easy
            to scan.
          </div>
        </div>
      }
    >
      {activeContent}
    </AppShell>
  );
}
