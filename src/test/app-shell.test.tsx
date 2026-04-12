import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  emitDefaultKeySummaryChanged,
  getAccountCapabilityDetail,
  getDefaultKeySummary,
  getLogSummary,
  getRuntimeLogMetadata,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPolicies,
  listRelays,
  listUsageRequestHistory,
  queryUsageLedger,
  refreshAccountBalance,
  refreshRelayBalance,
  setDefaultKeyMode,
} = vi.hoisted(() => {
  let listener:
    | ((summary: {
        name: string;
        allowedMode: "account_only" | "relay_only" | "hybrid" | null;
        rawAllowedMode: string;
      }) => void)
    | null = null;

  return {
    emitDefaultKeySummaryChanged(summary: {
      name: string;
      allowedMode: "account_only" | "relay_only" | "hybrid" | null;
      rawAllowedMode: string;
    }) {
      listener?.(summary);
    },
    getAccountCapabilityDetail: vi.fn(),
    getDefaultKeySummary: vi.fn(),
    getLogSummary: vi.fn(),
    getRuntimeLogMetadata: vi.fn(),
    getRelayCapabilityDetail: vi.fn(),
    getUsageRequestDetail: vi.fn(),
    listenForDefaultKeySummaryChanged: vi.fn(async (handler) => {
      listener = handler;
      return () => {
        listener = null;
      };
    }),
    listAccounts: vi.fn(),
    listPolicies: vi.fn(),
    listRelays: vi.fn(),
    listUsageRequestHistory: vi.fn(),
    queryUsageLedger: vi.fn(),
    refreshAccountBalance: vi.fn(),
    refreshRelayBalance: vi.fn(),
    setDefaultKeyMode: vi.fn(),
  };
});

vi.mock("../lib/tauri", () => ({
  getAccountCapabilityDetail,
  getDefaultKeySummary,
  getLogSummary,
  getRuntimeLogMetadata,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPolicies,
  listRelays,
  listUsageRequestHistory,
  queryUsageLedger,
  refreshAccountBalance,
  refreshRelayBalance,
  setDefaultKeyMode,
}));

import App from "../App";

describe("App shell", () => {
  beforeEach(() => {
    getAccountCapabilityDetail.mockReset();
    getDefaultKeySummary.mockReset();
    getLogSummary.mockReset();
    getRuntimeLogMetadata.mockReset();
    getRelayCapabilityDetail.mockReset();
    getUsageRequestDetail.mockReset();
    listenForDefaultKeySummaryChanged.mockClear();
    listAccounts.mockReset();
    listPolicies.mockReset();
    listRelays.mockReset();
    listUsageRequestHistory.mockReset();
    queryUsageLedger.mockReset();
    refreshAccountBalance.mockReset();
    refreshRelayBalance.mockReset();
    setDefaultKeyMode.mockReset();

    getDefaultKeySummary.mockResolvedValue({
      name: "default",
      allowedMode: "hybrid",
      rawAllowedMode: "hybrid",
    });
    getLogSummary.mockResolvedValue({
      level: "info",
      last_event: "Loopback gateway ready for key 'default' in hybrid mode",
    });
    getRuntimeLogMetadata.mockResolvedValue({
      log_dir: "/tmp/codexlag/logs",
      files: ["gateway.log", "gateway.1.log"],
    });
    listAccounts.mockResolvedValue([
      { account_id: "official-primary", name: "Primary Publisher", provider: "openai" },
    ]);
    refreshAccountBalance.mockResolvedValue({
      account_id: "official-primary",
      provider: "openai",
      refreshed_at: "1713370000",
      balance: {
        kind: "non_queryable",
        reason: "official accounts do not expose a balance endpoint",
      },
    });
    getAccountCapabilityDetail.mockResolvedValue({
      account_id: "official-primary",
      provider: "openai",
      refresh_capability: true,
      balance_capability: "non_queryable",
    });
    listPolicies.mockResolvedValue([{ name: "default", status: "active" }]);
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
        relay_id: "relay-nobalance",
        endpoint: "https://relay.example.test",
        balance: { kind: "unsupported", reason: "relay does not provide a balance endpoint" },
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
        relay_id: "relay-nobalance",
        endpoint: "https://relay.example.test",
        balance_capability: { kind: "unsupported" },
      };
    });
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
    });
  });

  it("renders overview status cards with balance and usage summaries", async () => {
    render(<App />);

    expect(await screen.findByText("Runtime status")).toBeInTheDocument();
    expect(screen.getByText("Balance observability")).toBeInTheDocument();
    expect(screen.getByText("Usage ledger")).toBeInTheDocument();
    expect(screen.getByText("Runtime diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Default key state | Current mode: hybrid")).toBeInTheDocument();
    expect(screen.getByText("Non-queryable accounts: 1")).toBeInTheDocument();
    expect(screen.getByText("Queryable relays: 1")).toBeInTheDocument();
    expect(screen.getByText("Account refresh failures: 0")).toBeInTheDocument();
    expect(screen.getByText("Relay refresh failures: 0")).toBeInTheDocument();
    expect(screen.getByText("Total ledger tokens: 222")).toBeInTheDocument();
    expect(screen.getByText("Usage cost provenance: unknown")).toBeInTheDocument();
    expect(screen.getByText("Log directory: /tmp/codexlag/logs")).toBeInTheDocument();
    expect(screen.getByText("Tracked log files: 2")).toBeInTheDocument();
    expect(queryUsageLedger).toHaveBeenCalledTimes(1);
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

  it("loads relay balances and capability details", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Relays" }));
    });

    expect(await screen.findByText("Local Gateway")).toBeInTheDocument();
    expect(screen.getByText("Balance state: queryable")).toBeInTheDocument();
    expect(screen.getByText("Adapter: new_api")).toBeInTheDocument();
    expect(screen.getByText("Total: 25.00")).toBeInTheDocument();
    expect(screen.getByText("Used: 7.50")).toBeInTheDocument();
    expect(screen.getByText("Upstream Proxy")).toBeInTheDocument();
    expect(screen.getByText("Balance state: unsupported")).toBeInTheDocument();
    expect(screen.getByText("Capability: unsupported")).toBeInTheDocument();
    expect(refreshRelayBalance).toHaveBeenCalledWith("relay-newapi");
    expect(getRelayCapabilityDetail).toHaveBeenCalledWith("relay-newapi");
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
      });
    });

    expect(
      await screen.findByText("Default key state | Current mode: account_only"),
    ).toBeInTheDocument();
  });
});
