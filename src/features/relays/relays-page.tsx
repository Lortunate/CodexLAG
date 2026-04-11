import { useEffect, useState } from "react";
import { listRelays } from "../../lib/tauri";
import type { RelaySummary } from "../../lib/types";

export function RelaysPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [relays, setRelays] = useState<RelaySummary[]>([]);

  useEffect(() => {
    let isMounted = true;

    listRelays()
      .then((nextRelays) => {
        if (isMounted) {
          setRelays(nextRelays);
          setErrorMessage(null);
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to load relays.");
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="relays-heading">
      <h2 id="relays-heading">Relay Status</h2>
      <p>Review upstream relay targets and the local endpoint used by the desktop shell.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <ul>
        {relays.map((relay) => (
          <li key={relay.name}>
            <strong>{relay.name}</strong>
            <span>{relay.endpoint}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
