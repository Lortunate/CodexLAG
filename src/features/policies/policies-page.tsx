import { useEffect, useState } from "react";
import { listPolicies } from "../../lib/tauri";
import type { PolicySummary } from "../../lib/types";

export function PoliciesPage() {
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [policies, setPolicies] = useState<PolicySummary[]>([]);

  useEffect(() => {
    let isMounted = true;

    listPolicies()
      .then((nextPolicies) => {
        if (isMounted) {
          setPolicies(nextPolicies);
          setErrorMessage(null);
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to load policies.");
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="policies-heading">
      <h2 id="policies-heading">Policy Status</h2>
      <p>Inspect the policy layer that decides how requests flow between accounts and relays.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <ul>
        {policies.map((policy) => (
          <li key={policy.name}>
            <strong>{policy.name}</strong>
            <span>{policy.status}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
