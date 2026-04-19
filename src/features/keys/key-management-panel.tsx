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

function modeLabel(mode: KeyDraft["allowed_mode"]) {
  switch (mode) {
    case "account_only":
      return "Account only";
    case "relay_only":
      return "Relay only";
    case "hybrid":
    default:
      return "Hybrid";
  }
}

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
      <div className="panel-heading">
        <div>
          <h3 id="key-management-heading">Platform Key Management</h3>
          <p>Create local credentials and keep policy bindings and allowed upstream modes readable.</p>
        </div>
      </div>
      <form className="operator-form" onSubmit={handleSubmit}>
        <div className="operator-fields">
          <div className="operator-field">
            <label htmlFor="platform-key-id">Key ID</label>
            <input
              id="platform-key-id"
              name="key_id"
              value={draft.key_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, key_id: event.target.value }))
              }
            />
          </div>
          <div className="operator-field">
            <label htmlFor="platform-key-name">Key Name</label>
            <input
              id="platform-key-name"
              name="name"
              value={draft.name}
              onChange={(event) =>
                setDraft((current) => ({ ...current, name: event.target.value }))
              }
            />
          </div>
          <div className="operator-field">
            <label htmlFor="platform-policy-id">Policy ID</label>
            <input
              id="platform-policy-id"
              name="policy_id"
              value={draft.policy_id}
              onChange={(event) =>
                setDraft((current) => ({ ...current, policy_id: event.target.value }))
              }
            />
          </div>
          <div className="operator-field">
            <label htmlFor="platform-allowed-mode">Allowed Mode</label>
            <select
              id="platform-allowed-mode"
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
          </div>
        </div>
        <div className="operator-form-actions">
          <button type="submit" disabled={isCreating}>
            {isCreating ? "Creating key..." : "Create key"}
          </button>
        </div>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p className="operator-success">{successMessage}</p> : null}
      <ul className="history-list" aria-label="Platform key inventory">
        {keys.map((key) => (
          <li key={key.id}>
            <div className="operator-stack">
              <div className="operator-list__item-header">
                <strong className="operator-list__item-title">{key.name}</strong>
                <code>{key.id}</code>
              </div>
              <dl className="operator-inline-pairs">
                <div>
                  <dt>Policy</dt>
                  <dd>{key.policy_id}</dd>
                </div>
                <div>
                  <dt>Allowed mode</dt>
                  <dd>{modeLabel(key.allowed_mode)}</dd>
                </div>
                <div>
                  <dt>Status</dt>
                  <dd>{key.enabled ? "Enabled" : "Disabled"}</dd>
                </div>
              </dl>
            </div>
            {key.enabled ? (
              <button
                type="button"
                disabled={keyActionId === key.id}
                onClick={() => {
                  onDisable(key.id);
                }}
              >
                {keyActionId === key.id ? "Disabling..." : `Disable key ${key.id}`}
              </button>
            ) : (
              <button
                type="button"
                disabled={keyActionId === key.id}
                onClick={() => {
                  onEnable(key.id);
                }}
              >
                {keyActionId === key.id ? "Enabling..." : `Enable key ${key.id}`}
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
