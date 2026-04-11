import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  emitDefaultKeySummaryChanged,
  getDefaultKeySummary,
  getLogSummary,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPolicies,
  listRelays,
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
    getDefaultKeySummary: vi.fn(),
    getLogSummary: vi.fn(),
    listenForDefaultKeySummaryChanged: vi.fn(async (handler) => {
      listener = handler;
      return () => {
        listener = null;
      };
    }),
    listAccounts: vi.fn(),
    listPolicies: vi.fn(),
    listRelays: vi.fn(),
    setDefaultKeyMode: vi.fn(),
  };
});

vi.mock("../lib/tauri", () => ({
  getDefaultKeySummary,
  getLogSummary,
  listenForDefaultKeySummaryChanged,
  listAccounts,
  listPolicies,
  listRelays,
  setDefaultKeyMode,
}));

import App from "../App";

describe("App shell", () => {
  beforeEach(() => {
    getDefaultKeySummary.mockReset();
    getLogSummary.mockReset();
    listenForDefaultKeySummaryChanged.mockClear();
    listAccounts.mockReset();
    listPolicies.mockReset();
    listRelays.mockReset();
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
    listAccounts.mockResolvedValue([
      { name: "default-account", provider: "openai" },
    ]);
    listPolicies.mockResolvedValue([{ name: "default", status: "active" }]);
    listRelays.mockResolvedValue([
      { name: "primary-relay", endpoint: "https://relay.example.com" },
    ]);
    setDefaultKeyMode.mockResolvedValue({
      name: "default",
      allowedMode: "relay_only",
      rawAllowedMode: "relay_only",
    });
  });

  it("renders the six primary navigation sections and defaults to the overview page", async () => {
    render(<App />);

    expect(screen.getByRole("button", { name: "Overview" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Official Accounts" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Relays" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Platform Keys" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Policies" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Logs & Usage" }),
    ).toBeInTheDocument();

    expect(await screen.findByText("Default Key Mode")).toBeInTheDocument();
    expect(screen.getByText("Gateway Overview")).toBeInTheDocument();
    expect(screen.queryByText("Account Details")).not.toBeInTheDocument();
  });

  it("switches the active page from the sidebar", async () => {
    render(<App />);
    await screen.findByText("Default Key Mode");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });

    expect(screen.getByText("Account Details")).toBeInTheDocument();
    expect(screen.queryByText("Gateway Overview")).not.toBeInTheDocument();
    expect(screen.queryByText("Default Key Mode")).not.toBeInTheDocument();
  });

  it("loads the default key summary through the Tauri wrapper", async () => {
    render(<App />);

    expect(await screen.findByText("Default key: default")).toBeInTheDocument();
    expect(screen.getByText("Allowed mode: hybrid")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "hybrid" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(getDefaultKeySummary).toHaveBeenCalledTimes(1);
  });

  it("updates the default key mode through the Tauri wrapper", async () => {
    render(<App />);

    await screen.findByText("Allowed mode: hybrid");
    fireEvent.click(screen.getByRole("button", { name: "relay_only" }));

    expect(setDefaultKeyMode).toHaveBeenCalledWith("relay_only");
    expect(await screen.findByText("Allowed mode: relay_only")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "relay_only" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "hybrid" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("does not invoke mode updates for the already-active mode", async () => {
    render(<App />);

    await screen.findByText("Allowed mode: hybrid");
    fireEvent.click(screen.getByRole("button", { name: "hybrid" }));

    expect(setDefaultKeyMode).not.toHaveBeenCalled();
  });

  it("refreshes the overview when the backend emits a default key summary change", async () => {
    render(<App />);

    await screen.findByText("Allowed mode: hybrid");

    await act(async () => {
      emitDefaultKeySummaryChanged({
        name: "default",
        allowedMode: "account_only",
        rawAllowedMode: "account_only",
      });
    });

    expect(
      await screen.findByText("Allowed mode: account_only"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "account_only" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("shows an error message when mode update fails", async () => {
    setDefaultKeyMode.mockRejectedValueOnce(new Error("update failed"));

    render(<App />);

    await screen.findByText("Allowed mode: hybrid");
    fireEvent.click(screen.getByRole("button", { name: "relay_only" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Failed to update default key mode.",
    );
  });

  it("loads data-backed detail pages through the Tauri wrappers", async () => {
    render(<App />);

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));
    });
    expect(await screen.findByText("default-account")).toBeInTheDocument();
    expect(screen.getByText("openai")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Relays" }));
    });
    expect(await screen.findByText("primary-relay")).toBeInTheDocument();
    expect(screen.getByText("https://relay.example.com")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Policies" }));
    });
    expect(await screen.findByText("default")).toBeInTheDocument();
    expect(screen.getByText("active")).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Logs & Usage" }));
    });
    expect(await screen.findByText("info")).toBeInTheDocument();
    expect(
      screen.getByText("Loopback gateway ready for key 'default' in hybrid mode"),
    ).toBeInTheDocument();

    expect(listAccounts).toHaveBeenCalledTimes(1);
    expect(listRelays).toHaveBeenCalledTimes(1);
    expect(listPolicies).toHaveBeenCalledTimes(1);
    expect(getLogSummary).toHaveBeenCalledTimes(1);
  });
});
