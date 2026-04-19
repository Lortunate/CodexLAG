import { useState, type FormEvent } from "react";
import type { RelayConnectionTestResult, RelaySummary, RelayUpsertInput } from "../../lib/types";

interface RelayEditorProps {
  connectionResults: Record<string, RelayConnectionTestResult>;
  errorMessage: string | null;
  isCreating: boolean;
  isTestingRelayId: string | null;
  onCreate: (input: RelayUpsertInput) => Promise<boolean>;
  onTest: (relayId: string) => Promise<void>;
  relays: RelaySummary[];
  successMessage: string | null;
}

interface RelayDraft {
  relay_id: string;
  name: string;
  endpoint: string;
  adapter: "newapi" | "none";
}

const initialDraft: RelayDraft = {
  relay_id: "",
  name: "",
  endpoint: "",
  adapter: "newapi",
};

export function RelayEditor({
  connectionResults,
  errorMessage,
  isCreating,
  isTestingRelayId,
  onCreate,
  onTest,
  relays,
  successMessage,
}: RelayEditorProps) {
  const [draft, setDraft] = useState<RelayDraft>(initialDraft);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const didCreate = await onCreate({
      relay_id: draft.relay_id.trim(),
      name: draft.name.trim(),
      endpoint: draft.endpoint.trim(),
      adapter: draft.adapter,
    });
    if (didCreate) {
      setDraft((current) => ({
        ...initialDraft,
        adapter: current.adapter,
      }));
    }
  }

  return (
    <section className="panel" aria-labelledby="relay-editor-heading">
      <h3 id="relay-editor-heading">Manage Relays</h3>
      <p className="panel-intro">
        Stage relay definitions, keep adapter shape explicit, and verify connectivity before the
        relay is allowed into rotation.
      </p>
      <form onSubmit={handleSubmit}>
        <div className="form-grid">
          <label>
            Relay ID
            <input
              name="relay_id"
              value={draft.relay_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, relay_id: event.target.value }))
              }
            />
          </label>
        </div>
        <div className="form-grid">
          <label>
            Relay Name
            <input
              name="name"
              value={draft.name}
              onChange={(event) =>
                setDraft((current) => ({ ...current, name: event.target.value }))
              }
            />
          </label>
        </div>
        <div className="form-grid">
          <label>
            Relay Endpoint
            <input
              name="endpoint"
              value={draft.endpoint}
              onChange={(event) =>
                setDraft((current) => ({ ...current, endpoint: event.target.value }))
              }
            />
          </label>
        </div>
        <div className="form-grid">
          <label>
            Adapter
            <select
              name="adapter"
              value={draft.adapter}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  adapter: event.target.value as RelayDraft["adapter"],
                }))
              }
            >
              <option value="newapi">newapi</option>
              <option value="none">none</option>
            </select>
          </label>
        </div>
        <button type="submit" disabled={isCreating}>
          Create relay
        </button>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p role="status">{successMessage}</p> : null}
      <div className="panel-subsection">
        <h4>Connection verification</h4>
        <p className="panel-intro">
          Run point-in-time checks against configured relays and keep the latest latency result
          attached to each endpoint record.
        </p>
      </div>
      <ul className="history-list" aria-label="Relay management list">
        {relays.map((relay) => {
          const result = connectionResults[relay.relay_id];
          return (
            <li key={relay.relay_id}>
              <div>
                <strong>{relay.name}</strong>
                <p>Endpoint: {relay.endpoint}</p>
                <p>Relay ID: {relay.relay_id}</p>
                {result ? (
                  <p>
                    {result.relay_id}: {result.status} ({result.latency_ms}ms)
                  </p>
                ) : null}
              </div>
              <button
                type="button"
                disabled={isTestingRelayId === relay.relay_id}
                onClick={() => {
                  onTest(relay.relay_id);
                }}
              >
                Test relay {relay.relay_id}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
