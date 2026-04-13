import { useState, type FormEvent } from "react";
import type { RelayConnectionTestResult, RelaySummary, RelayUpsertInput } from "../../lib/types";

interface RelayEditorProps {
  connectionResults: Record<string, RelayConnectionTestResult>;
  errorMessage: string | null;
  isCreating: boolean;
  isTestingRelayId: string | null;
  onCreate: (input: RelayUpsertInput) => Promise<void>;
  onTest: (relayId: string) => Promise<void>;
  relays: RelaySummary[];
  successMessage: string | null;
}

interface RelayDraft {
  relay_id: string;
  name: string;
  endpoint: string;
  adapter: string;
}

const initialDraft: RelayDraft = {
  relay_id: "",
  name: "",
  endpoint: "",
  adapter: "new_api",
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
    await onCreate({
      relay_id: draft.relay_id.trim(),
      name: draft.name.trim(),
      endpoint: draft.endpoint.trim(),
      adapter: draft.adapter.trim(),
    });
    setDraft((current) => ({
      ...initialDraft,
      adapter: current.adapter,
    }));
  }

  return (
    <section className="panel" aria-labelledby="relay-editor-heading">
      <h3 id="relay-editor-heading">Manage Relays</h3>
      <form onSubmit={handleSubmit}>
        <p>
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
        </p>
        <p>
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
        </p>
        <p>
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
        </p>
        <p>
          <label>
            Adapter
            <select
              name="adapter"
              value={draft.adapter}
              onChange={(event) =>
                setDraft((current) => ({ ...current, adapter: event.target.value }))
              }
            >
              <option value="new_api">new_api</option>
              <option value="no_balance">no_balance</option>
            </select>
          </label>
        </p>
        <button type="submit" disabled={isCreating}>
          Create relay
        </button>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p>{successMessage}</p> : null}
      <ul className="history-list" aria-label="Relay management list">
        {relays.map((relay) => {
          const result = connectionResults[relay.relay_id];
          return (
            <li key={relay.relay_id}>
              <div>
                <strong>{relay.name}</strong>
                <p>{relay.endpoint}</p>
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
