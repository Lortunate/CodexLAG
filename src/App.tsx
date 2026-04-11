import React from "react";

const primarySections = [
  {
    title: "Overview",
    description: "Quick status of relays, usage, and alerts across your Tauri setup."
  },
  {
    title: "Official Accounts",
    description: "Manage the brands, identities, and credentials that represent your organization."
  },
  {
    title: "Relays",
    description: "Track relay health, latency, and event throughput in real time."
  },
  {
    title: "Platform Keys",
    description: "Inspect API credentials, rotate secrets, and audit key usage."
  },
  {
    title: "Policies",
    description: "Review security policies, access levels, and compliance checks."
  },
  {
    title: "Logs & Usage",
    description: "Browse detailed logs, usage charts, and billing-related events."
  }
];

const App = () => {
  return (
    <div className="app-shell">
      <header>
        <p className="eyebrow">Tauri Desktop Control</p>
        <h1>Codex Gateway</h1>
        <p className="lede">
          Everything you need to understand the state of your engine, right from this shell.
        </p>
      </header>

      <nav className="app-nav" aria-label="Primary navigation">
        {primarySections.map((section) => (
          <article key={section.title} className="nav-card">
            <h2>{section.title}</h2>
            <p>{section.description}</p>
          </article>
        ))}
      </nav>
    </div>
  );
};

export default App;
