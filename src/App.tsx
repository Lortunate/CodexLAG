import { useState } from "react";
import { AccountsPage } from "./features/accounts/accounts-page";
import { KeysPage } from "./features/keys/keys-page";
import { LogsPage } from "./features/logs/logs-page";
import { OverviewPage } from "./features/overview/overview-page";
import { PoliciesPage } from "./features/policies/policies-page";
import { RelaysPage } from "./features/relays/relays-page";

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
    <div className="app-shell">
      <aside className="sidebar">
        <h1>CodexLAG</h1>
        <nav>
          {sections.map((section) => (
            <button
              key={section.id}
              type="button"
              aria-pressed={section.id === activePage ? "true" : "false"}
              onClick={() => setActivePage(section.id)}
            >
              {section.label}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">{activeContent}</main>
    </div>
  );
}
