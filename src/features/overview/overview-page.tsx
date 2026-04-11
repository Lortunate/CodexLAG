import { useEffect, useState } from "react";
import {
  getDefaultKeySummary,
  listenForDefaultKeySummaryChanged,
  setDefaultKeyMode,
} from "../../lib/tauri";
import type { DefaultKeySummary } from "../../lib/types";
import { DefaultKeyModeToggle } from "../default-key/default-key-mode-toggle";

const initialSummary: DefaultKeySummary = {
  name: "loading",
  allowedMode: null,
  rawAllowedMode: "loading",
};

export function OverviewPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isUpdatingMode, setIsUpdatingMode] = useState(false);
  const [summary, setSummary] = useState<DefaultKeySummary>(initialSummary);

  useEffect(() => {
    let isMounted = true;
    let disposeListener: (() => void) | null = null;

    getDefaultKeySummary()
      .then((nextSummary) => {
        if (isMounted) {
          setSummary(nextSummary);
          setErrorMessage(null);
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to load default key mode.");
        }
      });

    listenForDefaultKeySummaryChanged((nextSummary) => {
      if (isMounted) {
        setSummary(nextSummary);
        setErrorMessage(null);
      }
    })
      .then((unlisten) => {
        if (isMounted) {
          disposeListener = unlisten;
        } else {
          unlisten();
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to subscribe to default key mode updates.");
        }
      });

    return () => {
      isMounted = false;
      disposeListener?.();
    };
  }, []);

  async function handleSelectMode(mode: "account_only" | "relay_only" | "hybrid") {
    if (isUpdatingMode || summary.allowedMode === mode) {
      return;
    }

    setIsUpdatingMode(true);
    try {
      const nextSummary = await setDefaultKeyMode(mode);
      setSummary(nextSummary);
      setErrorMessage(null);
    } catch {
      setErrorMessage("Failed to update default key mode.");
    } finally {
      setIsUpdatingMode(false);
    }
  }

  return (
    <section aria-labelledby="overview-heading">
      <h2 id="overview-heading">Gateway Overview</h2>
      <p>CodexLAG manages local accounts, relays, keys, policy routing, and logs.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <DefaultKeyModeToggle
        activeMode={summary.allowedMode}
        disabled={isUpdatingMode}
        rawMode={summary.rawAllowedMode}
        summaryName={summary.name}
        onSelectMode={handleSelectMode}
      />
    </section>
  );
}
