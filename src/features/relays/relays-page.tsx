import { useEffect, useState } from "react";
import {
  addRelay,
  getRelayCapabilityDetail,
  listRelays,
  refreshRelayBalance,
  testRelayConnection,
} from "../../lib/tauri";
import type {
  RelayConnectionTestResult,
  RelayBalanceSnapshot,
  RelayCapabilityDetail,
  RelaySummary,
  RelayUpsertInput,
} from "../../lib/types";
import { PageHeader } from "../../components/page-header";
import { RelayEditor } from "./relay-editor";

interface RelayPanelState {
  relay: RelaySummary;
  balanceError: string | null;
  balanceSnapshot: RelayBalanceSnapshot | null;
  capabilityDetail: RelayCapabilityDetail | null;
  capabilityError: string | null;
}

export function RelaysPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [editorErrorMessage, setEditorErrorMessage] = useState<string | null>(null);
  const [editorSuccessMessage, setEditorSuccessMessage] = useState<string | null>(null);
  const [isCreatingRelay, setIsCreatingRelay] = useState(false);
  const [testingRelayId, setTestingRelayId] = useState<string | null>(null);
  const [relayConnectionResults, setRelayConnectionResults] = useState<
    Record<string, RelayConnectionTestResult>
  >({});
  const [relays, setRelays] = useState<RelayPanelState[]>([]);

  async function loadRelays(isMounted: () => boolean = () => true) {
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

      if (isMounted()) {
        setRelays(relayPanels);
        setErrorMessage(null);
      }
    } catch {
      if (isMounted()) {
        setErrorMessage("Failed to load relays.");
      }
    }
  }

  useEffect(() => {
    let isMounted = true;
    void loadRelays(() => isMounted);
    return () => {
      isMounted = false;
    };
  }, []);

  async function handleCreateRelay(input: RelayUpsertInput): Promise<boolean> {
    if (isCreatingRelay) {
      return false;
    }

    setIsCreatingRelay(true);
    setEditorErrorMessage(null);
    setEditorSuccessMessage(null);
    try {
      const created = await addRelay(input);
      await loadRelays();
      setEditorSuccessMessage(`Created relay: ${created.relay_id}`);
      return true;
    } catch (error) {
      setEditorErrorMessage(error instanceof Error ? error.message : "Failed to create relay.");
      return false;
    } finally {
      setIsCreatingRelay(false);
    }
  }

  async function handleTestRelay(relayId: string) {
    if (testingRelayId) {
      return;
    }

    setTestingRelayId(relayId);
    setEditorErrorMessage(null);
    try {
      const result = await testRelayConnection(relayId);
      setRelayConnectionResults((current) => ({
        ...current,
        [relayId]: result,
      }));
    } catch (error) {
      setEditorErrorMessage(
        error instanceof Error ? error.message : `Failed to test relay ${relayId}.`,
      );
    } finally {
      setTestingRelayId(null);
    }
  }

  return (
    <section className="workspace-page" aria-labelledby="relays-heading">
      <PageHeader
        eyebrow="Relay inventory"
        title="Relays"
        titleId="relays-heading"
        description="Manage NewAPI relay endpoints, validate connectivity, and compare which relays expose usable balance signals."
        meta={`${relays.length} configured relay${relays.length === 1 ? "" : "s"}`}
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <RelayEditor
        connectionResults={relayConnectionResults}
        errorMessage={editorErrorMessage}
        isCreating={isCreatingRelay}
        isTestingRelayId={testingRelayId}
        relays={relays.map((panel) => panel.relay)}
        successMessage={editorSuccessMessage}
        onCreate={handleCreateRelay}
        onTest={handleTestRelay}
      />
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
