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
    <section aria-labelledby="policies-heading">
      <PageHeader
        eyebrow="Routing behavior"
        titleId="policies-heading"
        title="Policies"
        description="Edit endpoint order, retry budget, and recovery thresholds with a preview that keeps routing consequences explicit."
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <PolicyEditor
        endpointIds={endpointIds}
        errorMessage={editorErrorMessage}
        isSaving={isSavingPolicy}
        policies={policies}
        successMessage={editorSuccessMessage}
        onSave={handleSavePolicy}
      />
    </section>
  );
}
