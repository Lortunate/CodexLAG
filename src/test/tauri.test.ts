import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, listenMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

import {
  CodexLagInvokeError,
  createPlatformKey,
  getAccountCapabilityDetail,
  getRelayCapabilityDetail,
  listAccounts,
  getUsageRequestDetail,
  refreshAccountBalance,
  refreshRelayBalance,
} from "../lib/tauri";

async function expectInvokeError(operation: Promise<unknown>): Promise<CodexLagInvokeError> {
  try {
    await operation;
    throw new Error("Expected CodexLagInvokeError");
  } catch (error) {
    expect(error).toBeInstanceOf(CodexLagInvokeError);
    return error as CodexLagInvokeError;
  }
}

describe("tauri wrappers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  it("uses snake_case payload keys for id arguments", async () => {
    invokeMock.mockResolvedValue({});

    await refreshAccountBalance("acc-1");
    await getAccountCapabilityDetail("acc-2");
    await refreshRelayBalance("relay-1");
    await getUsageRequestDetail("req-7");
    await createPlatformKey({
      key_id: "key-new",
      name: "new key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
    });

    expect(invokeMock).toHaveBeenNthCalledWith(1, "refresh_account_balance", {
      account_id: "acc-1",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "get_account_capability_detail", {
      account_id: "acc-2",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "refresh_relay_balance", {
      relay_id: "relay-1",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "get_usage_request_detail", {
      request_id: "req-7",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(5, "create_platform_key", {
      input: {
        key_id: "key-new",
        name: "new key",
        policy_id: "default-policy",
        allowed_mode: "hybrid",
      },
    });
  });

  it("normalizes relay capability queryable adapter values from backend shape", async () => {
    invokeMock.mockResolvedValue({
      relay_id: "relay-newapi",
      endpoint: "http://127.0.0.1:8787",
      balance_capability: {
        queryable: {
          adapter: "new_api",
        },
      },
    });

    const detail = await getRelayCapabilityDetail("relay-newapi");

    expect(invokeMock).toHaveBeenCalledWith("get_relay_capability_detail", {
      relay_id: "relay-newapi",
    });
    expect(detail.balance_capability).toEqual({
      kind: "queryable",
      adapter: "new_api",
    });
  });

  it("parses structured object payloads from invoke failures", async () => {
    invokeMock.mockRejectedValue({
      code: "quota.provider_rate_limited",
      category: "QuotaError",
      message: "Rate limited by relay provider.",
      internal_context: "provider=relay;status=429",
    });

    const error = await expectInvokeError(listAccounts());

    expect(error.payload).toEqual({
      code: "quota.provider_rate_limited",
      category: "QuotaError",
      message: "Rate limited by relay provider.",
      internal_context: "provider=relay;status=429",
    });
  });

  it("parses nested error and payload wrappers from invoke failures", async () => {
    invokeMock.mockRejectedValue({
      error: {
        payload: {
          code: "routing.no_available_endpoint",
          category: "RoutingError",
          message: "No endpoint matched routing policy.",
          internal_context: "mode=relay_only",
        },
      },
    });

    const error = await expectInvokeError(refreshAccountBalance("acc-1"));

    expect(error.payload).toEqual({
      code: "routing.no_available_endpoint",
      category: "RoutingError",
      message: "No endpoint matched routing policy.",
      internal_context: "mode=relay_only",
    });
  });

  it("parses JSON-string payloads from invoke failures", async () => {
    invokeMock.mockRejectedValue(
      JSON.stringify({
        payload: {
          code: "credential.provider_auth_failed",
          category: "CredentialError",
          message: "Relay credentials were rejected.",
          internal_context: "provider=relay;status=401",
        },
      }),
    );

    const error = await expectInvokeError(listAccounts());

    expect(error.payload).toEqual({
      code: "credential.provider_auth_failed",
      category: "CredentialError",
      message: "Relay credentials were rejected.",
      internal_context: "provider=relay;status=401",
    });
  });

  it("uses legacy timeout fallback for timeout-like invoke failure strings", async () => {
    invokeMock.mockRejectedValue("Upstream request timed out while contacting relay.");

    const error = await expectInvokeError(listAccounts());

    expect(error.payload).toEqual({
      code: "upstream.provider_timeout",
      category: "UpstreamError",
      message: "Upstream request timed out while contacting relay.",
      internal_context: null,
    });
  });
});
