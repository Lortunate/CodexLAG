import { useEffect, useState } from "react";
import {
  getAccountCapabilityDetail,
  listAccounts,
  listProviderDescriptors,
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
  ProviderDescriptor,
  ProviderAccountHealth,
  ProviderSessionSummary,
} from "../../lib/types";
import { PageHeader } from "../../components/page-header";

interface AccountPanelState {
  account: AccountSummary;
  balanceError: string | null;
  balanceSnapshot: AccountBalanceSnapshot | null;
  capabilityDetail: AccountCapabilityDetail | null;
  capabilityError: string | null;
  providerHealth: ProviderAccountHealth;
}

interface OnboardingCardState {
  providerId: string;
  authProfile: string;
}

function isOpenAiProvider(providerId: string) {
  return providerId === "openai" || providerId === "openai_official";
}

function authProfileLabel(authProfile: string | null | undefined) {
  switch (authProfile) {
    case "browser":
    case "browser_oauth_pkce":
      return "Browser sign-in";
    case "api_key":
    case "static_api_key":
      return "API key required";
    default:
      return "Authentication required";
  }
}

function resolveProviderAuthProfile(provider: string) {
  return provider === "openai" ? "browser_oauth_pkce" : "static_api_key";
}

function authProfileGuidance(authProfile: string | null | undefined) {
  return authProfile === "browser" || authProfile === "browser_oauth_pkce"
    ? "Launch the official browser login flow and manage persisted desktop sessions."
    : "Authenticate this provider with a configured API key before using the account.";
}

function buildOnboardingCards(
  descriptors: ProviderDescriptor[],
  accounts: AccountPanelState[],
  providerSessions: ProviderSessionSummary[],
) {
  const cards = new Map<string, OnboardingCardState>();
  cards.set("openai", {
    providerId: "openai",
    authProfile: "browser_oauth_pkce",
  });

  for (const descriptor of descriptors) {
    cards.set(descriptor.provider_id, {
      providerId: descriptor.provider_id,
      authProfile: descriptor.auth_profile,
    });
  }

  for (const panel of accounts) {
    if (!cards.has(panel.account.provider)) {
      cards.set(panel.account.provider, {
        providerId: panel.account.provider,
        authProfile: panel.providerHealth.auth_profile,
      });
    }
  }

  for (const session of providerSessions) {
    const providerId = isOpenAiProvider(session.provider_id) ? "openai" : session.provider_id;
    if (!cards.has(providerId)) {
      cards.set(providerId, {
        providerId,
        authProfile: session.auth_profile ?? resolveProviderAuthProfile(providerId),
      });
    }
  }

  return Array.from(cards.values());
}

function entitlementSummary(detail: AccountCapabilityDetail | null) {
  if (!detail?.entitlement?.plan_type) {
    return "No plan metadata projected.";
  }

  const source = detail.entitlement.claim_source
    ? ` via ${detail.entitlement.claim_source}`
    : "";
  return `Plan ${detail.entitlement.plan_type}${source}`;
}

export function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountPanelState[]>([]);
  const [providerDescriptors, setProviderDescriptors] = useState<ProviderDescriptor[]>([]);
  const [providerSessions, setProviderSessions] = useState<ProviderSessionSummary[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [authActionMessage, setAuthActionMessage] = useState<string | null>(null);
  const [sessionActionError, setSessionActionError] = useState<string | null>(null);
  const [sessionActionPending, setSessionActionPending] = useState<string | null>(null);

  async function loadAccounts(isMounted: () => boolean = () => true) {
    try {
      const [summaries, sessions] = await Promise.all([listAccounts(), listProviderSessions()]);
      let descriptors: ProviderDescriptor[] = [];

      try {
        descriptors = await listProviderDescriptors();
      } catch {
        descriptors = [];
      }

      const sessionHealthByAccountId = new Map(
        sessions.map((session) => [
          session.account_id,
          {
            provider_id: session.provider_id,
            account_id: session.account_id,
            auth_state: session.auth_state,
            auth_profile: session.auth_profile ?? null,
            last_error_message: session.last_error_message ?? session.last_refresh_error ?? null,
          },
        ]),
      );
      const descriptorByProviderId = new Map(
        descriptors.map((descriptor) => [descriptor.provider_id, descriptor]),
      );
      const accountPanels = await Promise.all(
        summaries.map(async (account) => {
          const matchedHealth = sessionHealthByAccountId.get(account.account_id);
          const descriptor = descriptorByProviderId.get(account.provider);
          const panelState: AccountPanelState = {
            account,
            balanceError: null,
            balanceSnapshot: null,
            capabilityDetail: null,
            capabilityError: null,
            providerHealth: {
              provider_id: matchedHealth?.provider_id ?? account.provider,
              account_id: account.account_id,
              auth_state: matchedHealth?.auth_state ?? "ready",
              auth_profile:
                matchedHealth?.auth_profile ??
                descriptor?.auth_profile ??
                resolveProviderAuthProfile(account.provider),
              last_error_message: matchedHealth?.last_error_message ?? null,
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
        setProviderDescriptors(descriptors);
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
      <PageHeader
        eyebrow="Provider onboarding"
        titleId="accounts-heading"
        title="Official Accounts"
        description="Review provider identity, launch the correct auth flow, and keep degraded sessions and capability state visible without hiding partial failures."
      />
      {errorMessage ? <p role="alert">{errorMessage}</p> : null}
      <div className="operator-summary-grid">
        <article className="operator-callout">
          <h4>Providers in scope</h4>
          <p>{buildOnboardingCards(providerDescriptors, accounts, providerSessions).length} auth paths tracked locally.</p>
        </article>
        <article className="operator-callout">
          <h4>Stored sessions</h4>
          <p>{providerSessions.length} provider sessions currently persisted on this desktop.</p>
        </article>
        <article className="operator-callout">
          <h4>Imported accounts</h4>
          <p>{accounts.length} official accounts available to the gateway and operator surfaces.</p>
        </article>
      </div>
      <div className="operator-section-title">
        <h3>Onboarding paths</h3>
        <p>Each provider advertises its required auth profile so operators can use the right entry point immediately.</p>
      </div>
      <div className="detail-grid">
        {buildOnboardingCards(providerDescriptors, accounts, providerSessions).map((card) => {
          const supportsBrowserLogin =
            card.authProfile === "browser" || card.authProfile === "browser_oauth_pkce";

          return (
            <article className="detail-card" key={card.providerId}>
              <div className="operator-list__item-header">
                <h3>{authProfileLabel(card.authProfile)}</h3>
                <code>{card.providerId}</code>
              </div>
              <p>{authProfileGuidance(card.authProfile)}</p>
              {isOpenAiProvider(card.providerId) && supportsBrowserLogin ? (
                <button onClick={() => void handleStartOpenAiLogin()} type="button">
                  Sign in with OpenAI
                </button>
              ) : null}
            </article>
          );
        })}
      </div>
      {authActionMessage ? <p role="status">{authActionMessage}</p> : null}
      {sessionActionError ? <p role="alert">{sessionActionError}</p> : null}
      <div className="operator-section-title">
        <h3>Provider sessions</h3>
        <p>Persisted browser sessions remain actionable here so refresh and sign-out stay close to auth health.</p>
      </div>
      {providerSessions.length === 0 ? (
        <p className="operator-empty">No provider sessions stored.</p>
      ) : (
        <div className="detail-grid">
          {providerSessions.map((session) => (
            <article className="detail-card" key={`${session.provider_id}:${session.account_id}`}>
              <div className="operator-list__item-header">
                <h4>{session.display_name}</h4>
                <code>{session.account_id}</code>
              </div>
              <dl className="operator-inline-pairs">
                <div>
                  <dt>Auth state</dt>
                  <dd>{session.auth_state}</dd>
                </div>
                <div>
                  <dt>Auth profile</dt>
                  <dd>{authProfileLabel(session.auth_profile)}</dd>
                </div>
              </dl>
              <p>Provider session: {session.account_id}</p>
              <p>Auth state: {session.auth_state}</p>
              <p>Auth profile: {authProfileLabel(session.auth_profile)}</p>
              <p>Last auth error: {session.last_error_message ?? session.last_refresh_error ?? "none"}</p>
              <div className="operator-actions">
                <button onClick={() => void handleRefreshProviderSession(session.account_id)} type="button">
                  Refresh
                </button>
                <button onClick={() => void handleLogoutProviderSession(session.account_id)} type="button">
                  Sign out
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
      <div className="operator-section-title">
        <h3>Account inventory</h3>
        <p>Capability, balance, auth degradation, and entitlement hints stay grouped at the account row so failures remain attributable.</p>
      </div>
      <div className="detail-grid">
        {accounts.map((panel) => (
          <article className="detail-card" key={panel.account.account_id}>
            <div className="operator-list__item-header">
              <h3>{panel.account.name}</h3>
              <code>{panel.account.account_id}</code>
            </div>
            <dl className="operator-inline-pairs">
              <div>
                <dt>Provider</dt>
                <dd>{panel.account.provider}</dd>
              </div>
              <div>
                <dt>Auth state</dt>
                <dd>{panel.providerHealth.auth_state}</dd>
              </div>
              <div>
                <dt>Auth profile</dt>
                <dd>{authProfileLabel(panel.providerHealth.auth_profile)}</dd>
              </div>
            </dl>
            <p>Provider: {panel.account.provider}</p>
            <p>Auth state: {panel.providerHealth.auth_state}</p>
            <p>Auth profile: {authProfileLabel(panel.providerHealth.auth_profile)}</p>
            <p>
              Last auth error:{" "}
              {panel.providerHealth.last_error_message ?? "No active auth errors."}
            </p>
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
                <p>{entitlementSummary(panel.capabilityDetail)}</p>
                <p>Capability status: {panel.capabilityDetail.status}</p>
                <p>Account identity: {panel.capabilityDetail.account_identity ?? "unknown"}</p>
                <p>Refresh support: {String(panel.capabilityDetail.refresh_capability)}</p>
                <p>Balance capability: {panel.capabilityDetail.balance_capability}</p>
                {panel.capabilityDetail.entitlement?.plan_type ? (
                  <p>Plan: {panel.capabilityDetail.entitlement.plan_type}</p>
                ) : null}
                {panel.capabilityDetail.entitlement?.claim_source ? (
                  <p>Source: {panel.capabilityDetail.entitlement.claim_source}</p>
                ) : null}
                {panel.capabilityDetail.entitlement?.subscription_active_until ? (
                  <p>
                    Active until: {panel.capabilityDetail.entitlement.subscription_active_until}
                  </p>
                ) : null}
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
