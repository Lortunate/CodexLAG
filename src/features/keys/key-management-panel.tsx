import { useState, type FormEvent } from "react";
import type { CreatePlatformKeyInput, PlatformKeyInventoryEntry } from "../../lib/types";

interface KeyManagementPanelProps {
  errorMessage: string | null;
  isCreating: boolean;
  keyActionId: string | null;
  keys: PlatformKeyInventoryEntry[];
  onCreate: (input: CreatePlatformKeyInput) => Promise<boolean>;
  onDisable: (keyId: string) => Promise<void>;
  onEnable: (keyId: string) => Promise<void>;
  successMessage: string | null;
}

interface KeyDraft {
  key_id: string;
  name: string;
  policy_id: string;
  allowed_mode: "hybrid" | "account_only" | "relay_only";
}

const initialDraft: KeyDraft = {
  key_id: "",
  name: "",
  policy_id: "",
  allowed_mode: "hybrid",
};

export function KeyManagementPanel({
  errorMessage,
  isCreating,
  keyActionId,
  keys,
  onCreate,
  onDisable,
  onEnable,
  successMessage,
}: KeyManagementPanelProps) {
  const [draft, setDraft] = useState<KeyDraft>(initialDraft);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const didCreate = await onCreate({
      key_id: draft.key_id.trim(),
      name: draft.name.trim(),
      policy_id: draft.policy_id.trim(),
      allowed_mode: draft.allowed_mode,
    });
    if (didCreate) {
      setDraft((current) => ({
        ...initialDraft,
        allowed_mode: current.allowed_mode,
        policy_id: current.policy_id,
      }));
    }
  }

  return (
    <section className="panel" aria-labelledby="key-management-heading">
      <h3 id="key-management-heading">Platform Key Management</h3>
      <p className="panel-intro">
        Mint local credentials against a specific policy lane and keep allowed runtime mode visible
        before the secret is issued.
      </p>
      <form onSubmit={handleSubmit}>
        <div className="form-grid">
          <label>
            Key ID
            <input
              name="key_id"
              value={draft.key_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, key_id: event.target.value }))
              }
            />
          </label>
        </div>
        <div className="form-grid">
          <label>
            Key Name
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
            Policy ID
            <input
              name="policy_id"
              value={draft.policy_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, policy_id: event.target.value }))
              }
            />
          </label>
        </div>
        <div className="form-grid">
          <label>
            Allowed Mode
            <select
              name="allowed_mode"
              value={draft.allowed_mode}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  allowed_mode: event.target.value as KeyDraft["allowed_mode"],
                }))
              }
            >
              <option value="hybrid">hybrid</option>
              <option value="account_only">account_only</option>
              <option value="relay_only">relay_only</option>
            </select>
          </label>
        </div>
        <button type="submit" disabled={isCreating}>
          Create key
        </button>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p role="status">{successMessage}</p> : null}
      <div className="panel-subsection">
        <h4>Issued credentials</h4>
        <p className="panel-intro">
          Track which key is enabled, which policy it binds to, and whether the runtime should admit
          account traffic, relay traffic, or both.
        </p>
      </div>
      <ul className="history-list" aria-label="Platform key inventory">
        {keys.map((key) => (
          <li key={key.id}>
            <div>
              <strong>{key.name}</strong>
              <p>ID: {key.id}</p>
              <p>Policy: {key.policy_id}</p>
              <p>Allowed mode: {key.allowed_mode}</p>
              <p>{key.enabled ? "Enabled" : "Disabled"}</p>
            </div>
            {key.enabled ? (
              <button
                type="button"
                disabled={keyActionId === key.id}
                onClick={() => {
                  onDisable(key.id);
                }}
              >
                Disable key {key.id}
              </button>
            ) : (
              <button
                type="button"
                disabled={keyActionId === key.id}
                onClick={() => {
                  onEnable(key.id);
                }}
              >
                Enable key {key.id}
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
