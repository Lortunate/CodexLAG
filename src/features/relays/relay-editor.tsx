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
      <div className="panel-heading">
        <div>
          <h3 id="relay-editor-heading">Manage Relays</h3>
          <p>Register managed upstreams, then run quick connection checks without leaving the page.</p>
        </div>
      </div>
      <form className="operator-form" onSubmit={handleSubmit}>
        <div className="operator-fields">
          <div className="operator-field">
            <label htmlFor="relay-id">Relay ID</label>
            <input
              id="relay-id"
              name="relay_id"
              value={draft.relay_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, relay_id: event.target.value }))
              }
            />
          </div>
          <div className="operator-field">
            <label htmlFor="relay-name">Relay Name</label>
            <input
              id="relay-name"
              name="name"
              value={draft.name}
              onChange={(event) =>
                setDraft((current) => ({ ...current, name: event.target.value }))
              }
            />
          </div>
          <div className="operator-field operator-field--full">
            <label htmlFor="relay-endpoint">Relay Endpoint</label>
            <input
              id="relay-endpoint"
              name="endpoint"
              value={draft.endpoint}
              onChange={(event) =>
                setDraft((current) => ({ ...current, endpoint: event.target.value }))
              }
            />
            <span className="operator-field-help">
              Use the exact loopback or managed upstream URL the gateway should test and route through.
            </span>
          </div>
          <div className="operator-field">
            <label htmlFor="relay-adapter">Adapter</label>
            <select
              id="relay-adapter"
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
          </div>
        </div>
        <div className="operator-form-actions">
          <button type="submit" disabled={isCreating}>
            {isCreating ? "Creating relay..." : "Create relay"}
          </button>
        </div>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p className="operator-success">{successMessage}</p> : null}
      <ul className="history-list" aria-label="Relay management list">
        {relays.map((relay) => {
          const result = connectionResults[relay.relay_id];
          return (
            <li key={relay.relay_id}>
              <div className="operator-stack">
                <div className="operator-list__item-header">
                  <strong className="operator-list__item-title">{relay.name}</strong>
                  <code>{relay.relay_id}</code>
                </div>
                <p>{relay.endpoint}</p>
                {result ? (
                  <>
                    <p>{result.relay_id}: {result.status} ({result.latency_ms}ms)</p>
                    <dl className="operator-inline-pairs">
                      <div>
                        <dt>Status</dt>
                        <dd>{result.status}</dd>
                      </div>
                      <div>
                        <dt>Latency</dt>
                        <dd>{result.latency_ms}ms</dd>
                      </div>
                    </dl>
                  </>
                ) : null}
              </div>
              <button
                type="button"
                disabled={isTestingRelayId === relay.relay_id}
                onClick={() => {
                  onTest(relay.relay_id);
                }}
              >
                {isTestingRelayId === relay.relay_id
                  ? `Testing ${relay.relay_id}...`
                  : `Test relay ${relay.relay_id}`}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
