// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { JSDOM } from "jsdom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createQueryClient } from "../lib/query-client";

if (typeof document === "undefined") {
  const dom = new JSDOM("<!doctype html><html><body></body></html>", {
    url: "http://localhost/",
  });

  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    navigator: dom.window.navigator,
    HTMLElement: dom.window.HTMLElement,
    Node: dom.window.Node,
    Event: dom.window.Event,
    CustomEvent: dom.window.CustomEvent,
    getComputedStyle: dom.window.getComputedStyle.bind(dom.window),
  });
}

const {
  act,
  fireEvent,
  render: rtlRender,
  screen,
} = await import("@testing-library/react");

// @ts-expect-error bun:test is only available when executed via `bun test`.
const bunMock = (await import("bun:test").catch(() => null))?.mock;

function createMockState() {
  let listener:
    | ((summary: {
        name: string;
        allowedMode: "account_only" | "relay_only" | "hybrid" | null;
        rawAllowedMode: string;
        unavailableReason: string | null;
      }) => void)
    | null = null;

  return {
    addRelay: vi.fn(),
    createPlatformKey: vi.fn(),
    disablePlatformKey: vi.fn(),
    enablePlatformKey: vi.fn(),
    emitDefaultKeySummaryChanged(summary: {
      name: string;
      allowedMode: "account_only" | "relay_only" | "hybrid" | null;
      rawAllowedMode: string;
      unavailableReason: string | null;
    }) {
      listener?.(summary);
    },
    exportRuntimeDiagnostics: vi.fn(),
    getAccountCapabilityDetail: vi.fn(),
    getDefaultKeySummary: vi.fn(),
    getLogSummary: vi.fn(),
    getProviderDiagnostics: vi.fn(),
    getRuntimeLogMetadata: vi.fn(),
    getRelayCapabilityDetail: vi.fn(),
    getUsageRequestDetail: vi.fn(),
    importOfficialAccountLogin: vi.fn(),
    listenForDefaultKeySummaryChanged: vi.fn(async (handler) => {
      listener = handler;
      return () => {
        listener = null;
      };
    }),
    listAccounts: vi.fn(),
    listProviderDescriptors: vi.fn(),
    listProviderInventory: vi.fn(),
    listProviderSessions: vi.fn(),
    listPlatformKeys: vi.fn(),
    listPolicies: vi.fn(),
    listRelays: vi.fn(),
    logoutOpenAiSession: vi.fn(),
    listUsageRequestHistory: vi.fn(),
    queryUsageLedger: vi.fn(),
    refreshAccountBalance: vi.fn(),
    refreshOpenAiSession: vi.fn(),
    refreshRelayBalance: vi.fn(),
    startOpenAiBrowserLogin: vi.fn(),
    testRelayConnection: vi.fn(),
    setDefaultKeyMode: vi.fn(),
    updatePolicy: vi.fn(),
  };
}

let mockState: ReturnType<typeof createMockState>;

if (bunMock?.module) {
  mockState = createMockState();
  bunMock.module("../lib/tauri", () => ({
    ...mockState,
  }));
} else {
  vi.mock("../lib/tauri", () => ({
    ...(mockState = createMockState()),
  }));
}

const { default: App } = (await import("../App")) as typeof import("../App");
const {
  addRelay,
  createPlatformKey,
  disablePlatformKey,
  enablePlatformKey,
  emitDefaultKeySummaryChanged,
  exportRuntimeDiagnostics,
  getAccountCapabilityDetail,
  getDefaultKeySummary,
  getLogSummary,
  getProviderDiagnostics,
  getRuntimeLogMetadata,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  importOfficialAccountLogin,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listProviderDescriptors,
  listProviderInventory,
  listProviderSessions,
  listPlatformKeys,
  listPolicies,
  listRelays,
  logoutOpenAiSession,
  listUsageRequestHistory,
  queryUsageLedger,
  refreshAccountBalance,
  refreshOpenAiSession,
  refreshRelayBalance,
  startOpenAiBrowserLogin,
  testRelayConnection,
  setDefaultKeyMode,
  updatePolicy,
} = mockState;

function renderApp() {
  return rtlRender(
    <QueryClientProvider client={createQueryClient()}>
      <App />
    </QueryClientProvider>,
  );
}

const render = (ui = <App />) =>
  rtlRender(<QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>);

describe("App shell", () => {
  beforeEach(() => {
    addRelay.mockReset();
    createPlatformKey.mockReset();
    disablePlatformKey.mockReset();
    enablePlatformKey.mockReset();
    exportRuntimeDiagnostics.mockReset();
    getAccountCapabilityDetail.mockReset();
    getDefaultKeySummary.mockReset();
    getLogSummary.mockReset();
    getProviderDiagnostics.mockReset();
    getRuntimeLogMetadata.mockReset();
    getRelayCapabilityDetail.mockReset();
    getUsageRequestDetail.mockReset();
    importOfficialAccountLogin.mockReset();
    listenForDefaultKeySummaryChanged.mockClear();
    listAccounts.mockReset();
    listProviderDescriptors.mockReset();
    listProviderInventory.mockReset();
    listProviderSessions.mockReset();
    listPlatformKeys.mockReset();
    listPolicies.mockReset();
    listRelays.mockReset();
    logoutOpenAiSession.mockReset();
    listUsageRequestHistory.mockReset();
    queryUsageLedger.mockReset();
    refreshAccountBalance.mockReset();
    refreshOpenAiSession.mockReset();
    refreshRelayBalance.mockReset();
    startOpenAiBrowserLogin.mockReset();
    testRelayConnection.mockReset();
    setDefaultKeyMode.mockReset();
    updatePolicy.mockReset();

    getDefaultKeySummary.mockResolvedValue({
      name: "default",
      allowedMode: "hybrid",
      rawAllowedMode: "hybrid",
      unavailableReason: null,
    });
    getLogSummary.mockResolvedValue({
      level: "info",
      last_event: "Loopback gateway ready for key 'default' in hybrid mode",
    });
    getProviderDiagnostics.mockResolvedValue({
      sections: [
        {
          id: "auth_health",
          title: "Auth health",
          status: "healthy",
          summary: "1 provider session available.",
          rows: [
            {
              key: "openai-primary",
              label: "OpenAI Primary",
              status: "healthy",
              value: "state=active | expires_at_ms=none | provider=openai_official",
              details: [
                { label: "account_id", value: "openai-primary" },
                { label: "last_refresh_at_ms", value: "none" },
              ],
            },
          ],
        },
        {
          id: "provider_health",
          title: "Provider health",
          status: "healthy",
          summary: "2 runtime endpoint projections available.",
          rows: [
            {
              key: "official-primary",
              label: "Primary Publisher",
              status: "healthy",
              value: "pool=official | available=true | priority=10 | health=Healthy",
              details: [{ label: "provider_id", value: "openai" }],
            },
          ],
        },
      ],
    });
    getRuntimeLogMetadata.mockResolvedValue({
      log_dir: "<app-local-data>/logs",
      files: [
        {
          name: "gateway.log",
          path: "<app-local-data>/logs/gateway.log",
          size: 128,
          mtime: 1713370000000,
        },
        {
          name: "gateway.1.log",
          path: "<app-local-data>/logs/gateway.1.log",
          size: 96,
          mtime: 1713369000000,
        },
      ],
    });
    exportRuntimeDiagnostics.mockResolvedValue(
      "<app-local-data>/logs/diagnostics/diagnostics-manifest.txt",
    );
    listAccounts.mockResolvedValue([
      { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
    ]);
    listProviderDescriptors.mockResolvedValue([
      {
        provider_id: "openai",
        auth_profile: "browser_oauth_pkce",
        supports_model_discovery: true,
        supports_capability_probe: true,
      },
      {
        provider_id: "claude_official",
        auth_profile: "static_api_key",
        supports_model_discovery: true,
        supports_capability_probe: true,
      },
    ]);
    listProviderInventory.mockResolvedValue({
      accounts: [
        {
          provider_id: "openai_official",
          account_id: "official-primary",
          display_name: "Primary Publisher",
          auth_state: "active",
          available: true,
          registered: true,
          base_url: null,
        },
        {
          provider_id: "generic_openai_compatible",
          account_id: "openai-primary",
          display_name: "OpenAI Primary",
          auth_state: "expired",
          available: false,
          registered: true,
          base_url: "https://gateway.example.test/v1",
        },
      ],
      models: [
        {
          provider_id: "openai_official",
          account_id: "official-primary",
          model_id: "gpt-5-mini",
          supports_tools: true,
          supports_streaming: true,
          supports_reasoning: true,
          source: "default",
        },
        {
          provider_id: "generic_openai_compatible",
          account_id: "openai-primary",
          model_id: "gpt-4.1-mini",
          supports_tools: true,
          supports_streaming: true,
          supports_reasoning: false,
          source: "manual",
        },
      ],
    });
    listProviderSessions.mockResolvedValue([
      {
        provider_id: "openai_official",
        account_id: "openai-primary",
        display_name: "OpenAI Primary",
        auth_state: "active",
        expires_at_ms: null,
        last_refresh_at_ms: null,
        last_refresh_error: null,
      },
    ]);
    importOfficialAccountLogin.mockResolvedValue({
      account_id: "imported-openai",
      name: "Imported OpenAI",
      provider: "openai",
    });
    refreshAccountBalance.mockImplementation(async (accountId: string) => ({
      account_id: accountId,
      provider: "openai",
      refreshed_at: "1713370000",
      balance: {
        kind: "non_queryable",
        reason: "official accounts do not expose a balance endpoint",
      },
    }));
    getAccountCapabilityDetail.mockImplementation(async (accountId: string) => ({
      account_id: accountId,
      provider: "openai",
      refresh_capability: true,
      balance_capability: "non_queryable",
    }));
    refreshOpenAiSession.mockResolvedValue({
      provider_id: "openai_official",
      account_id: "openai-primary",
      display_name: "OpenAI Primary",
      auth_state: "active",
      expires_at_ms: 1_731_111_111_000,
      last_refresh_at_ms: 1_731_111_000_500,
      last_refresh_error: null,
    });
    startOpenAiBrowserLogin.mockResolvedValue({
      summary: {
        provider_id: "openai_official",
        account_id: "openai-primary",
        display_name: "OpenAI Primary",
        auth_state: "pending",
        expires_at_ms: null,
        last_refresh_at_ms: null,
        last_refresh_error: null,
      },
      authorization_url: "https://auth.openai.com/oauth/authorize?response_type=code",
      callback_url: "http://127.0.0.1:1455/auth/openai/callback",
    });
    logoutOpenAiSession.mockResolvedValue(true);
    listPolicies.mockResolvedValue([
      {
        policy_id: "default-policy",
        name: "default",
        status: "active",
        selection_order: ["official-primary", "relay-newapi"],
        cross_pool_fallback: false,
        retry_budget: 1,
        timeout_open_after: 2,
        server_error_open_after: 3,
        cooldown_ms: 1000,
        half_open_after_ms: 1000,
        success_close_after: 2,
      },
    ]);
    listRelays.mockResolvedValue([
      {
        relay_id: "relay-newapi",
        name: "Local Gateway",
        endpoint: "http://127.0.0.1:8787",
      },
      {
        relay_id: "relay-nobalance",
        name: "Upstream Proxy",
        endpoint: "https://relay.example.test",
      },
    ]);
    addRelay.mockResolvedValue({
      relay_id: "relay-managed",
      name: "Managed Relay",
      endpoint: "https://managed.example.test",
    });
    refreshRelayBalance.mockImplementation(async (relayId: string) => {
      if (relayId === "relay-newapi") {
        return {
          relay_id: "relay-newapi",
          endpoint: "http://127.0.0.1:8787",
          balance: {
            kind: "queryable",
            adapter: "new_api",
            balance: { total: "25.00", used: "7.50" },
          },
        };
      }
      return {
        relay_id: relayId,
        endpoint: relayId === "relay-managed" ? "https://managed.example.test" : "https://relay.example.test",
        balance:
          relayId === "relay-managed"
            ? {
                kind: "queryable",
                adapter: "new_api",
                balance: { total: "10.00", used: "0.50" },
              }
            : { kind: "unsupported", reason: "relay does not provide a balance endpoint" },
      };
    });
    getRelayCapabilityDetail.mockImplementation(async (relayId: string) => {
      if (relayId === "relay-newapi") {
        return {
          relay_id: "relay-newapi",
          endpoint: "http://127.0.0.1:8787",
          balance_capability: { kind: "queryable", adapter: "new_api" },
        };
      }
      return {
        relay_id: relayId,
        endpoint: relayId === "relay-managed" ? "https://managed.example.test" : "https://relay.example.test",
        balance_capability:
          relayId === "relay-managed"
            ? { kind: "queryable", adapter: "new_api" }
            : { kind: "unsupported" },
      };
    });
    testRelayConnection.mockResolvedValue({
      relay_id: "relay-managed",
      endpoint: "https://managed.example.test",
      status: "ok",
      latency_ms: 11,
    });
    listPlatformKeys.mockResolvedValue([
      {
        id: "default-key",
        name: "default",
        policy_id: "default-policy",
        allowed_mode: "hybrid",
        enabled: true,
      },
    ]);
    createPlatformKey.mockResolvedValue({
      id: "ops-key",
      name: "Operations Key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
      enabled: true,
      secret: "ck_local_mocked_ops_key_secret",
    });
    disablePlatformKey.mockResolvedValue({
      id: "ops-key",
      name: "Operations Key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
      enabled: false,
    });
    enablePlatformKey.mockResolvedValue({
      id: "ops-key",
      name: "Operations Key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
      enabled: true,
    });
    updatePolicy.mockImplementation(async (input: unknown) => input);
    listUsageRequestHistory.mockResolvedValue([
      {
        request_id: "req-2",
        endpoint_id: "relay-1",
        input_tokens: 40,
        output_tokens: 15,
        cache_read_tokens: 5,
        cache_write_tokens: 2,
        total_tokens: 62,
        cost: { amount: null, provenance: "unknown" },
      },
      {
        request_id: "req-1",
        endpoint_id: "official-1",
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        total_tokens: 160,
        cost: { amount: "0.0123", provenance: "estimated" },
      },
    ]);
    queryUsageLedger.mockResolvedValue({
      entries: [
        {
          request_id: "req-2",
          endpoint_id: "relay-1",
          input_tokens: 40,
          output_tokens: 15,
          cache_read_tokens: 5,
          cache_write_tokens: 2,
          total_tokens: 62,
          cost: { amount: null, provenance: "unknown" },
        },
        {
          request_id: "req-1",
          endpoint_id: "official-1",
          input_tokens: 120,
          output_tokens: 30,
          cache_read_tokens: 10,
          cache_write_tokens: 0,
          total_tokens: 160,
          cost: { amount: "0.0123", provenance: "estimated" },
        },
      ],
      total_tokens: 222,
      total_cost: { amount: null, provenance: "unknown" },
    });
    getUsageRequestDetail.mockResolvedValue({
      request_id: "req-1",
      endpoint_id: "official-1",
      input_tokens: 120,
      output_tokens: 30,
      cache_read_tokens: 10,
      cache_write_tokens: 0,
      total_tokens: 160,
      cost: { amount: "0.0123", provenance: "estimated" },
      route_explanation: {
        selected_candidate_id: "official-primary",
        rejected_candidates: ["relay-newapi"],
        fallback_trigger: "fallback_after_429",
        final_reason: "selected candidate returned upstream status 200",
      },
    });
    setDefaultKeyMode.mockResolvedValue({
      name: "default",
      allowedMode: "relay_only",
      rawAllowedMode: "relay_only",
      unavailableReason: null,
    });
  });

  it("renders the production desktop shell with persistent navigation and header chrome", async () => {
    render(<App />);
    await screen.findByText("Runtime status");

    expect(screen.getByRole("navigation", { name: /primary/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: /codexlag/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2, name: /overview/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: /gateway overview/i })).toBeInTheDocument();
  });

  it("renders the shared desktop shell and highlights the active page", async () => {
    render(<App />);
    await screen.findByText("Runtime status");

    const overviewButton = screen.getByRole("button", { name: /overview/i });
    expect(overviewButton).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("main")).toBeInTheDocument();
  });

  it("keeps the six primary navigation targets available from the new shell", async () => {
    render(<App />);
    await screen.findByText("Runtime status");

    expect(screen.getByRole("button", { name: /overview/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /accounts/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /relays/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /platform keys/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /policies/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /logs/i })).toBeInTheDocument();
  });

  it("renders overview status cards with balance and usage summaries", async () => {
    render(<App />);

    expect(await screen.findByText("Runtime status")).toBeInTheDocument();
    expect(screen.getByText("Balance observability")).toBeInTheDocument();
    expect(screen.getByText("Usage ledger")).toBeInTheDocument();
    expect(screen.getByText("Runtime diagnostics")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Export diagnostics" })).toBeInTheDocument();
    expect(screen.getByText("Default key state | Current mode: hybrid")).toBeInTheDocument();
    expect(screen.getByText("Non-queryable accounts: 1")).toBeInTheDocument();
    expect(screen.getByText("Queryable relays: 1")).toBeInTheDocument();
    expect(screen.getByText("Account refresh failures: 0")).toBeInTheDocument();
    expect(screen.getByText("Relay refresh failures: 0")).toBeInTheDocument();
    expect(screen.getByText("Total ledger tokens: 222")).toBeInTheDocument();
    expect(screen.getByText("Usage cost provenance: unknown")).toBeInTheDocument();
    expect(screen.getByText("Log directory: <app-local-data>/logs")).toBeInTheDocument();
    expect(screen.getByText("Tracked log files: 2")).toBeInTheDocument();
    expect(screen.getByRole("table", { name: "Runtime log files metadata" })).toBeInTheDocument();
    expect(screen.getByText("gateway.log")).toBeInTheDocument();
    expect(queryUsageLedger).toHaveBeenCalledTimes(1);
  });

  it("shows the overview diagnostics panel and default key operations in the rebuilt pages", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: /gateway overview/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /capability matrix/i })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: /runtime diagnostics/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /default key mode/i })).toBeInTheDocument();
  });

  it("exports runtime diagnostics and renders manifest path fallback", async () => {
    render(<App />);

    const exportButton = await screen.findByRole("button", { name: "Export diagnostics" });
    fireEvent.click(exportButton);

    expect(exportRuntimeDiagnostics).toHaveBeenCalledTimes(1);
    expect(
      await screen.findByText(
        "Diagnostics manifest: <app-local-data>/logs/diagnostics/diagnostics-manifest.txt",
      ),
    ).toBeInTheDocument();

    exportRuntimeDiagnostics.mockRejectedValueOnce(new Error("export failed"));
    fireEvent.click(exportButton);

    expect(await screen.findByText("Diagnostics manifest: Export failed")).toBeInTheDocument();
  });

  it("keeps overview available when runtime diagnostics loading fails", async () => {
    getRuntimeLogMetadata.mockRejectedValueOnce(new Error("diagnostics unavailable"));

    render(<App />);

    expect(await screen.findByText("Runtime status")).toBeInTheDocument();
    expect(screen.getByText("Balance observability")).toBeInTheDocument();
    expect(screen.getByText("Usage ledger")).toBeInTheDocument();
    expect(screen.getByText("Runtime diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Total ledger tokens: 222")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getByText("Log directory: unavailable")).toBeInTheDocument();
    expect(screen.getByText("Tracked log files: 0")).toBeInTheDocument();
  });

  it("renders a query-backed capability matrix with filters and column visibility", async () => {
    render(<App />);

    const capabilityMatrix = await screen.findByRole("table", { name: /capability matrix/i });
    expect(capabilityMatrix).toBeInTheDocument();
    expect(screen.getAllByText("OpenAI Primary").length).toBeGreaterThan(0);
    expect(screen.getByText(/expired \(degraded\)/i)).toBeInTheDocument();

    await act(async () => {
      fireEvent.change(screen.getByLabelText("Provider scope"), {
        target: { value: "generic_openai_compatible" },
      });
    });
    expect(screen.queryByRole("cell", { name: "Primary Publisher" })).not.toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "OpenAI Primary" })).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /columns/i }));
    });
    await act(async () => {
      fireEvent.click(screen.getByLabelText("Source"));
    });
    expect(screen.queryByText("manual")).not.toBeInTheDocument();
  });

  it("loads account balances and capability details", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByText("Primary Publisher")).toBeInTheDocument();
    expect(screen.getByText("Balance state: non_queryable")).toBeInTheDocument();
    expect(
      screen.getByText("official accounts do not expose a balance endpoint"),
    ).toBeInTheDocument();
    expect(screen.getByText("Refresh support: true")).toBeInTheDocument();
    expect(screen.getByText("Balance capability: non_queryable")).toBeInTheDocument();
    expect(refreshAccountBalance).toHaveBeenCalledWith("official-primary");
    expect(getAccountCapabilityDetail).toHaveBeenCalledWith("official-primary");
  });

  it("keeps official accounts focused on browser login instead of manual import", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByRole("heading", { name: /browser sign-in/i })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /import official account/i })).not.toBeInTheDocument();
    expect(importOfficialAccountLogin).not.toHaveBeenCalled();
  });

  it("shows provider-specific onboarding actions for browser and api-key accounts", async () => {
    listAccounts.mockResolvedValue([
      { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
      { account_id: "anthropic-direct", name: "Anthropic Direct", provider: "anthropic" },
    ]);

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByRole("heading", { name: /browser sign-in/i })).toBeInTheDocument();
    expect(
      (
        await screen.findAllByText(
          /authenticate this provider with a configured api key before using the account/i,
        )
      ).length,
    ).toBeGreaterThan(0);
  });

  it("renders onboarding guidance per provider descriptor", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByRole("heading", { name: /browser sign-in/i })).toBeInTheDocument();
    expect(screen.getByText("claude_official")).toBeInTheDocument();
    expect(screen.getByText("API key required")).toBeInTheDocument();
  });

  it("shows degraded account health and last error guidance", async () => {
    listAccounts.mockResolvedValue([
      { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
      { account_id: "claude-primary", name: "Claude Primary", provider: "claude_official" },
    ]);
    refreshAccountBalance.mockImplementation(async (accountId: string) => ({
      account_id: accountId,
      provider: accountId === "claude-primary" ? "claude_official" : "openai",
      refreshed_at: "1713370000",
      balance: {
        kind: "non_queryable",
        reason: "balance endpoint unavailable",
      },
    }));
    getAccountCapabilityDetail.mockImplementation(async (accountId: string) => ({
      account_id: accountId,
      provider: accountId === "claude-primary" ? "claude_official" : "openai",
      refresh_capability: accountId !== "claude-primary",
      balance_capability: "non_queryable",
    }));
    listProviderSessions.mockResolvedValue([
      {
        provider_id: "openai",
        account_id: "official-primary",
        display_name: "Primary Publisher",
        auth_state: "active",
        auth_profile: "browser_oauth_pkce",
        expires_at_ms: null,
        last_refresh_at_ms: null,
        last_refresh_error: null,
        last_error_message: null,
      },
      {
        provider_id: "claude_official",
        account_id: "claude-primary",
        display_name: "Claude Primary",
        auth_state: "expired",
        auth_profile: "static_api_key",
        expires_at_ms: null,
        last_refresh_at_ms: null,
        last_refresh_error: "Session expired",
        last_error_message: "Re-authenticate with a new API key.",
      },
    ]);

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findAllByText(/expired/i)).not.toHaveLength(0);
    expect(screen.getAllByText(/re-authenticate with a new api key/i)).not.toHaveLength(0);
  });

  it("keeps accounts visible and preserves fallback onboarding when descriptors fail", async () => {
    listProviderDescriptors.mockRejectedValue(new Error("descriptor timeout"));

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByText("Primary Publisher")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /browser sign-in/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Sign in with OpenAI" })).toBeInTheDocument();
    expect(screen.queryByRole("alert", { name: /failed to load accounts/i })).not.toBeInTheDocument();
  });

  it("starts browser login and manages persisted OpenAI provider sessions", async () => {
    listProviderSessions
      .mockResolvedValueOnce([
        {
          provider_id: "openai_official",
          account_id: "openai-primary",
          display_name: "OpenAI Primary",
          auth_state: "active",
          expires_at_ms: null,
          last_refresh_at_ms: null,
          last_refresh_error: null,
        },
      ])
      .mockResolvedValueOnce([
        {
          provider_id: "openai_official",
          account_id: "openai-primary",
          display_name: "OpenAI Primary",
          auth_state: "pending",
          expires_at_ms: null,
          last_refresh_at_ms: null,
          last_refresh_error: null,
        },
      ])
      .mockResolvedValueOnce([
        {
          provider_id: "openai_official",
          account_id: "openai-primary",
          display_name: "OpenAI Primary",
          auth_state: "active",
          expires_at_ms: 1_731_111_111_000,
          last_refresh_at_ms: 1_731_111_000_500,
          last_refresh_error: null,
        },
      ])
      .mockResolvedValueOnce([]);

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Sign in with OpenAI" }));
    });
    expect(startOpenAiBrowserLogin).toHaveBeenCalledTimes(1);
    expect(
      await screen.findByText(/Browser login started for OpenAI Primary/),
    ).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    });
    expect(refreshOpenAiSession).toHaveBeenCalledWith("openai-primary");
    expect(await screen.findByText("Refreshed OpenAI session: openai-primary")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    });
    expect(logoutOpenAiSession).toHaveBeenCalledWith("openai-primary");
    expect(await screen.findByText("Signed out OpenAI session: openai-primary")).toBeInTheDocument();
  });

  it("renders OpenAI session and relay creation as structured operations panels", async () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Official Accounts" })).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /accounts/i }));
    });
    expect(await screen.findByRole("heading", { level: 1, name: /official accounts/i })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: /browser sign-in/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /provider sessions/i })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /import official account/i })).not.toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /relays/i }));
    });
    expect(await screen.findByRole("heading", { name: /manage relays/i })).toBeInTheDocument();
  });

  it("loads relay balances and capability details", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Relays" }));
    });

    expect(await screen.findByText("Endpoint: http://127.0.0.1:8787")).toBeInTheDocument();
    expect(screen.getByText("Balance state: queryable")).toBeInTheDocument();
    expect(screen.getByText("Adapter: new_api")).toBeInTheDocument();
    expect(screen.getByText("Total: 25.00")).toBeInTheDocument();
    expect(screen.getByText("Used: 7.50")).toBeInTheDocument();
    expect(screen.getByText("Endpoint: https://relay.example.test")).toBeInTheDocument();
    expect(screen.getByText("Balance state: unsupported")).toBeInTheDocument();
    expect(screen.getByText("Capability: unsupported")).toBeInTheDocument();
    expect(refreshRelayBalance).toHaveBeenCalledWith("relay-newapi");
    expect(getRelayCapabilityDetail).toHaveBeenCalledWith("relay-newapi");
  });

  it("shows claim-derived OpenAI plan metadata on the accounts page", async () => {
    getAccountCapabilityDetail.mockImplementation(async (accountId: string) => ({
      account_id: accountId,
      provider: "openai",
      refresh_capability: true,
      balance_capability: "non_queryable",
      status: "active",
      account_identity: "user@example.com",
      entitlement: {
        plan_type: "pro",
        subscription_active_start: "2026-04-01T00:00:00Z",
        subscription_active_until: "2026-05-01T00:00:00Z",
        claim_source: "id_token_claim",
      },
    }));

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(await screen.findByText("Plan: pro")).toBeInTheDocument();
    expect(screen.getByText("Source: id_token_claim")).toBeInTheDocument();
    expect(screen.getByText("Active until: 2026-05-01T00:00:00Z")).toBeInTheDocument();
  });

  it("creates a relay and runs connection test flow", async () => {
    listRelays
      .mockResolvedValueOnce([
        {
          relay_id: "relay-newapi",
          name: "Local Gateway",
          endpoint: "http://127.0.0.1:8787",
        },
      ])
      .mockResolvedValueOnce([
        {
          relay_id: "relay-newapi",
          name: "Local Gateway",
          endpoint: "http://127.0.0.1:8787",
        },
      ])
      .mockResolvedValueOnce([
        {
          relay_id: "relay-newapi",
          name: "Local Gateway",
          endpoint: "http://127.0.0.1:8787",
        },
        {
          relay_id: "relay-managed",
          name: "Managed Relay",
          endpoint: "https://managed.example.test",
        },
      ]);

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Relays" }));
    });

    fireEvent.change(screen.getByLabelText("Relay ID"), { target: { value: "relay-managed" } });
    fireEvent.change(screen.getByLabelText("Relay Name"), { target: { value: "Managed Relay" } });
    fireEvent.change(screen.getByLabelText("Relay Endpoint"), {
      target: { value: "https://managed.example.test" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create relay" }));

    expect(addRelay).toHaveBeenCalledWith({
      relay_id: "relay-managed",
      name: "Managed Relay",
      endpoint: "https://managed.example.test",
      adapter: "newapi",
    });
    expect(await screen.findByText("Created relay: relay-managed")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Test relay relay-managed" }));
    expect(testRelayConnection).toHaveBeenCalledWith("relay-managed");
    expect(await screen.findByText("relay-managed: ok (11ms)")).toBeInTheDocument();
  });

  it("creates and disables a platform key", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Platform Keys" }));
    });

    fireEvent.change(screen.getByLabelText("Key ID"), { target: { value: "ops-key" } });
    fireEvent.change(screen.getByLabelText("Key Name"), { target: { value: "Operations Key" } });
    fireEvent.change(screen.getByLabelText("Policy ID"), { target: { value: "default-policy" } });
    fireEvent.click(screen.getByRole("button", { name: "Create key" }));

    expect(createPlatformKey).toHaveBeenCalledWith({
      key_id: "ops-key",
      name: "Operations Key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
    });
    expect(await screen.findByText("Generated secret")).toBeInTheDocument();
    expect(await screen.findByText("ck_local_mocked_ops_key_secret")).toBeInTheDocument();
    expect(await screen.findByText("Operations Key")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Disable key ops-key" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Disable key ops-key" }));
    expect(disablePlatformKey).toHaveBeenCalledWith("ops-key");
    expect(await screen.findByText("Disabled")).toBeInTheDocument();
  });

  it("shows the generated platform key secret after key creation", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /platform keys/i }));
    });

    fireEvent.change(screen.getByLabelText("Key ID"), { target: { value: "ops-key" } });
    fireEvent.change(screen.getByLabelText("Key Name"), { target: { value: "Operations Key" } });
    fireEvent.change(screen.getByLabelText("Policy ID"), { target: { value: "default-policy" } });
    fireEvent.click(screen.getByRole("button", { name: "Create key" }));

    expect(await screen.findByText(/generated secret/i)).toBeInTheDocument();
    expect(screen.getByText("ck_local_mocked_ops_key_secret")).toBeInTheDocument();
  });

  it("edits and saves policy rules", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Policies" }));
    });

    expect(await screen.findByLabelText("Policy Name")).toHaveValue("default");
    fireEvent.change(screen.getByLabelText("Policy Name"), { target: { value: "default-updated" } });
    fireEvent.click(screen.getByRole("button", { name: "Add relay-nobalance" }));
    fireEvent.click(screen.getByRole("button", { name: "Move relay-nobalance up" }));
    fireEvent.change(screen.getByLabelText("Cross Pool Fallback"), {
      target: { value: "false" },
    });
    fireEvent.change(screen.getByLabelText("Retry Budget"), { target: { value: "2" } });
    fireEvent.change(screen.getByLabelText("Timeout Open After"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("Server Error Open After"), { target: { value: "3" } });
    fireEvent.change(screen.getByLabelText("Cooldown (ms)"), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Half Open After (ms)"), {
      target: { value: "1000" },
    });
    fireEvent.change(screen.getByLabelText("Success Close After"), { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Save policy" }));

    expect(updatePolicy).toHaveBeenCalledWith({
      policy_id: "default-policy",
      name: "default-updated",
      selection_order: ["official-primary", "relay-nobalance", "relay-newapi"],
      cross_pool_fallback: false,
      retry_budget: 2,
      timeout_open_after: 3,
      server_error_open_after: 3,
      cooldown_ms: 1000,
      half_open_after_ms: 1000,
      success_close_after: 2,
    });
    expect(await screen.findByText("Policy saved: default-updated")).toBeInTheDocument();
  });

  it("renders policy fields from hydrated backend data", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /policies/i }));
    });

    expect(await screen.findByLabelText(/retry budget/i)).toHaveValue("1");
    expect(screen.getByLabelText(/^Selection Order$/)).toHaveValue(
      "official-primary, relay-newapi",
    );
  });

  it("shows validation feedback before saving an invalid policy", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /policies/i }));
    });

    const retryBudget = await screen.findByLabelText(/retry budget/i);
    fireEvent.change(retryBudget, { target: { value: "" } });
    fireEvent.click(screen.getByRole("button", { name: /save policy/i }));

    expect(await screen.findByText("Retry budget must be a positive integer.")).toBeInTheDocument();
    expect(updatePolicy).not.toHaveBeenCalled();
  });

  it("shows candidate preview and ordering controls while editing policies", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /policies/i }));
    });

    expect(await screen.findByRole("heading", { name: /candidate preview/i })).toBeInTheDocument();
    expect(screen.getByText(/eligible candidates: official-primary, relay-newapi/i)).toBeInTheDocument();
    expect(screen.getByText(/rejected candidates: relay-nobalance/i)).toBeInTheDocument();
    expect(screen.getByText(/configured preference starts with official-primary/i)).toBeInTheDocument();
    expect(screen.getByText(/not explicitly ordered/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Add relay-nobalance" }));
    expect(
      screen.getByText(/eligible candidates: official-primary, relay-newapi, relay-nobalance/i),
    ).toBeInTheDocument();
  });

  it("shows request history and request-detail affordances", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Logs & Usage" }));
    });

    expect(await screen.findByText("Request history")).toBeInTheDocument();
    expect(screen.getByText("req-2")).toBeInTheDocument();
    expect(screen.getByText("unknown")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "View request req-1" }));

    expect(getUsageRequestDetail).toHaveBeenCalledWith("req-1");
    expect(await screen.findByText("Request detail: req-1")).toBeInTheDocument();
    expect(screen.getByText("Cost provenance: estimated")).toBeInTheDocument();
    expect(screen.getByText("Final route: official-primary")).toBeInTheDocument();
    expect(screen.getByText("Rejected candidates: relay-newapi")).toBeInTheDocument();
    expect(screen.getByText("Fallback trigger: fallback_after_429")).toBeInTheDocument();
    expect(
      screen.getByText("Final routing reason: selected candidate returned upstream status 200"),
    ).toBeInTheDocument();
  });

  it("renders logs as a request history with detail and capability panels", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /logs/i }));
    });

    expect(await screen.findByRole("heading", { name: /request history/i })).toBeInTheDocument();
    expect(screen.getByText(/usage provenance/i)).toBeInTheDocument();
  });

  it("renders auth and provider diagnostics inside the logs console", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /logs/i }));
    });

    expect(await screen.findByRole("heading", { name: /provider diagnostics/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /auth health/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /provider health/i })).toBeInTheDocument();
  });

  it("reveals diagnostics detail rows when an operator expands a section row", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /logs/i }));
    });

    expect(await screen.findByRole("heading", { name: /provider diagnostics/i })).toBeInTheDocument();

    const detailToggles = screen.getAllByText(/view details/i);
    await act(async () => {
      fireEvent.click(detailToggles[0]);
    });

    expect(screen.getByText("account_id")).toBeInTheDocument();
    expect(screen.getByText("openai-primary")).toBeInTheDocument();
  });

  it("clears stale request detail when detail loading fails", async () => {
    getUsageRequestDetail
      .mockResolvedValueOnce({
        request_id: "req-1",
        endpoint_id: "official-1",
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        total_tokens: 160,
        cost: { amount: "0.0123", provenance: "estimated" },
      })
      .mockRejectedValueOnce(new Error("detail failed"));

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Logs & Usage" }));
    });

    fireEvent.click(screen.getByRole("button", { name: "View request req-1" }));
    expect(await screen.findByText("Request detail: req-1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View request req-2" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Failed to load request detail for req-2.",
    );
    expect(screen.queryByText("Request detail: req-1")).not.toBeInTheDocument();
  });

  it("updates the overview when the backend emits a default key summary change", async () => {
    render(<App />);
    await screen.findByText("Default key state | Current mode: hybrid");

    await act(async () => {
      emitDefaultKeySummaryChanged({
        name: "default",
        allowedMode: "account_only",
        rawAllowedMode: "account_only",
        unavailableReason: null,
      });
    });

    expect(
      await screen.findByText("Default key state | Current mode: account_only"),
    ).toBeInTheDocument();
  });

  it("shows unavailable reason when current default mode has no available endpoint", async () => {
    getDefaultKeySummary.mockResolvedValueOnce({
      name: "default",
      allowedMode: "relay_only",
      rawAllowedMode: "relay_only",
      unavailableReason: "no available endpoint for mode 'relay_only'",
    });

    const view = render(<App />);

    expect(await view.findByText("Default key state | Current mode: relay_only")).toBeInTheDocument();
    expect(view.getByText("no available endpoint for mode 'relay_only'")).toBeInTheDocument();
  });
});
