import type { DefaultKeyMode } from "../../lib/types";

const modes: DefaultKeyMode[] = ["account_only", "relay_only", "hybrid"];

interface DefaultKeyModeToggleProps {
  activeMode: DefaultKeyMode | null;
  disabled?: boolean;
  rawMode: string;
  summaryName: string;
  onSelectMode: (mode: DefaultKeyMode) => void;
}

export function DefaultKeyModeToggle({
  activeMode,
  disabled = false,
  rawMode,
  summaryName,
  onSelectMode,
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
            disabled={disabled || mode === activeMode}
            onClick={() => onSelectMode(mode)}
          >
            {mode}
          </button>
        ))}
      </div>
    </section>
  );
}
