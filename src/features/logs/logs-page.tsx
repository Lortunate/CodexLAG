import { useEffect, useState } from "react";
import { getLogSummary } from "../../lib/tauri";
import type { LogSummary } from "../../lib/types";

export function LogsPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [summary, setSummary] = useState<LogSummary | null>(null);

  useEffect(() => {
    let isMounted = true;

    getLogSummary()
      .then((nextSummary) => {
        if (isMounted) {
          setSummary(nextSummary);
          setErrorMessage(null);
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to load logs.");
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="logs-heading">
      <h2 id="logs-heading">Usage Timeline</h2>
      <p>Use desktop-visible logs to monitor the gateway and review recent usage signals.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {summary ? (
        <dl>
          <div>
            <dt>Level</dt>
            <dd>{summary.level}</dd>
          </div>
          <div>
            <dt>Last event</dt>
            <dd>{summary.last_event}</dd>
          </div>
        </dl>
      ) : null}
    </section>
  );
}
