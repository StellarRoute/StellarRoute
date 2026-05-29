import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SpreadIndicator } from "./SpreadIndicator";

describe("SpreadIndicator", () => {
  it("renders nothing when data is missing", () => {
    const { container } = render(<SpreadIndicator />);
    expect(container.firstChild).toBeNull();
  });

  it("renders loading state", () => {
    render(<SpreadIndicator isLoading={true} />);
    expect(screen.getByTestId("spread-indicator-loading")).toBeDefined();
  });

  it("formats spread percentage correctly", () => {
    render(<SpreadIndicator midpoint="1.0" spreadBps={15} />);
    expect(screen.getByText("0.15%")).toBeDefined();
  });

  it("applies green color for low spread", () => {
    render(<SpreadIndicator midpoint="1.0" spreadBps={5} />);
    const spreadValue = screen.getByText("0.05%");
    expect(spreadValue.className).toContain("text-emerald-500");
  });

  it("applies amber color for medium spread", () => {
    render(<SpreadIndicator midpoint="1.0" spreadBps={150} />);
    const spreadValue = screen.getByText("1.50%");
    expect(spreadValue.className).toContain("text-amber-500");
  });

  it("applies red color for critical spread", () => {
    render(<SpreadIndicator midpoint="1.0" spreadBps={500} />);
    const spreadValue = screen.getByText("5.00%");
    expect(spreadValue.className).toContain("text-destructive");
  });
});
