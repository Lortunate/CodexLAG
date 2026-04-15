import { useEffect, useState } from "react";
import {
  getAccountCapabilityDetail,
  listAccounts,
  listProviderSessions,
  logoutOpenAiSession,
  refreshAccountBalance,
  refreshOpenAiSession,
  startOpenAiBrowserLogin,
} from "../../lib/tauri";
import type {
  AccountBalanceSnapshot,
  AccountCapabilityDetail,
  AccountSummary,
  ProviderAccountHealth,
  ProviderSessionSummary,
} from "../../lib/types";

interface AccountPanelState {
  account: AccountSummary;
  balanceError: string | null;
  balanceSnapshot: AccountBalanceSnapshot | null;
  capabilityDetail: AccountCapabilityDetail | null;
  capabilityError: string | null;
  providerHealth: ProviderAccountHealth;
}

function authProfileLabel(authProfile: string) {
  return authProfile === "browser" ? "Browser sign-in" : "API key";
}

function resolveProviderAuthProfile(provider: string) {
  return provider === "openai" ? "browser" : "api_key";
}

export function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountPanelState[]>([]);
  const [providerSessions, setProviderSessions] = useState<ProviderSessionSummary[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [authActionMessage, setAuthActionMessage] = useState<string | null>(null);
  const [sessionActionError, setSessionActionError] = useState<string | null>(null);
  const [sessionActionPending, setSessionActionPending] = useState<string | null>(null);

  async function loadAccounts(isMounted: () => boolean = () => true) {
    try {
      const [summaries, sessions] = await Promise.all([listAccounts(), listProviderSessions()]);
      const accountPanels = await Promise.all(
        summaries.map(async (account) => {
          const panelState: AccountPanelState = {
            account,
            balanceError: null,
            balanceSnapshot: null,
            capabilityDetail: null,
            capabilityError: null,
            providerHealth: {
              provider_id: account.provider,
              account_id: account.account_id,
              auth_state: "ready",
              auth_profile: resolveProviderAuthProfile(account.provider),
              last_error_message: null,
            },
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
        setProviderSessions(sessions);
        setErrorMessage(null);
      }
    } catch {
      if (isMounted()) {
        setErrorMessage("Failed to load accounts.");
      }
    }
  }

  async function handleStartOpenAiLogin() {
    if (sessionActionPending) {
      return;
    }

    setAuthActionMessage(null);
    setSessionActionError(null);
    setSessionActionPending("browser-login");

    try {
      const pending = await startOpenAiBrowserLogin();
      await loadAccounts();
      setAuthActionMessage(
        `Browser login started for ${pending.summary.display_name}. Callback: ${pending.callback_url}`,
      );
    } catch (error) {
      setSessionActionError(
        error instanceof Error ? error.message : "Failed to start OpenAI browser login.",
      );
    } finally {
      setSessionActionPending(null);
    }
  }

  useEffect(() => {
    let isMounted = true;
    void loadAccounts(() => isMounted);

    return () => {
      isMounted = false;
    };
  }, []);

  async function handleRefreshProviderSession(accountId: string) {
    if (sessionActionPending) {
      return;
    }

    setAuthActionMessage(null);
    setSessionActionError(null);
    setSessionActionPending(`refresh:${accountId}`);

    try {
      await refreshOpenAiSession(accountId);
      await loadAccounts();
      setAuthActionMessage(`Refreshed OpenAI session: ${accountId}`);
    } catch (error) {
      setSessionActionError(error instanceof Error ? error.message : "Failed to refresh session.");
    } finally {
      setSessionActionPending(null);
    }
  }

  async function handleLogoutProviderSession(accountId: string) {
    if (sessionActionPending) {
      return;
    }

    setAuthActionMessage(null);
    setSessionActionError(null);
    setSessionActionPending(`logout:${accountId}`);

    try {
      await logoutOpenAiSession(accountId);
      await loadAccounts();
      setAuthActionMessage(`Signed out OpenAI session: ${accountId}`);
    } catch (error) {
      setSessionActionError(error instanceof Error ? error.message : "Failed to sign out session.");
    } finally {
      setSessionActionPending(null);
    }
  }

  return (
    <section aria-labelledby="accounts-heading">
      <h2 id="accounts-heading">Official Accounts</h2>
      <p>Review provider identity, launch browser login, and inspect capability status.</p>
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="detail-card">
        <h3>Browser sign-in</h3>
        <p>Launch the official OpenAI login flow and manage persisted desktop sessions.</p>
        <button onClick={() => void handleStartOpenAiLogin()} type="button">
          Sign in with OpenAI
        </button>
        {authActionMessage ? <p>{authActionMessage}</p> : null}
        {sessionActionError ? <p role="alert">{sessionActionError}</p> : null}
        {providerSessions.length === 0 ? (
          <p>No OpenAI provider sessions stored.</p>
        ) : (
          <div className="detail-grid">
            {providerSessions.map((session) => (
              <article className="detail-card" key={`${session.provider_id}:${session.account_id}`}>
                <h4>{session.display_name}</h4>
                <p>Provider session: {session.account_id}</p>
                <p>Auth state: {session.auth_state}</p>
                <p>Auth profile: {authProfileLabel(session.auth_profile ?? "browser")}</p>
                <p>Last auth error: {session.last_error_message ?? session.last_refresh_error ?? "none"}</p>
                <button
                  onClick={() => void handleRefreshProviderSession(session.account_id)}
                  type="button"
                >
                  Refresh
                </button>
                <button
                  onClick={() => void handleLogoutProviderSession(session.account_id)}
                  type="button"
                >
                  Sign out
                </button>
              </article>
            ))}
          </div>
        )}
      </div>
      <div className="detail-grid">
        {accounts.map((panel) => (
          <article className="detail-card" key={panel.account.account_id}>
            <h3>{panel.account.name}</h3>
            <p>Provider: {panel.account.provider}</p>
            <p>Auth profile: {authProfileLabel(panel.providerHealth.auth_profile)}</p>
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
