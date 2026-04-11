import { useEffect, useState } from "react";
import { listAccounts } from "../../lib/tauri";
import type { AccountSummary } from "../../lib/types";

export function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountSummary[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    listAccounts()
      .then((nextAccounts) => {
        if (isMounted) {
          setAccounts(nextAccounts);
          setErrorMessage(null);
        }
      })
      .catch(() => {
        if (isMounted) {
          setErrorMessage("Failed to load accounts.");
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section aria-labelledby="accounts-heading">
      <h2 id="accounts-heading">Account Details</h2>
      <p>View the accounts available to the local gateway and verify provider ownership.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <ul>
        {accounts.map((account) => (
          <li key={account.name}>
            <strong>{account.name}</strong>
            <span>{account.provider}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
