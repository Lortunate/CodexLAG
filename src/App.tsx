import { AccountsPage } from "./features/accounts/accounts-page";
import { KeysPage } from "./features/keys/keys-page";
import { LogsPage } from "./features/logs/logs-page";
import { OverviewPage } from "./features/overview/overview-page";
import { PoliciesPage } from "./features/policies/policies-page";
import { RelaysPage } from "./features/relays/relays-page";

const sections = [
  "Overview",
  "Official Accounts",
  "Relays",
  "Platform Keys",
  "Policies",
  "Logs & Usage",
];

export default function App() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <h1>CodexLAG</h1>
        <nav>
          {sections.map((section) => (
            <button key={section} type="button">
              {section}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        <OverviewPage />
        <AccountsPage />
        <RelaysPage />
        <KeysPage />
        <PoliciesPage />
        <LogsPage />
      </main>
    </div>
  );
}
