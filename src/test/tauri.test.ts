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
  getAccountCapabilityDetail,
  getRelayCapabilityDetail,
  getUsageRequestDetail,
  refreshAccountBalance,
  refreshRelayBalance,
} from "../lib/tauri";

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
});
