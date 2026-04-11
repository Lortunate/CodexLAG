import type { DefaultKeyMode } from "../../lib/types";

const modes: DefaultKeyMode[] = ["account_only", "relay_only", "hybrid"];

interface DefaultKeyModeToggleProps {
  activeMode: DefaultKeyMode | null;
  rawMode: string;
  summaryName: string;
}

export function DefaultKeyModeToggle({
  activeMode,
  rawMode,
  summaryName,
}: DefaultKeyModeToggleProps) {
  return (
    <section>
      <h3>Default Key Mode</h3>
      <p>Default key: {summaryName}</p>
      <p>Allowed mode: {activeMode ?? `unsupported (${rawMode})`}</p>
      <div>
        {modes.map((mode) => (
          <button
            key={mode}
            type="button"
            aria-pressed={mode === activeMode ? "true" : "false"}
          >
            {mode}
          </button>
        ))}
      </div>
    </section>
  );
}
