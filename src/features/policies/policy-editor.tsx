import { useEffect, useState, type FormEvent } from "react";
import type { PolicySummary, PolicyUpdateInput } from "../../lib/types";

interface PolicyEditorProps {
  endpointIds: string[];
  errorMessage: string | null;
  isSaving: boolean;
  onSave: (input: PolicyUpdateInput) => Promise<void>;
  policies: PolicySummary[];
  successMessage: string | null;
}

interface PolicyDraft {
  policy_id: string;
  name: string;
  selection_order: string;
  cross_pool_fallback: boolean;
  retry_budget: string;
  timeout_open_after: string;
  server_error_open_after: string;
  cooldown_ms: string;
  half_open_after_ms: string;
  success_close_after: string;
}

function defaultSelectionOrder(endpointIds: string[]) {
  return endpointIds.join(", ");
}

function toPositiveInteger(raw: string, fallback: number) {
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

export function PolicyEditor({
  endpointIds,
  errorMessage,
  isSaving,
  onSave,
  policies,
  successMessage,
}: PolicyEditorProps) {
  const [draft, setDraft] = useState<PolicyDraft>({
    policy_id: "",
    name: "",
    selection_order: defaultSelectionOrder(endpointIds),
    cross_pool_fallback: false,
    retry_budget: "2",
    timeout_open_after: "3",
    server_error_open_after: "3",
    cooldown_ms: "1000",
    half_open_after_ms: "1000",
    success_close_after: "2",
  });

  useEffect(() => {
    if (policies.length === 0) {
      return;
    }

    setDraft((current) => {
      const activePolicy =
        policies.find((policy) => policy.policy_id === current.policy_id) ?? policies[0];
      const hasName = current.name.trim().length > 0;
      const hasSelectionOrder = current.selection_order.trim().length > 0;
      return {
        ...current,
        policy_id: activePolicy.policy_id,
        name: hasName ? current.name : activePolicy.name,
        selection_order: hasSelectionOrder
          ? current.selection_order
          : defaultSelectionOrder(endpointIds),
      };
    });
  }, [endpointIds, policies]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    await onSave({
      policy_id: draft.policy_id,
      name: draft.name.trim(),
      selection_order: draft.selection_order
        .split(",")
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0),
      cross_pool_fallback: draft.cross_pool_fallback,
      retry_budget: toPositiveInteger(draft.retry_budget, 2),
      timeout_open_after: toPositiveInteger(draft.timeout_open_after, 3),
      server_error_open_after: toPositiveInteger(draft.server_error_open_after, 3),
      cooldown_ms: toPositiveInteger(draft.cooldown_ms, 1000),
      half_open_after_ms: toPositiveInteger(draft.half_open_after_ms, 1000),
      success_close_after: toPositiveInteger(draft.success_close_after, 2),
    });
  }

  return (
    <section className="panel" aria-labelledby="policy-editor-heading">
      <h3 id="policy-editor-heading">Policy Editor</h3>
      <ul aria-label="Policy summaries">
        {policies.map((policy) => (
          <li key={policy.policy_id}>
            <strong>{policy.name}</strong> <span>{policy.status}</span>
          </li>
        ))}
      </ul>
      <form onSubmit={handleSubmit}>
        <p>
          <label>
            Policy
            <select
              value={draft.policy_id}
              onChange={(event) => {
                const selectedPolicy = policies.find((policy) => policy.policy_id === event.target.value);
                setDraft((current) => ({
                  ...current,
                  policy_id: event.target.value,
                  name: selectedPolicy?.name ?? current.name,
                }));
              }}
            >
              {policies.map((policy) => (
                <option key={policy.policy_id} value={policy.policy_id}>
                  {policy.name}
                </option>
              ))}
            </select>
          </label>
        </p>
        <p>
          <label>
            Policy Name
            <input
              value={draft.name}
              onChange={(event) => {
                setDraft((current) => ({ ...current, name: event.target.value }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Selection Order
            <input
              value={draft.selection_order}
              onChange={(event) => {
                setDraft((current) => ({
                  ...current,
                  selection_order: event.target.value,
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            <input
              type="checkbox"
              checked={draft.cross_pool_fallback}
              onChange={(event) => {
                setDraft((current) => ({
                  ...current,
                  cross_pool_fallback: event.target.checked,
                }));
              }}
            />
            Cross Pool Fallback
          </label>
        </p>
        <p>
          <label>
            Retry Budget
            <input
              value={draft.retry_budget}
              onChange={(event) => {
                setDraft((current) => ({ ...current, retry_budget: event.target.value }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Timeout Open After
            <input
              value={draft.timeout_open_after}
              onChange={(event) => {
                setDraft((current) => ({ ...current, timeout_open_after: event.target.value }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Server Error Open After
            <input
              value={draft.server_error_open_after}
              onChange={(event) => {
                setDraft((current) => ({
                  ...current,
                  server_error_open_after: event.target.value,
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Cooldown (ms)
            <input
              value={draft.cooldown_ms}
              onChange={(event) => {
                setDraft((current) => ({ ...current, cooldown_ms: event.target.value }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Half Open After (ms)
            <input
              value={draft.half_open_after_ms}
              onChange={(event) => {
                setDraft((current) => ({
                  ...current,
                  half_open_after_ms: event.target.value,
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Success Close After
            <input
              value={draft.success_close_after}
              onChange={(event) => {
                setDraft((current) => ({
                  ...current,
                  success_close_after: event.target.value,
                }));
              }}
            />
          </label>
        </p>
        <button type="submit" disabled={isSaving || policies.length === 0}>
          Save policy
        </button>
      </form>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p>{successMessage}</p> : null}
    </section>
  );
}
