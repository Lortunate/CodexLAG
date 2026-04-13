import { useEffect, useState } from "react";
import { listAccounts, listPolicies, listRelays, updatePolicy } from "../../lib/tauri";
import type { PolicySummary, PolicyUpdateInput } from "../../lib/types";
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
          policy.policy_id === saved.policy_id ? { ...policy, name: saved.name } : policy,
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
      <h2 id="policies-heading">Policy Status</h2>
      <p>Inspect the policy layer that decides how requests flow between accounts and relays.</p>
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
