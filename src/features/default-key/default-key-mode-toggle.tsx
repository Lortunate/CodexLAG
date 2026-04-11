import type { DefaultKeyMode } from "../../lib/types";

const modes: DefaultKeyMode[] = ["account_only", "relay_only", "hybrid"];

export function DefaultKeyModeToggle() {
  return (
    <section>
      <h3>Default Key Mode</h3>
      <div>
        {modes.map((mode) => (
          <button key={mode} type="button">
            {mode}
          </button>
        ))}
      </div>
    </section>
  );
}
