import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

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
  getRuntimeLogMetadata,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  importOfficialAccountLogin,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPlatformKeys,
  listPolicies,
  listRelays,
  listUsageRequestHistory,
  queryUsageLedger,
  refreshAccountBalance,
  refreshRelayBalance,
  testRelayConnection,
  setDefaultKeyMode,
  updatePolicy,
} = vi.hoisted(() => {
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
    listPlatformKeys: vi.fn(),
    listPolicies: vi.fn(),
    listRelays: vi.fn(),
    listUsageRequestHistory: vi.fn(),
    queryUsageLedger: vi.fn(),
    refreshAccountBalance: vi.fn(),
    refreshRelayBalance: vi.fn(),
    testRelayConnection: vi.fn(),
    setDefaultKeyMode: vi.fn(),
    updatePolicy: vi.fn(),
  };
});

vi.mock("../lib/tauri", () => ({
  addRelay,
  createPlatformKey,
  disablePlatformKey,
  enablePlatformKey,
  getAccountCapabilityDetail,
  getDefaultKeySummary,
  getLogSummary,
  getRuntimeLogMetadata,
  exportRuntimeDiagnostics,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  importOfficialAccountLogin,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPlatformKeys,
  listPolicies,
  listRelays,
  listUsageRequestHistory,
  queryUsageLedger,
  refreshAccountBalance,
  refreshRelayBalance,
  testRelayConnection,
  setDefaultKeyMode,
  updatePolicy,
}));

import App from "../App";

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
    getRuntimeLogMetadata.mockReset();
    getRelayCapabilityDetail.mockReset();
    getUsageRequestDetail.mockReset();
    importOfficialAccountLogin.mockReset();
    listenForDefaultKeySummaryChanged.mockClear();
    listAccounts.mockReset();
    listPlatformKeys.mockReset();
    listPolicies.mockReset();
    listRelays.mockReset();
    listUsageRequestHistory.mockReset();
    queryUsageLedger.mockReset();
    refreshAccountBalance.mockReset();
    refreshRelayBalance.mockReset();
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
    expect(screen.getByText("CodexLAG")).toBeInTheDocument();
    expect(screen.getByText("Gateway Overview")).toBeInTheDocument();
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

    expect(await screen.findByText(/runtime diagnostics/i)).toBeInTheDocument();
    expect(screen.getByText(/default key mode/i)).toBeInTheDocument();
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

  it("submits account import flow and reloads account cards", async () => {
    listAccounts
      .mockResolvedValueOnce([
        { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
      ])
      .mockResolvedValueOnce([
        { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
        { account_id: "imported-openai", name: "Imported OpenAI", provider: "openai" },
      ]);

    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    fireEvent.change(screen.getByLabelText("Account ID"), { target: { value: "imported-openai" } });
    fireEvent.change(screen.getByLabelText("Account Name"), { target: { value: "Imported OpenAI" } });
    fireEvent.change(screen.getByLabelText("Session Credential Ref"), {
      target: { value: "credential://official/session/imported-openai" },
    });
    fireEvent.change(screen.getByLabelText("Token Credential Ref"), {
      target: { value: "credential://official/token/imported-openai" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Import account" }));

    expect(importOfficialAccountLogin).toHaveBeenCalledWith({
      account_id: "imported-openai",
      name: "Imported OpenAI",
      provider: "openai",
      session_credential_ref: "credential://official/session/imported-openai",
      token_credential_ref: "credential://official/token/imported-openai",
      account_identity: null,
      auth_mode: null,
    });
    expect(await screen.findByText("Imported account: imported-openai")).toBeInTheDocument();
    expect(listAccounts).toHaveBeenCalled();
  });

  it("renders account import and relay creation as structured operations panels", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /accounts/i }));
    });
    expect(
      await screen.findByRole("heading", { name: /import official account/i }),
    ).toBeInTheDocument();

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
    expect(screen.getByText("ck_local_mocked_ops_key_secret")).toBeInTheDocument();
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
    fireEvent.change(screen.getByLabelText("Selection Order"), {
      target: { value: "official-primary, relay-newapi" },
    });
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
      selection_order: ["official-primary", "relay-newapi"],
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
    expect(screen.getByLabelText(/selection order/i)).toHaveValue(
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
  });

  it("renders logs as a request history with detail and capability panels", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /logs/i }));
    });

    expect(await screen.findByRole("heading", { name: /request history/i })).toBeInTheDocument();
    expect(screen.getByText(/usage provenance/i)).toBeInTheDocument();
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

    render(<App />);

    expect(await screen.findByText("Default key state | Current mode: relay_only")).toBeInTheDocument();
    expect(
      screen.getByText("no available endpoint for mode 'relay_only'"),
    ).toBeInTheDocument();
  });
});
