const sections = [
  "Overview",
  "Official Accounts",
  "Relays",
  "Platform Keys",
  "Policies",
  "Logs & Usage"
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
        <h2>Gateway Shell</h2>
        <p>Windows-first local Codex gateway desktop console.</p>
      </main>
    </div>
  );
}
