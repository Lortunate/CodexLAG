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

const sections: {
  id: PageId;
  label: string;
  navLabel: string;
  navDescription: string;
  heroDescription: string;
  signal: string;
}[] = [
  {
    id: "overview",
    label: "Gateway Overview",
    navLabel: "Overview",
    navDescription: "Runtime posture, key state, and capability inventory.",
    heroDescription: "Inspect the current control-plane posture before changing providers, keys, or policy.",
    signal: "Health-first",
  },
  {
    id: "accounts",
    label: "Official Accounts",
    navLabel: "Official Accounts",
    navDescription: "Browser auth, account health, and provider readiness.",
    heroDescription: "Keep provider identity, auth state, and account capability visible in one place.",
    signal: "Auth visibility",
  },
  {
    id: "relays",
    label: "Relay Inventory",
    navLabel: "Relays",
    navDescription: "Endpoint health, balance support, and connectivity checks.",
    heroDescription: "Review relay posture, test connectivity, and compare upstream balance support.",
    signal: "Relay posture",
  },
  {
    id: "keys",
    label: "Platform Keys",
    navLabel: "Platform Keys",
    navDescription: "Key issuance, policy binding, and runtime mode access.",
    heroDescription: "Issue local gateway credentials with explicit routing scope and access mode.",
    signal: "Access control",
  },
  {
    id: "policies",
    label: "Routing Policies",
    navLabel: "Policies",
    navDescription: "Ordering, failover, retry budgets, and recovery rules.",
    heroDescription: "Author runtime selection logic with clear consequences for fallback and recovery.",
    signal: "Policy authoring",
  },
  {
    id: "logs",
    label: "Logs & Usage",
    navLabel: "Logs & Usage",
    navDescription: "Provider diagnostics, request history, and usage provenance.",
    heroDescription: "Trace what happened, why it happened, and which upstream path consumed tokens.",
    signal: "Request explainability",
  },
];

export default function App() {
  const [activePage, setActivePage] = useState<PageId>("overview");
  const activeSection = sections.find((section) => section.id === activePage) ?? sections[0];

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
        <nav aria-label="Primary" className="shell-nav">
          <div className="shell-brand">
            <p className="shell-brand__eyebrow">Single-machine runtime</p>
            <h1 className="shell-brand__title">CodexLAG</h1>
            <p className="shell-brand__description">
              Cold-path visibility for auth, routing, and diagnostics.
            </p>
          </div>
          <div className="shell-nav__status">
            <p className="shell-nav__status-label">Operator workbench</p>
            <p className="shell-nav__status-copy">
              Local-first control over provider auth, policy, and runtime fallout.
            </p>
          </div>
          <div className="shell-nav__list">
            {sections.map((section) => (
              <button
                key={section.id}
                type="button"
                className={cn("shell-nav__button", section.id === activePage && "is-active")}
                aria-label={section.navLabel}
                aria-pressed={section.id === activePage ? "true" : "false"}
                aria-current={section.id === activePage ? "page" : undefined}
                onClick={() => setActivePage(section.id)}
              >
                <span className="shell-nav__label">{section.navLabel}</span>
                <span className="shell-nav__meta">{section.navDescription}</span>
              </button>
            ))}
          </div>
        </nav>
      }
    >
      <section className="workspace-hero" aria-label="Current workspace">
        <div className="workspace-hero__copy">
          <p className="workspace-hero__eyebrow">Operator workbench</p>
          <p className="workspace-hero__title">{activeSection.label}</p>
          <p className="workspace-hero__description">{activeSection.heroDescription}</p>
        </div>
        <dl className="workspace-hero__meta">
          <div>
            <dt>Scope</dt>
            <dd>Single-machine runtime</dd>
          </div>
          <div>
            <dt>Focus</dt>
            <dd>{activeSection.signal}</dd>
          </div>
        </dl>
      </section>
      {activeContent}
    </AppShell>
  );
}
