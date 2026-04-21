import { useEffect, useState } from "react";
import { listAccounts, listPolicies, listRelays, updatePolicy } from "../../lib/tauri";
import type { PolicySummary, PolicyUpdateInput } from "../../lib/types";
import { PageHeader } from "../../components/page-header";
import { PolicyEditor } from "./policy-editor";

export function PoliciesPage() {
  const [endpointIds, setEndpointIds] = useState<string[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [editorErrorMessage, setEditorErrorMessage] = useState<string | null>(null);
  const [editorSuccessMessage, setEditorSuccessMessage] = useState<string | null>(null);
  const [isSavingPolicy, setIsSavingPolicy] = useState(false);
  const [policies, setPolicies] = useState<PolicySummary[]>([]);

  async function loadPolicies(isMounted: () => boolean = () => true) {
    try {
      const [nextPolicies, accounts, relays] = await Promise.all([
        listPolicies(),
        listAccounts(),
        listRelays(),
      ]);
      if (isMounted()) {
        setPolicies(nextPolicies);
        setEndpointIds([
          ...accounts.map((account) => account.account_id),
          ...relays.map((relay) => relay.relay_id),
        ]);
        setErrorMessage(null);
      }
    } catch {
      if (isMounted()) {
        setErrorMessage("Failed to load policies.");
      }
    }
  }

  useEffect(() => {
    let isMounted = true;

    void loadPolicies(() => isMounted);

    return () => {
      isMounted = false;
    };
  }, []);

  async function handleSavePolicy(input: PolicyUpdateInput) {
    if (isSavingPolicy) {
      return;
    }

    setIsSavingPolicy(true);
    setEditorErrorMessage(null);
    setEditorSuccessMessage(null);
    try {
      const saved = await updatePolicy(input);
      setPolicies((current) =>
        current.map((policy) =>
          policy.policy_id === saved.policy_id ? { ...policy, ...saved } : policy,
        ),
      );
      setEditorSuccessMessage(`Policy saved: ${saved.name}`);
      setErrorMessage(null);
    } catch (error) {
      setEditorErrorMessage(error instanceof Error ? error.message : "Failed to save policy.");
    } finally {
      setIsSavingPolicy(false);
    }
  }

  return (
    <section className="workspace-page" aria-labelledby="policies-heading">
      <PageHeader
        eyebrow="Routing behavior"
        titleId="policies-heading"
        title="Policies"
        description="Edit endpoint order, retry budget, and recovery thresholds with a preview that keeps routing consequences explicit."
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="workspace-summary-strip">
        <article className="operator-callout">
          <h4>Policies loaded</h4>
          <p>{policies.length} routing policies are currently available for local gateway selection.</p>
        </article>
        <article className="operator-callout">
          <h4>Known endpoints</h4>
          <p>{endpointIds.length} account and relay identifiers can participate in ordered candidate lists.</p>
        </article>
        <article className="operator-callout">
          <h4>Editor mode</h4>
          <p>Policy editing remains local-first and previews routing consequences before saving changes.</p>
        </article>
      </div>
      <div className="workspace-grid">
        <div className="workspace-column">
          <PolicyEditor
            endpointIds={endpointIds}
            errorMessage={editorErrorMessage}
            isSaving={isSavingPolicy}
            policies={policies}
            successMessage={editorSuccessMessage}
            onSave={handleSavePolicy}
          />
        </div>
        <div className="workspace-column">
          <section className="panel">
            <div className="panel-heading">
              <div>
                <h3>Policy inventory</h3>
                <p>Quick reference for policy ids and current status while editing thresholds or candidate order.</p>
              </div>
            </div>
            <ul className="operator-list" aria-label="Policy inventory reference">
              {policies.map((policy) => (
                <li className="operator-list__item" key={policy.policy_id}>
                  <div className="operator-list__item-header">
                    <strong className="operator-list__item-title">{policy.name}</strong>
                    <code>{policy.policy_id}</code>
                  </div>
                  <p className="operator-message">Status: {policy.status}</p>
                </li>
              ))}
            </ul>
          </section>
        </div>
      </div>
    </section>
  );
}
