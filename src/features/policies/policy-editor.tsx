import { useEffect, useMemo, useState, type FormEvent } from "react";
import { z } from "zod";
import type { PolicyPreviewSummary, PolicySummary, PolicyUpdateInput } from "../../lib/types";

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

const PolicySchema = z.object({
  name: z.string().trim().min(1, { error: "Policy name is required." }),
  selection_order: z.array(z.string()).min(1, {
    error: "Selection order must include at least one endpoint.",
  }),
  cross_pool_fallback: z.boolean({
    error: "Select a cross-pool fallback behavior.",
  }),
  retry_budget: z.number({ error: "Retry budget must be a positive integer." }).int().positive({
    error: "Retry budget must be a positive integer.",
  }),
  timeout_open_after: z
    .number({ error: "Timeout open after must be a positive integer." })
    .int()
    .positive({
      error: "Timeout open after must be a positive integer.",
    }),
  server_error_open_after: z
    .number({ error: "Server error open after must be a positive integer." })
    .int()
    .positive({
      error: "Server error open after must be a positive integer.",
    }),
  cooldown_ms: z.number({ error: "Cooldown must be a positive integer." }).int().positive({
    error: "Cooldown must be a positive integer.",
  }),
  half_open_after_ms: z
    .number({ error: "Half open after must be a positive integer." })
    .int()
    .positive({
      error: "Half open after must be a positive integer.",
    }),
  success_close_after: z
    .number({ error: "Success close after must be a positive integer." })
    .int()
    .positive({
      error: "Success close after must be a positive integer.",
    }),
});

function policyToDraft(policy: PolicySummary): PolicyDraft {
  return {
    policy_id: policy.policy_id,
    name: policy.name,
    selection_order: policy.selection_order.join(", "),
    cross_pool_fallback: policy.cross_pool_fallback,
    retry_budget: policy.retry_budget.toString(),
    timeout_open_after: policy.timeout_open_after.toString(),
    server_error_open_after: policy.server_error_open_after.toString(),
    cooldown_ms: policy.cooldown_ms.toString(),
    half_open_after_ms: policy.half_open_after_ms.toString(),
    success_close_after: policy.success_close_after.toString(),
  };
}

function parsePositiveInteger(raw: string): number | null {
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function buildPolicyPreviewSummary(
  selectionOrder: string[],
  endpointIds: string[],
): PolicyPreviewSummary {
  const endpointSet = new Set(endpointIds);
  return {
    eligible_candidates: selectionOrder.filter((endpointId) => endpointSet.has(endpointId)),
    rejected_candidates: endpointIds.filter((endpointId) => !selectionOrder.includes(endpointId)),
  };
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
  const [showValidationErrors, setShowValidationErrors] = useState(false);

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
    setShowValidationErrors(false);
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
  const validationResult = useMemo(() => {
    if (!activeDraft) {
      return null;
    }

    return PolicySchema.safeParse({
      name: activeDraft.name.trim(),
      selection_order: selectionOrder,
      cross_pool_fallback: activeDraft.cross_pool_fallback ?? undefined,
      retry_budget: retryBudget ?? undefined,
      timeout_open_after: timeoutOpenAfter ?? undefined,
      server_error_open_after: serverErrorOpenAfter ?? undefined,
      cooldown_ms: cooldownMs ?? undefined,
      half_open_after_ms: halfOpenAfterMs ?? undefined,
      success_close_after: successCloseAfter ?? undefined,
    });
  }, [
    activeDraft,
    cooldownMs,
    halfOpenAfterMs,
    retryBudget,
    selectionOrder,
    serverErrorOpenAfter,
    successCloseAfter,
    timeoutOpenAfter,
  ]);
  const fieldErrors =
    showValidationErrors && validationResult && !validationResult.success
      ? validationResult.error.flatten().fieldErrors
      : {};
  const canSave = !!validationResult?.success;
  const crossPoolFallbackValue = activeDraft
    ? activeDraft.cross_pool_fallback === null
      ? ""
      : activeDraft.cross_pool_fallback
        ? "true"
        : "false"
    : "";
  const availableEndpointIds = endpointIds.filter((endpointId) => !selectionOrder.includes(endpointId));
  const previewSummary = buildPolicyPreviewSummary(selectionOrder, endpointIds);
  const orderedAttemptPath =
    previewSummary.eligible_candidates.length > 0
      ? previewSummary.eligible_candidates.join(" -> ")
      : "none";
  const rejectionReason =
    previewSummary.rejected_candidates.length > 0
      ? "These endpoints are not explicitly ordered; the runtime may still append them after ordered candidates if they remain available."
      : "All known endpoints are explicitly represented in the current ordered selection list.";
  const fallbackBehaviorSummary =
    activeDraft?.cross_pool_fallback === true
      ? "If every ordered candidate fails, the runtime may continue into cross-pool fallback instead of stopping at the current lane."
      : activeDraft?.cross_pool_fallback === false
        ? "If every ordered candidate fails, the runtime stops at the configured lane and will not spill into cross-pool fallback."
        : "Cross-pool fallback is not selected yet, so the preview cannot determine whether routing may spill into another lane.";
  const firstAttemptSummary =
    previewSummary.eligible_candidates.length > 0
      ? `Configured preference starts with ${previewSummary.eligible_candidates[0]}. Actual runtime choice still depends on availability, health, and recovery state.`
      : "No configured preference can be evaluated until at least one known candidate is ordered.";

  function updateSelectionOrder(nextSelectionOrder: string[]) {
    if (!activeDraft) {
      return;
    }
    setDraftsByPolicyId((current) => ({
      ...current,
      [activeDraft.policy_id]: {
        ...activeDraft,
        selection_order: nextSelectionOrder.join(", "),
      },
    }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setShowValidationErrors(true);
    if (!activeDraft || !validationResult?.success) {
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
    setShowValidationErrors(false);
  }

  return (
    <section className="panel" aria-labelledby="policy-editor-heading">
      <div className="panel-heading">
        <div>
          <h3 id="policy-editor-heading">Policy Editor</h3>
          <p>Adjust candidate order and recovery thresholds without hiding the routing consequences.</p>
        </div>
      </div>
      <ul aria-label="Policy summaries" className="operator-list">
        {policies.map((policy) => (
          <li key={policy.policy_id} className="operator-list__item">
            <div className="operator-list__item-header">
              <strong className="operator-list__item-title">{policy.name}</strong>
              <code>{policy.policy_id}</code>
            </div>
            <p className="operator-message">Status: {policy.status}</p>
          </li>
        ))}
      </ul>
      <form className="operator-form" onSubmit={handleSubmit}>
        <div className="operator-fields">
          <div className="operator-field">
            <label htmlFor="policy-select">Policy</label>
            <select
              id="policy-select"
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
          </div>
          <div className="operator-field operator-field--full">
            <label htmlFor="policy-name">Policy Name</label>
            <input
              id="policy-name"
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
            {fieldErrors.name?.[0] ? <span role="alert">{fieldErrors.name[0]}</span> : null}
          </div>
          <div className="operator-field operator-field--full">
            <label htmlFor="policy-selection-order">Selection Order</label>
            <input id="policy-selection-order" readOnly value={selectionOrder.join(", ")} />
            {fieldErrors.selection_order?.[0] ? (
              <span role="alert">{fieldErrors.selection_order[0]}</span>
            ) : null}
          </div>
        </div>
        <div aria-label="Selection order controls" className="operator-preview">
          <p className="operator-field-help">
            Reorder candidates explicitly instead of editing a comma-separated string.
          </p>
          <ul className="operator-selection-list">
            {selectionOrder.map((endpointId, index) => (
              <li key={endpointId}>
                <code>{endpointId}</code>
                <div className="operator-actions">
                  <button
                    type="button"
                    onClick={() => {
                      if (index === 0) {
                        return;
                      }
                      const next = [...selectionOrder];
                      [next[index - 1], next[index]] = [next[index], next[index - 1]];
                      updateSelectionOrder(next);
                    }}
                  >
                    Move {endpointId} up
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      if (index === selectionOrder.length - 1) {
                        return;
                      }
                      const next = [...selectionOrder];
                      [next[index], next[index + 1]] = [next[index + 1], next[index]];
                      updateSelectionOrder(next);
                    }}
                  >
                    Move {endpointId} down
                  </button>
                  <button
                    type="button"
                    onClick={() =>
                      updateSelectionOrder(selectionOrder.filter((candidate) => candidate !== endpointId))
                    }
                  >
                    Remove {endpointId}
                  </button>
                </div>
              </li>
            ))}
          </ul>
          {availableEndpointIds.length > 0 ? (
            <div className="operator-stack">
              <p className="operator-message">Available endpoints</p>
              <div className="operator-pill-list">
                {availableEndpointIds.map((endpointId) => (
                  <button
                    key={endpointId}
                    type="button"
                    onClick={() => updateSelectionOrder([...selectionOrder, endpointId])}
                  >
                    Add {endpointId}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        </div>
        <div className="operator-fields">
          <div className="operator-field">
            <label htmlFor="policy-fallback">Cross Pool Fallback</label>
            <select
              id="policy-fallback"
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
            {fieldErrors.cross_pool_fallback?.[0] ? (
              <span role="alert">{fieldErrors.cross_pool_fallback[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-retry-budget">Retry Budget</label>
            <input
              id="policy-retry-budget"
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
            {fieldErrors.retry_budget?.[0] ? (
              <span role="alert">{fieldErrors.retry_budget[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-timeout-open-after">Timeout Open After</label>
            <input
              id="policy-timeout-open-after"
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
            {fieldErrors.timeout_open_after?.[0] ? (
              <span role="alert">{fieldErrors.timeout_open_after[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-server-error-open-after">Server Error Open After</label>
            <input
              id="policy-server-error-open-after"
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
            {fieldErrors.server_error_open_after?.[0] ? (
              <span role="alert">{fieldErrors.server_error_open_after[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-cooldown-ms">Cooldown (ms)</label>
            <input
              id="policy-cooldown-ms"
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
            {fieldErrors.cooldown_ms?.[0] ? (
              <span role="alert">{fieldErrors.cooldown_ms[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-half-open-after-ms">Half Open After (ms)</label>
            <input
              id="policy-half-open-after-ms"
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
            {fieldErrors.half_open_after_ms?.[0] ? (
              <span role="alert">{fieldErrors.half_open_after_ms[0]}</span>
            ) : null}
          </div>
          <div className="operator-field">
            <label htmlFor="policy-success-close-after">Success Close After</label>
            <input
              id="policy-success-close-after"
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
            {fieldErrors.success_close_after?.[0] ? (
              <span role="alert">{fieldErrors.success_close_after[0]}</span>
            ) : null}
          </div>
        </div>
        <div className="operator-form-actions">
          <button type="submit" disabled={isSaving || policies.length === 0}>
            Save policy
          </button>
        </div>
      </form>
      <section className="operator-preview" aria-labelledby="policy-preview-heading">
        <h4 id="policy-preview-heading">Candidate preview</h4>
        <p>Eligible candidates: {previewSummary.eligible_candidates.join(", ") || "none"}</p>
        <p>Rejected candidates: {previewSummary.rejected_candidates.join(", ") || "none"}</p>
        <p>Ordered attempt path: {orderedAttemptPath}</p>
        <p>{firstAttemptSummary}</p>
        <p>{fallbackBehaviorSummary}</p>
        <p>{rejectionReason}</p>
      </section>
      {endpointIds.length > 0 ? (
        <p className="operator-message">Known endpoint ids: {endpointIds.join(", ")}</p>
      ) : null}
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      {successMessage ? <p className="operator-success">{successMessage}</p> : null}
    </section>
  );
}
