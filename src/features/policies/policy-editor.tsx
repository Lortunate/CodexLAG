import { useEffect, useMemo, useState, type FormEvent } from "react";
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
  cross_pool_fallback: boolean | null;
  retry_budget: string;
  timeout_open_after: string;
  server_error_open_after: string;
  cooldown_ms: string;
  half_open_after_ms: string;
  success_close_after: string;
}

function policyToDraft(policy: PolicySummary): PolicyDraft {
  return {
    policy_id: policy.policy_id,
    name: policy.name,
    selection_order: policy.selection_order?.join(", ") ?? "",
    cross_pool_fallback: policy.cross_pool_fallback ?? null,
    retry_budget: policy.retry_budget?.toString() ?? "",
    timeout_open_after: policy.timeout_open_after?.toString() ?? "",
    server_error_open_after: policy.server_error_open_after?.toString() ?? "",
    cooldown_ms: policy.cooldown_ms?.toString() ?? "",
    half_open_after_ms: policy.half_open_after_ms?.toString() ?? "",
    success_close_after: policy.success_close_after?.toString() ?? "",
  };
}

function parsePositiveInteger(raw: string): number | null {
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function hasHydratedPolicySnapshot(policy: PolicySummary) {
  return (
    policy.selection_order !== undefined &&
    policy.cross_pool_fallback !== undefined &&
    policy.retry_budget !== undefined &&
    policy.timeout_open_after !== undefined &&
    policy.server_error_open_after !== undefined &&
    policy.cooldown_ms !== undefined &&
    policy.half_open_after_ms !== undefined &&
    policy.success_close_after !== undefined
  );
}

export function PolicyEditor({
  endpointIds,
  errorMessage,
  isSaving,
  onSave,
  policies,
  successMessage,
}: PolicyEditorProps) {
  const [activePolicyId, setActivePolicyId] = useState("");
  const [draftsByPolicyId, setDraftsByPolicyId] = useState<Record<string, PolicyDraft>>({});

  useEffect(() => {
    if (policies.length === 0) {
      setActivePolicyId("");
      setDraftsByPolicyId({});
      return;
    }

    setDraftsByPolicyId((current) => {
      const next = { ...current };
      for (const policy of policies) {
        if (!next[policy.policy_id]) {
          next[policy.policy_id] = policyToDraft(policy);
        }
      }
      return next;
    });

    setActivePolicyId((current) => {
      if (current && policies.some((policy) => policy.policy_id === current)) {
        return current;
      }
      return policies[0].policy_id;
    });
  }, [policies]);

  const activePolicy = useMemo(
    () => policies.find((policy) => policy.policy_id === activePolicyId) ?? null,
    [activePolicyId, policies],
  );
  const activeDraft = activePolicyId ? draftsByPolicyId[activePolicyId] : undefined;
  const selectionOrder = activeDraft
    ? activeDraft.selection_order
        .split(",")
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0)
    : [];
  const retryBudget = activeDraft ? parsePositiveInteger(activeDraft.retry_budget) : null;
  const timeoutOpenAfter = activeDraft ? parsePositiveInteger(activeDraft.timeout_open_after) : null;
  const serverErrorOpenAfter = activeDraft
    ? parsePositiveInteger(activeDraft.server_error_open_after)
    : null;
  const cooldownMs = activeDraft ? parsePositiveInteger(activeDraft.cooldown_ms) : null;
  const halfOpenAfterMs = activeDraft ? parsePositiveInteger(activeDraft.half_open_after_ms) : null;
  const successCloseAfter = activeDraft ? parsePositiveInteger(activeDraft.success_close_after) : null;
  const canSave =
    !!activeDraft &&
    activeDraft.name.trim().length > 0 &&
    selectionOrder.length > 0 &&
    activeDraft.cross_pool_fallback !== null &&
    retryBudget !== null &&
    timeoutOpenAfter !== null &&
    serverErrorOpenAfter !== null &&
    cooldownMs !== null &&
    halfOpenAfterMs !== null &&
    successCloseAfter !== null;
  const crossPoolFallbackValue = activeDraft
    ? activeDraft.cross_pool_fallback === null
      ? ""
      : activeDraft.cross_pool_fallback
        ? "true"
        : "false"
    : "";

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (
      !activeDraft ||
      activeDraft.cross_pool_fallback === null ||
      retryBudget === null ||
      timeoutOpenAfter === null ||
      serverErrorOpenAfter === null ||
      cooldownMs === null ||
      halfOpenAfterMs === null ||
      successCloseAfter === null
    ) {
      return;
    }

    await onSave({
      policy_id: activeDraft.policy_id,
      name: activeDraft.name.trim(),
      selection_order: selectionOrder,
      cross_pool_fallback: activeDraft.cross_pool_fallback,
      retry_budget: retryBudget,
      timeout_open_after: timeoutOpenAfter,
      server_error_open_after: serverErrorOpenAfter,
      cooldown_ms: cooldownMs,
      half_open_after_ms: halfOpenAfterMs,
      success_close_after: successCloseAfter,
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
              value={activePolicyId}
              onChange={(event) => {
                setActivePolicyId(event.target.value);
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
              value={activeDraft?.name ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: { ...activeDraft, name: event.target.value },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Selection Order
            <input
              value={activeDraft?.selection_order ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    selection_order: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Cross Pool Fallback
            <select
              value={crossPoolFallbackValue}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                const nextValue =
                  event.target.value === "" ? null : event.target.value === "true";
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    cross_pool_fallback: nextValue,
                  },
                }));
              }}
            >
              <option value="">Select behavior</option>
              <option value="false">false</option>
              <option value="true">true</option>
            </select>
          </label>
        </p>
        <p>
          <label>
            Retry Budget
            <input
              value={activeDraft?.retry_budget ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: { ...activeDraft, retry_budget: event.target.value },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Timeout Open After
            <input
              value={activeDraft?.timeout_open_after ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    timeout_open_after: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Server Error Open After
            <input
              value={activeDraft?.server_error_open_after ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    server_error_open_after: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Cooldown (ms)
            <input
              value={activeDraft?.cooldown_ms ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    cooldown_ms: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Half Open After (ms)
            <input
              value={activeDraft?.half_open_after_ms ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    half_open_after_ms: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <p>
          <label>
            Success Close After
            <input
              value={activeDraft?.success_close_after ?? ""}
              onChange={(event) => {
                if (!activeDraft) {
                  return;
                }
                setDraftsByPolicyId((current) => ({
                  ...current,
                  [activeDraft.policy_id]: {
                    ...activeDraft,
                    success_close_after: event.target.value,
                  },
                }));
              }}
            />
          </label>
        </p>
        <button type="submit" disabled={isSaving || policies.length === 0 || !canSave}>
          Save policy
        </button>
      </form>
      {!activePolicy || hasHydratedPolicySnapshot(activePolicy) ? null : (
        <p>
          Policy snapshot details are not available from runtime. Enter all policy fields before
          saving.
        </p>
      )}
      {endpointIds.length > 0 ? (
        <p>Known endpoint ids: {endpointIds.join(", ")}</p>
      ) : null}
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p>{successMessage}</p> : null}
    </section>
  );
}
