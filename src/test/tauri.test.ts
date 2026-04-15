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
  getProviderDiagnostics,
  getRelayCapabilityDetail,
  listAccounts,
  listProviderSessions,
  listPolicies,
  getUsageRequestDetail,
  logoutOpenAiSession,
  refreshAccountBalance,
  refreshOpenAiSession,
  refreshRelayBalance,
  startOpenAiBrowserLogin,
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
    await getProviderDiagnostics();
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
    expect(invokeMock).toHaveBeenNthCalledWith(5, "get_provider_diagnostics");
    expect(invokeMock).toHaveBeenNthCalledWith(6, "create_platform_key", {
      input: {
        key_id: "key-new",
        name: "new key",
        policy_id: "default-policy",
        allowed_mode: "hybrid",
      },
    });
  });

  it("starts OpenAI browser login through the dedicated command surface", async () => {
    invokeMock.mockResolvedValue({
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

    const pending = await startOpenAiBrowserLogin();

    expect(invokeMock).toHaveBeenCalledWith("start_openai_browser_login");
    expect(pending.summary.provider_id).toBe("openai_official");
    expect(pending.callback_url).toContain("127.0.0.1");
  });

  it("lists provider sessions through the dedicated command surface", async () => {
    invokeMock.mockResolvedValue([
      {
        provider_id: "openai_official",
        account_id: "openai-primary",
        display_name: "OpenAI Primary",
        auth_state: "active",
        expires_at_ms: 1_731_111_111_000,
        last_refresh_at_ms: 1_731_111_000_500,
        last_refresh_error: null,
      },
    ]);

    const sessions = await listProviderSessions();

    expect(invokeMock).toHaveBeenCalledWith("list_provider_sessions");
    expect(sessions[0]?.account_id).toBe("openai-primary");
  });

  it("refreshes and logs out an OpenAI provider session through the dedicated command surface", async () => {
    invokeMock
      .mockResolvedValueOnce({
        provider_id: "openai_official",
        account_id: "openai-primary",
        display_name: "OpenAI Primary",
        auth_state: "active",
        expires_at_ms: 1_731_111_222_000,
        last_refresh_at_ms: 1_731_111_111_000,
        last_refresh_error: null,
      })
      .mockResolvedValueOnce(true);

    const refreshed = await refreshOpenAiSession("openai-primary");
    const loggedOut = await logoutOpenAiSession("openai-primary");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "refresh_openai_session", {
      account_id: "openai-primary",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "logout_openai_session", {
      account_id: "openai-primary",
    });
    expect(refreshed.last_refresh_at_ms).toBe(1_731_111_111_000);
    expect(loggedOut).toBe(true);
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

  it("preserves generated secrets and hydrated policy fields from backend contracts", async () => {
    invokeMock
      .mockResolvedValueOnce({
        id: "ops-key",
        name: "Operations Key",
        policy_id: "default-policy",
        allowed_mode: "hybrid",
        enabled: true,
        secret: "ck_local_mocked_ops_key_secret",
      })
      .mockResolvedValueOnce([
        {
          policy_id: "default-policy",
          name: "default",
          status: "active",
          selection_order: ["official-primary", "relay-newapi"],
          cross_pool_fallback: false,
          retry_budget: 2,
          timeout_open_after: 3,
          server_error_open_after: 4,
          cooldown_ms: 1000,
          half_open_after_ms: 1000,
          success_close_after: 2,
        },
      ]);

    const created = await createPlatformKey({
      key_id: "ops-key",
      name: "Operations Key",
      policy_id: "default-policy",
      allowed_mode: "hybrid",
    });
    const policies = await listPolicies();

    expect(created.secret).toBe("ck_local_mocked_ops_key_secret");
    expect(policies).toEqual([
      {
        policy_id: "default-policy",
        name: "default",
        status: "active",
        selection_order: ["official-primary", "relay-newapi"],
        cross_pool_fallback: false,
        retry_budget: 2,
        timeout_open_after: 3,
        server_error_open_after: 4,
        cooldown_ms: 1000,
        half_open_after_ms: 1000,
        success_close_after: 2,
      },
    ]);
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
