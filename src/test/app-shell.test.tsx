import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { getDefaultKeySummary } = vi.hoisted(() => ({
  getDefaultKeySummary: vi.fn(),
}));

vi.mock("../lib/tauri", () => ({
  getDefaultKeySummary,
}));

import App from "../App";

describe("App shell", () => {
  beforeEach(() => {
    getDefaultKeySummary.mockReset();
    getDefaultKeySummary.mockResolvedValue({
      name: "default",
      allowedMode: "hybrid",
      rawAllowedMode: "hybrid",
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

    fireEvent.click(screen.getByRole("button", { name: "Official Accounts" }));

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
});
