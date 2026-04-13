import { useEffect, useState } from "react";
import {
  getAccountCapabilityDetail,
  importOfficialAccountLogin,
  listAccounts,
  refreshAccountBalance,
} from "../../lib/tauri";
import type {
  AccountBalanceSnapshot,
  AccountCapabilityDetail,
  AccountSummary,
  OfficialAccountImportInput,
} from "../../lib/types";
import { AccountImportForm } from "./account-import-form";

interface AccountPanelState {
  account: AccountSummary;
  balanceError: string | null;
  balanceSnapshot: AccountBalanceSnapshot | null;
  capabilityDetail: AccountCapabilityDetail | null;
  capabilityError: string | null;
}

export function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountPanelState[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [importErrorMessage, setImportErrorMessage] = useState<string | null>(null);
  const [importSuccessMessage, setImportSuccessMessage] = useState<string | null>(null);
  const [isImportingAccount, setIsImportingAccount] = useState(false);

  async function loadAccounts(isMounted: () => boolean = () => true) {
    try {
      const summaries = await listAccounts();
      const accountPanels = await Promise.all(
        summaries.map(async (account) => {
          const panelState: AccountPanelState = {
            account,
            balanceError: null,
            balanceSnapshot: null,
            capabilityDetail: null,
            capabilityError: null,
          };

          try {
            panelState.balanceSnapshot = await refreshAccountBalance(account.account_id);
          } catch (error) {
            panelState.balanceError =
              error instanceof Error ? error.message : "Failed to refresh account balance.";
          }

          try {
            panelState.capabilityDetail = await getAccountCapabilityDetail(account.account_id);
          } catch (error) {
            panelState.capabilityError =
              error instanceof Error ? error.message : "Failed to load account capability detail.";
          }

          return panelState;
        }),
      );

      if (isMounted()) {
        setAccounts(accountPanels);
        setErrorMessage(null);
      }
    } catch {
      if (isMounted()) {
        setErrorMessage("Failed to load accounts.");
      }
    }
  }

  useEffect(() => {
    let isMounted = true;
    void loadAccounts(() => isMounted);

    return () => {
      isMounted = false;
    };
  }, []);

  async function handleImportAccount(input: OfficialAccountImportInput) {
    if (isImportingAccount) {
      return;
    }

    setIsImportingAccount(true);
    setImportErrorMessage(null);
    setImportSuccessMessage(null);

    try {
      const imported = await importOfficialAccountLogin(input);
      await loadAccounts();
      setImportSuccessMessage(`Imported account: ${imported.account_id}`);
      setErrorMessage(null);
    } catch (error) {
      setImportErrorMessage(error instanceof Error ? error.message : "Failed to import account.");
    } finally {
      setIsImportingAccount(false);
    }
  }

  return (
    <section aria-labelledby="accounts-heading">
      <h2 id="accounts-heading">Account Details</h2>
      <p>View the accounts available to the local gateway and verify provider ownership.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <AccountImportForm
        errorMessage={importErrorMessage}
        isSubmitting={isImportingAccount}
        successMessage={importSuccessMessage}
        onSubmit={handleImportAccount}
      />
      <div className="detail-grid">
        {accounts.map((panel) => (
          <article className="detail-card" key={panel.account.account_id}>
            <h3>{panel.account.name}</h3>
            <p>Provider: {panel.account.provider}</p>
            {panel.balanceSnapshot ? (
              <>
                <p>Balance state: {panel.balanceSnapshot.balance.kind}</p>
                {panel.balanceSnapshot.balance.kind === "queryable" ? (
                  <>
                    <p>Total: {panel.balanceSnapshot.balance.total}</p>
                    <p>Used: {panel.balanceSnapshot.balance.used}</p>
                  </>
                ) : (
                  <p>{panel.balanceSnapshot.balance.reason}</p>
                )}
              </>
            ) : (
              <p>{panel.balanceError ?? "Balance unavailable."}</p>
            )}
            {panel.capabilityDetail ? (
              <>
                <p>Refresh support: {String(panel.capabilityDetail.refresh_capability)}</p>
                <p>Balance capability: {panel.capabilityDetail.balance_capability}</p>
              </>
            ) : (
              <p>{panel.capabilityError ?? "Capability detail unavailable."}</p>
            )}
          </article>
        ))}
      </div>
    </section>
  );
}
