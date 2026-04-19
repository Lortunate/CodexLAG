import { cn } from "../../lib/utils";
import type { DefaultKeyMode } from "../../lib/types";

const modes: DefaultKeyMode[] = ["account_only", "relay_only", "hybrid"];

interface DefaultKeyModeToggleProps {
  activeMode: DefaultKeyMode | null;
  disabled?: boolean;
  rawMode: string;
  unavailableReason?: string | null;
  summaryName: string;
  onSelectMode: (mode: DefaultKeyMode) => void;
}

function buildTraySummaryText(activeMode: DefaultKeyMode | null, rawMode: string) {
  const modeText = activeMode ?? `unsupported (${rawMode})`;
  return `Default key state | Current mode: ${modeText}`;
}

export function DefaultKeyModeToggle({
  activeMode,
  disabled = false,
  rawMode,
  unavailableReason = null,
  summaryName,
  onSelectMode,
}: DefaultKeyModeToggleProps) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <div>
          <h3>Default Key Mode</h3>
          <p>Choose which upstream path the default platform key is allowed to use.</p>
        </div>
      </div>
      <p>Default key: {summaryName}</p>
      <p>{buildTraySummaryText(activeMode, rawMode)}</p>
      <p>Allowed mode: {activeMode ?? `unsupported (${rawMode})`}</p>
      {unavailableReason ? <p role="status">{unavailableReason}</p> : null}
      <div className="mode-toggle-row">
        {modes.map((mode) => (
          <button
            key={mode}
            type="button"
            className={cn("mode-toggle-chip", mode === activeMode && "is-active")}
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
