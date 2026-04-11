import { useEffect, useState } from "react";
import { getDefaultKeySummary } from "../../lib/tauri";
import type { DefaultKeySummary } from "../../lib/types";
import { DefaultKeyModeToggle } from "../default-key/default-key-mode-toggle";

const initialSummary: DefaultKeySummary = {
  name: "loading",
  allowedMode: null,
  rawAllowedMode: "loading",
};

export function OverviewPage() {
  const [summary, setSummary] = useState<DefaultKeySummary>(initialSummary);

  useEffect(() => {
    let isMounted = true;

    getDefaultKeySummary().then((nextSummary) => {
      if (isMounted) {
        setSummary(nextSummary);
      }
    });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="overview-heading">
      <h2 id="overview-heading">Gateway Overview</h2>
      <p>CodexLAG manages local accounts, relays, keys, policy routing, and logs.</p>
      <DefaultKeyModeToggle
        activeMode={summary.allowedMode}
        rawMode={summary.rawAllowedMode}
        summaryName={summary.name}
      />
    </section>
  );
}
