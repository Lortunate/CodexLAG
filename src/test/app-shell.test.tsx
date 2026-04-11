import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "../App";

describe("App shell", () => {
  it("renders the six primary navigation sections", () => {
    render(<App />);

    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Official Accounts")).toBeInTheDocument();
    expect(screen.getByText("Relays")).toBeInTheDocument();
    expect(screen.getByText("Platform Keys")).toBeInTheDocument();
    expect(screen.getByText("Policies")).toBeInTheDocument();
    expect(screen.getByText("Logs & Usage")).toBeInTheDocument();
  });
});
