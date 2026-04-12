import { useEffect, useState } from "react";
import { getRelayCapabilityDetail, listRelays, refreshRelayBalance } from "../../lib/tauri";
import type {
  RelayBalanceSnapshot,
  RelayCapabilityDetail,
  RelaySummary,
} from "../../lib/types";

interface RelayPanelState {
  relay: RelaySummary;
  balanceError: string | null;
  balanceSnapshot: RelayBalanceSnapshot | null;
  capabilityDetail: RelayCapabilityDetail | null;
  capabilityError: string | null;
}

export function RelaysPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [relays, setRelays] = useState<RelayPanelState[]>([]);

  useEffect(() => {
    let isMounted = true;

    const loadRelays = async () => {
      try {
        const summaries = await listRelays();
        const relayPanels = await Promise.all(
          summaries.map(async (relay) => {
            const panelState: RelayPanelState = {
              relay,
              balanceError: null,
              balanceSnapshot: null,
              capabilityDetail: null,
              capabilityError: null,
            };

            try {
              panelState.balanceSnapshot = await refreshRelayBalance(relay.relay_id);
            } catch (error) {
              panelState.balanceError =
                error instanceof Error ? error.message : "Failed to refresh relay balance.";
            }

            try {
              panelState.capabilityDetail = await getRelayCapabilityDetail(relay.relay_id);
            } catch (error) {
              panelState.capabilityError =
                error instanceof Error ? error.message : "Failed to load relay capability detail.";
            }

            return panelState;
          }),
        );

        if (isMounted) {
          setRelays(relayPanels);
          setErrorMessage(null);
        }
      } catch {
        if (isMounted) {
          setErrorMessage("Failed to load relays.");
        }
      }
    };

    loadRelays();

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="relays-heading">
      <h2 id="relays-heading">Relay Status</h2>
      <p>Review upstream relay targets and the local endpoint used by the desktop shell.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="detail-grid">
        {relays.map((panel) => (
          <article className="detail-card" key={panel.relay.relay_id}>
            <h3>{panel.relay.name}</h3>
            <p>Endpoint: {panel.relay.endpoint}</p>
            {panel.balanceSnapshot ? (
              <>
                <p>Balance state: {panel.balanceSnapshot.balance.kind}</p>
                {panel.balanceSnapshot.balance.kind === "queryable" ? (
                  <>
                    <p>Adapter: {panel.balanceSnapshot.balance.adapter}</p>
                    <p>Total: {panel.balanceSnapshot.balance.balance.total}</p>
                    <p>Used: {panel.balanceSnapshot.balance.balance.used}</p>
                  </>
                ) : (
                  <p>{panel.balanceSnapshot.balance.reason}</p>
                )}
              </>
            ) : (
              <p>{panel.balanceError ?? "Balance unavailable."}</p>
            )}
            {panel.capabilityDetail ? (
              panel.capabilityDetail.balance_capability.kind === "queryable" ? (
                <p>Capability: queryable ({panel.capabilityDetail.balance_capability.adapter})</p>
              ) : (
                <p>Capability: unsupported</p>
              )
            ) : (
              <p>{panel.capabilityError ?? "Capability detail unavailable."}</p>
            )}
          </article>
        ))}
      </div>
    </section>
  );
}
