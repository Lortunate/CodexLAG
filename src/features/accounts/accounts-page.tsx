import { useEffect, useState } from "react";

import { StatusBadge } from "../../components/status-badge";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../../components/ui/card";
import {
  getAccountCapabilityDetail,
  importOfficialAccountLogin,
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
  OfficialAccountImportInput,
  ProviderDescriptor,
  ProviderAccountHealth,
  ProviderSessionSummary,
} from "../../lib/types";
import { AccountImportForm } from "./account-import-form";

interface AccountsPageProps {
  onOpenRelays?: () => void;
}

interface AccountPanelState {
  account: AccountSummary;
  balanceError: string | null;
  balanceSnapshot: AccountBalanceSnapshot | null;
  capabilityDetail: AccountCapabilityDetail | null;
  capabilityError: string | null;
  providerHealth: ProviderAccountHealth;
}

interface OnboardingCardState {
  authProfile: string;
  connectedAccounts: number;
  providerId: string;
}

type FilterAuthProfile = "all" | "browser_oauth_pkce" | "static_api_key";

function isOpenAiProvider(providerId: string) {
  return providerId === "openai" || providerId === "openai_official";
}

function isBrowserAuthProfile(authProfile: string | null | undefined) {
  return authProfile === "browser" || authProfile === "browser_oauth_pkce";
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
  return isBrowserAuthProfile(authProfile)
    ? "Launch the official browser flow, persist the desktop session, then refresh health to expose capability tags."
    : "Bind a credential reference first. Static API key providers remain available here, while routing priority stays in policies.";
}

function formatProviderName(providerId: string) {
  if (providerId === "openai" || providerId === "openai_official") {
    return "OpenAI";
  }
  if (providerId === "claude" || providerId === "claude_official") {
    return "Claude";
  }

  return providerId
    .split(/[_-]+/)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

function formatAuthState(authState: string) {
  return authState
    .split(/[_-]+/)
    .filter(Boolean)
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(" ");
}

function formatEpochMs(value: number | null) {
  if (!value) {
    return "Not yet refreshed";
  }

  return new Date(value).toLocaleString();
}

function statusVariant(
  authState: string,
  lastErrorMessage: string | null,
): "danger" | "neutral" | "success" | "warning" {
  if (lastErrorMessage) {
    return "danger";
  }
  if (authState === "active" || authState === "ready") {
    return "success";
  }
  if (authState === "pending" || authState === "refreshing") {
    return "warning";
  }
  return "neutral";
}

function getBalanceSummary(panel: AccountPanelState) {
  if (!panel.balanceSnapshot) {
    return panel.balanceError ?? "Balance unavailable";
  }

  if (panel.balanceSnapshot.balance.kind === "queryable") {
    return `${panel.balanceSnapshot.balance.used} used / ${panel.balanceSnapshot.balance.total} total`;
  }

  return panel.balanceSnapshot.balance.reason;
}

function buildOnboardingCards(
  descriptors: ProviderDescriptor[],
  accounts: AccountPanelState[],
  providerSessions: ProviderSessionSummary[],
) {
  const cards = new Map<string, OnboardingCardState>();
  cards.set("openai", {
    authProfile: "browser_oauth_pkce",
    connectedAccounts: 0,
    providerId: "openai",
  });

  for (const descriptor of descriptors) {
    cards.set(descriptor.provider_id, {
      authProfile: descriptor.auth_profile,
      connectedAccounts: 0,
      providerId: descriptor.provider_id,
    });
  }

  for (const panel of accounts) {
    const existing = cards.get(panel.account.provider);
    cards.set(panel.account.provider, {
      authProfile:
        existing?.authProfile ??
        panel.providerHealth.auth_profile ??
        resolveProviderAuthProfile(panel.account.provider),
      connectedAccounts: (existing?.connectedAccounts ?? 0) + 1,
      providerId: panel.account.provider,
    });
  }

  for (const session of providerSessions) {
    const providerId = isOpenAiProvider(session.provider_id) ? "openai" : session.provider_id;
    if (!cards.has(providerId)) {
      cards.set(providerId, {
        authProfile: session.auth_profile ?? resolveProviderAuthProfile(providerId),
        connectedAccounts: 0,
        providerId,
      });
    }
  }

  return Array.from(cards.values()).sort((left, right) => left.providerId.localeCompare(right.providerId));
}

export function AccountsPage({ onOpenRelays }: AccountsPageProps = {}) {
  const [accounts, setAccounts] = useState<AccountPanelState[]>([]);
  const [providerDescriptors, setProviderDescriptors] = useState<ProviderDescriptor[]>([]);
  const [providerSessions, setProviderSessions] = useState<ProviderSessionSummary[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [authActionMessage, setAuthActionMessage] = useState<string | null>(null);
  const [sessionActionError, setSessionActionError] = useState<string | null>(null);
  const [sessionActionPending, setSessionActionPending] = useState<string | null>(null);
  const [searchValue, setSearchValue] = useState("");
  const [authFilter, setAuthFilter] = useState<FilterAuthProfile>("all");
  const [isImportDialogOpen, setIsImportDialogOpen] = useState(false);
  const [importErrorMessage, setImportErrorMessage] = useState<string | null>(null);
  const [isImportPending, setIsImportPending] = useState(false);

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
            account_id: session.account_id,
            auth_profile: session.auth_profile ?? null,
            auth_state: session.auth_state,
            last_error_message: session.last_error_message ?? session.last_refresh_error ?? null,
            provider_id: session.provider_id,
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
              account_id: account.account_id,
              auth_profile:
                matchedHealth?.auth_profile ??
                descriptor?.auth_profile ??
                resolveProviderAuthProfile(account.provider),
              auth_state: matchedHealth?.auth_state ?? "ready",
              last_error_message: matchedHealth?.last_error_message ?? null,
              provider_id: matchedHealth?.provider_id ?? account.provider,
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

  useEffect(() => {
    let isMounted = true;
    void loadAccounts(() => isMounted);

    return () => {
      isMounted = false;
    };
  }, []);

  async function handleRefreshAll() {
    if (sessionActionPending) {
      return;
    }

    setAuthActionMessage(null);
    setSessionActionError(null);
    setSessionActionPending("refresh:all");

    try {
      await loadAccounts();
      setAuthActionMessage("Refreshed upstream account health and session diagnostics.");
    } catch (error) {
      setSessionActionError(error instanceof Error ? error.message : "Failed to refresh account health.");
    } finally {
      setSessionActionPending(null);
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

  async function handleImportOfficialAccount(input: OfficialAccountImportInput) {
    setImportErrorMessage(null);
    setAuthActionMessage(null);
    setSessionActionError(null);
    setIsImportPending(true);

    try {
      const imported = await importOfficialAccountLogin(input);
      await loadAccounts();
      setAuthActionMessage(`Imported official account: ${imported.name}`);
      setIsImportDialogOpen(false);
      return true;
    } catch (error) {
      setImportErrorMessage(
        error instanceof Error ? error.message : "Failed to import official account.",
      );
      return false;
    } finally {
      setIsImportPending(false);
    }
  }

  const fallbackDescriptors = [
    { provider_id: "openai", auth_profile: "browser_oauth_pkce" as const },
  ];
  const providerOptions = (providerDescriptors.length > 0 ? providerDescriptors : fallbackDescriptors).map(
    (descriptor) => ({
      label: formatProviderName(descriptor.provider_id),
      value: descriptor.provider_id,
    }),
  );
  const sessionByAccountId = new Map(providerSessions.map((session) => [session.account_id, session]));
  const normalizedSearch = searchValue.trim().toLowerCase();
  const filteredAccounts = accounts.filter((panel) => {
    const matchesSearch =
      normalizedSearch.length === 0 ||
      panel.account.name.toLowerCase().includes(normalizedSearch) ||
      panel.account.account_id.toLowerCase().includes(normalizedSearch) ||
      panel.account.provider.toLowerCase().includes(normalizedSearch);
    const normalizedProfile = isBrowserAuthProfile(panel.providerHealth.auth_profile)
      ? "browser_oauth_pkce"
      : "static_api_key";
    const matchesProfile = authFilter === "all" || normalizedProfile === authFilter;

    return matchesSearch && matchesProfile;
  });
  const activeSessionCount = providerSessions.filter((session) => session.auth_state === "active").length;
  const directUsableCount = accounts.filter(
    (panel) =>
      !panel.providerHealth.last_error_message &&
      (panel.providerHealth.auth_state === "active" || panel.providerHealth.auth_state === "ready"),
  ).length;
  const unhealthyCount = accounts.filter((panel) => panel.providerHealth.last_error_message).length;
  const onboardingCards = buildOnboardingCards(providerDescriptors, accounts, providerSessions);

  return (
    <section className="space-y-5" aria-labelledby="accounts-heading">
      <header className="rounded-[1.75rem] border border-border/80 bg-card/85 p-6 shadow-[0_22px_60px_rgba(0,0,0,0.22)]">
        <div className="flex flex-col gap-5 xl:flex-row xl:items-start xl:justify-between">
          <div className="max-w-3xl space-y-3">
            <p className="text-xs font-semibold uppercase tracking-[0.22em] text-primary">
              Operations Console / Upstream Control
            </p>
            <div className="space-y-2">
              <h2 id="accounts-heading" className="text-4xl font-extrabold tracking-tight text-foreground">
                Upstream Nodes
              </h2>
              <p className="max-w-2xl text-sm leading-6 text-muted-foreground">
                Manage traffic-bearing upstream endpoints with pool-specific views. This surface keeps official
                account identity, session health, and capability diagnostics together while routing policy remains
                separate.
              </p>
            </div>
          </div>

          <div className="flex flex-wrap gap-3">
            <Button
              type="button"
              onClick={() => {
                setImportErrorMessage(null);
                setIsImportDialogOpen(true);
              }}
            >
              Add official account
            </Button>
            <Button
              disabled={sessionActionPending !== null}
              type="button"
              variant="outline"
              onClick={() => void handleRefreshAll()}
            >
              {sessionActionPending === "refresh:all" ? "Refreshing health..." : "Refresh health"}
            </Button>
          </div>
        </div>
      </header>
      <div className="inline-flex rounded-2xl border border-border/80 bg-card/80 p-1.5">
        <button
          aria-pressed="true"
          className="rounded-xl bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground"
          type="button"
        >
          Official Accounts
        </button>
        <button
          aria-pressed="false"
          className="rounded-xl px-4 py-2 text-sm font-semibold text-muted-foreground transition hover:bg-muted hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
          disabled={!onOpenRelays}
          type="button"
          onClick={() => onOpenRelays?.()}
        >
          Relay Endpoints
        </button>
      </div>

      <div className="rounded-2xl border border-border/80 bg-accent/30 px-5 py-4 text-sm leading-6 text-accent-foreground">
        Official Accounts show verifiable allowance modules only. Plan tags are detected from token diagnostics,
        not manually configured. Unknown quotas stay in reserved extension fields, and routing order is still
        controlled by policies.
      </div>

      {errorMessage ? (
        <div
          role="alert"
          className="rounded-2xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground"
        >
          {errorMessage}
        </div>
      ) : null}
      {authActionMessage ? (
        <div className="rounded-2xl border border-primary/30 bg-primary/10 px-4 py-3 text-sm text-primary-foreground">
          {authActionMessage}
        </div>
      ) : null}
      {sessionActionError ? (
        <div
          role="alert"
          className="rounded-2xl border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground"
        >
          {sessionActionError}
        </div>
      ) : null}

      <div className="space-y-5 rounded-[1.75rem] border border-border/80 bg-card/75 p-5 shadow-[0_20px_50px_rgba(0,0,0,0.18)]">
        <div className="flex flex-col gap-4 rounded-2xl border border-border/70 bg-background/30 p-4 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex flex-1 flex-col gap-3 md:flex-row">
            <label className="flex-1 space-y-2 text-sm font-medium text-foreground">
              Search accounts, workspaces, and credentials
              <input
                className="w-full rounded-xl border border-border bg-background/80 px-3 py-2.5 text-sm text-foreground outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/20"
                placeholder="Search by display name, provider, or account id"
                value={searchValue}
                onChange={(event) => setSearchValue(event.target.value)}
              />
            </label>
            <label className="space-y-2 text-sm font-medium text-foreground md:min-w-56">
              Auth profile
              <select
                className="w-full rounded-xl border border-border bg-background/80 px-3 py-2.5 text-sm text-foreground outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/20"
                value={authFilter}
                onChange={(event) => setAuthFilter(event.target.value as FilterAuthProfile)}
              >
                <option value="all">All profiles</option>
                <option value="browser_oauth_pkce">Browser sign-in</option>
                <option value="static_api_key">API key required</option>
              </select>
            </label>
          </div>

          <div className="grid gap-3 sm:grid-cols-3">
            <div className="rounded-2xl border border-border/70 bg-card/80 px-4 py-3">
              <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Official nodes</p>
              <p className="mt-2 text-2xl font-semibold text-foreground">{accounts.length}</p>
            </div>
            <div className="rounded-2xl border border-border/70 bg-card/80 px-4 py-3">
              <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Direct usable with Codex</p>
              <p className="mt-2 text-2xl font-semibold text-foreground">{directUsableCount}</p>
            </div>
            <div className="rounded-2xl border border-border/70 bg-card/80 px-4 py-3">
              <p className="text-xs uppercase tracking-[0.18em] text-muted-foreground">Session modules available</p>
              <p className="mt-2 text-2xl font-semibold text-foreground">{activeSessionCount}</p>
            </div>
          </div>
        </div>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.7fr)_360px]">
          <div className="space-y-4">
            <div className="flex items-center justify-between px-1">
              <div>
                <h3 className="text-lg font-semibold text-foreground">Official account pool</h3>
                <p className="text-sm text-muted-foreground">
                  {filteredAccounts.length} visible account{filteredAccounts.length === 1 ? "" : "s"} · {unhealthyCount} flagged for review
                </p>
              </div>
            </div>

            {filteredAccounts.length === 0 ? (
              <Card className="rounded-3xl border-dashed bg-background/20">
                <CardHeader>
                  <CardTitle>No accounts match this view</CardTitle>
                  <CardDescription>
                    Clear the search or filter, or add a new official account to start building the pool.
                  </CardDescription>
                </CardHeader>
              </Card>
            ) : (
              <div className="grid gap-4 md:grid-cols-2 2xl:grid-cols-3">
                {filteredAccounts.map((panel) => {
                  const linkedSession = sessionByAccountId.get(panel.account.account_id);
                  const canManageSession = linkedSession && isOpenAiProvider(linkedSession.provider_id);

                  return (
                    <Card key={panel.account.account_id} className="rounded-3xl border-border/80 bg-background/45 p-0 shadow-[0_18px_40px_rgba(0,0,0,0.16)]">
                      <CardHeader className="mb-0 space-y-4 border-b border-border/70 px-5 py-5">
                        <div className="flex items-start justify-between gap-3">
                          <div className="space-y-2">
                            <div className="flex flex-wrap items-center gap-2">
                              <CardTitle className="text-lg">{panel.account.name}</CardTitle>
                              <StatusBadge className="border-current/10 text-xs" variant={statusVariant(panel.providerHealth.auth_state, panel.providerHealth.last_error_message)}>
                                {formatAuthState(panel.providerHealth.auth_state)}
                              </StatusBadge>
                            </div>
                            <CardDescription className="font-mono text-xs">{panel.account.account_id}</CardDescription>
                          </div>
                          <Badge variant="outline">{formatProviderName(panel.account.provider)}</Badge>
                        </div>

                        <div className="flex flex-wrap items-center gap-2">
                          <Badge variant="secondary">{authProfileLabel(panel.providerHealth.auth_profile)}</Badge>
                          <Badge variant="outline">{linkedSession ? "Session linked" : "Session missing"}</Badge>
                        </div>
                      </CardHeader>

                      <CardContent className="space-y-4 px-5 py-5">
                        <div className="grid gap-3 sm:grid-cols-2">
                          <div className="rounded-2xl border border-border/70 bg-card/80 p-3">
                            <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Balance</p>
                            {panel.balanceSnapshot ? (
                              <div className="mt-2 space-y-2 text-sm leading-6 text-foreground">
                                <p>Balance state: {panel.balanceSnapshot.balance.kind}</p>
                                {panel.balanceSnapshot.balance.kind === "queryable" ? (
                                  <>
                                    <p>Total: {panel.balanceSnapshot.balance.total}</p>
                                    <p>Used: {panel.balanceSnapshot.balance.used}</p>
                                  </>
                                ) : (
                                  <p>{getBalanceSummary(panel)}</p>
                                )}
                              </div>
                            ) : (
                              <p className="mt-2 text-sm leading-6 text-foreground">{getBalanceSummary(panel)}</p>
                            )}
                          </div>
                          <div className="rounded-2xl border border-border/70 bg-card/80 p-3">
                            <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Capability</p>
                            {panel.capabilityDetail ? (
                              <div className="mt-2 space-y-2 text-sm leading-6 text-foreground">
                                <p>
                                  Refresh support:{" "}
                                  {panel.capabilityDetail.refresh_capability === null
                                    ? "unknown"
                                    : String(panel.capabilityDetail.refresh_capability)}
                                </p>
                                <p>
                                  Balance capability: {panel.capabilityDetail.balance_capability}
                                </p>
                              </div>
                            ) : (
                              <p className="mt-2 text-sm leading-6 text-foreground">
                                {panel.capabilityError ?? "Capability detail unavailable."}
                              </p>
                            )}
                          </div>
                        </div>

                        <div className="rounded-2xl border border-border/70 bg-card/80 p-3">
                          <p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Last auth issue</p>
                          <p className="mt-2 text-sm leading-6 text-foreground">{panel.providerHealth.last_error_message ?? "No active auth errors."}</p>
                        </div>

                        <div className="flex flex-wrap gap-2">
                          {isOpenAiProvider(panel.account.provider) ? (
                            <Button
                              disabled={sessionActionPending !== null}
                              size="sm"
                              type="button"
                              onClick={() => (linkedSession ? void handleRefreshProviderSession(panel.account.account_id) : void handleStartOpenAiLogin())}
                            >
                              {linkedSession ? "Refresh session" : "Start browser sign-in"}
                            </Button>
                          ) : null}
                          {canManageSession ? (
                            <Button
                              disabled={sessionActionPending !== null}
                              size="sm"
                              type="button"
                              variant="outline"
                              onClick={() => void handleLogoutProviderSession(panel.account.account_id)}
                            >
                              Sign out
                            </Button>
                          ) : null}
                        </div>
                      </CardContent>
                    </Card>
                  );
                })}
              </div>
            )}
          </div>

          <div className="space-y-4">
            <Card className="rounded-3xl border-border/80 bg-background/40">
              <CardHeader>
                <CardTitle>Connection playbooks</CardTitle>
                <CardDescription>Provider-specific onboarding paths stay visible next to the pool so browser flows and API-key requirements remain obvious.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {onboardingCards.map((card) => {
                  const supportsBrowserLogin = isBrowserAuthProfile(card.authProfile);
                  const isOpenAiCard = isOpenAiProvider(card.providerId);

                  return (
                    <div key={card.providerId} className="rounded-2xl border border-border/70 bg-card/80 p-4">
                      <div className="flex items-start justify-between gap-3">
                        <div className="space-y-1">
                          <h3 className="text-base font-semibold text-foreground">{authProfileLabel(card.authProfile)}</h3>
                          <p className="text-sm text-muted-foreground">{card.providerId}</p>
                        </div>
                        <Badge variant="outline">{card.connectedAccounts} linked</Badge>
                      </div>
                      <p className="mt-3 text-sm leading-6 text-muted-foreground">
                        Auth profile: {authProfileLabel(card.authProfile)}. {authProfileGuidance(card.authProfile)}
                      </p>
                      <div className="mt-4 flex flex-wrap gap-2">
                        {isOpenAiCard && supportsBrowserLogin ? (
                          <Button disabled={sessionActionPending !== null} size="sm" type="button" onClick={() => void handleStartOpenAiLogin()}>
                            Sign in with OpenAI
                          </Button>
                        ) : null}
                        {!supportsBrowserLogin ? <Badge variant="secondary">Credential binding required</Badge> : null}
                      </div>
                    </div>
                  );
                })}
              </CardContent>
            </Card>

            <Card className="rounded-3xl border-border/80 bg-background/40">
              <CardHeader>
                <CardTitle>Session modules</CardTitle>
                <CardDescription>Manage persisted desktop sessions separately from imported account records.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {providerSessions.length === 0 ? (
                  <div className="rounded-2xl border border-dashed border-border/70 bg-card/60 p-4 text-sm text-muted-foreground">
                    No provider sessions stored yet.
                  </div>
                ) : (
                  providerSessions.map((session) => (
                    <div key={`${session.provider_id}:${session.account_id}`} className="rounded-2xl border border-border/70 bg-card/80 p-4">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold text-foreground">{session.display_name}</p>
                          <p className="mt-1 text-xs text-muted-foreground">{session.account_id}</p>
                        </div>
                        <StatusBadge className="border-current/10 text-xs" variant={statusVariant(session.auth_state, session.last_error_message ?? session.last_refresh_error ?? null)}>
                          {formatAuthState(session.auth_state)}
                        </StatusBadge>
                      </div>
                      <div className="mt-3 space-y-2 text-sm text-muted-foreground">
                        <p>Auth profile: {authProfileLabel(session.auth_profile)}</p>
                        <p>Last refresh: {formatEpochMs(session.last_refresh_at_ms)}</p>
                        <p>Expires at: {formatEpochMs(session.expires_at_ms)}</p>
                        <p>Last auth error: {session.last_error_message ?? session.last_refresh_error ?? "None"}</p>
                      </div>
                      <div className="mt-4 flex gap-2">
                        <Button disabled={sessionActionPending !== null} size="sm" type="button" onClick={() => void handleRefreshProviderSession(session.account_id)}>
                          Refresh
                        </Button>
                        <Button disabled={sessionActionPending !== null} size="sm" type="button" variant="outline" onClick={() => void handleLogoutProviderSession(session.account_id)}>
                          Sign out
                        </Button>
                      </div>
                    </div>
                  ))
                )}
              </CardContent>
            </Card>
          </div>
        </div>
      </div>

      {isImportDialogOpen ? (
        <div aria-modal="true" className="fixed inset-0 z-50 flex items-start justify-center bg-black/70 px-4 py-8 backdrop-blur-sm" role="dialog">
          <div className="w-full max-w-3xl rounded-[1.75rem] border border-border/80 bg-card p-6 shadow-[0_28px_90px_rgba(0,0,0,0.45)]">
            <div className="space-y-2">
              <p className="text-sm font-semibold uppercase tracking-[0.18em] text-primary">Official account management</p>
              <h3 className="text-2xl font-bold text-foreground">Add official account</h3>
              <p className="text-sm leading-6 text-muted-foreground">
                Bind an upstream endpoint, attach the session and token credential references, then refresh diagnostics to detect plan tags and public allowance modules from token context.
              </p>
            </div>

            <div className="mt-6">
              <AccountImportForm
                errorMessage={importErrorMessage}
                isSubmitting={isImportPending}
                onClose={() => setIsImportDialogOpen(false)}
                onSubmit={handleImportOfficialAccount}
                providerOptions={providerOptions}
              />
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
