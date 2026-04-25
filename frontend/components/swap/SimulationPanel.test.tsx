import { render, screen } from "@testing-library/react";
import { cleanup } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { SimulationPanel } from "./SimulationPanel";

describe("SimulationPanel", () => {
  afterEach(() => cleanup());

  it("should show empty state when no amount is entered", () => {
    render(
      <SimulationPanel
        payAmount=""
        expectedOutput="0"
        slippage={0.5}
      />
    );

    expect(
      screen.getByText("Enter an amount to see trade simulation")
    ).toBeInTheDocument();
  });

  it("should respect stored locale when rendering translated swap copy", () => {
    window.localStorage.setItem(
      "stellar_route_settings",
      JSON.stringify({ locale: "zh-CN" })
    );

    render(
      <SimulationPanel
        payAmount=""
        expectedOutput="0"
        slippage={0.5}
      />
    );

    expect(screen.getByText("输入数量后即可查看交易模拟")).toBeInTheDocument();

    window.localStorage.clear();
  });

  it("should show loading state", () => {
    render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
        isLoading={true}
      />
    );

    // Check for loading skeleton elements
    const skeletonElements = document.querySelectorAll(".animate-pulse");
    expect(skeletonElements.length).toBeGreaterThan(0);
  });

  it("should show error state", () => {
    const { container } = render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
        error="Network error occurred"
      />
    );

    expect(screen.getByText("Simulation Error")).toBeInTheDocument();
    expect(screen.getByText("Network error occurred")).toBeInTheDocument();
    const errorPanel = container.querySelector(".border-destructive\\/30");
    expect(errorPanel).toHaveClass("bg-destructive/5", "text-destructive");
  });

  it("should calculate and display simulation data correctly", () => {
    render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
      />
    );

    expect(screen.getByText("Trade Simulation")).toBeInTheDocument();
    expect(screen.getByText("0.5% slippage")).toBeInTheDocument();

    // Check expected output
    expect(screen.getByText("98.000000")).toBeInTheDocument();

    // Check min received calculation (98 * (1 - 0.005) = 97.51)
    expect(screen.getByText("97.510000")).toBeInTheDocument();

    // Check effective rate (98 / 100 = 0.98)
    expect(screen.getByText("1 XLM ≈ 0.980000 USDC")).toBeInTheDocument();
  });

  it("should handle zero amount gracefully", () => {
    render(
      <SimulationPanel
        payAmount="0"
        expectedOutput="0"
        slippage={0.5}
      />
    );

    expect(
      screen.getByText("Enter an amount to see trade simulation")
    ).toBeInTheDocument();
  });

  it("should handle invalid amount gracefully", () => {
    render(
      <SimulationPanel
        payAmount="invalid"
        expectedOutput="98"
        slippage={0.5}
      />
    );

    expect(
      screen.getByText("Enter an amount to see trade simulation")
    ).toBeInTheDocument();
  });

  it("should show high price impact warning", () => {
    // Use a large amount to trigger high price impact
    render(
      <SimulationPanel
        payAmount="5000"
        expectedOutput="4900"
        slippage={0.5}
      />
    );

    expect(screen.getByText("High Impact")).toBeInTheDocument();
    expect(screen.getByText(/High Price Impact:/)).toBeInTheDocument();
    expect(
      screen.getByText(
        /This trade may significantly affect the market price/
      )
    ).toBeInTheDocument();
    expect(screen.getByText("High Impact")).toHaveClass("text-warning");
    expect(screen.getByText(/High Price Impact:/).closest(".rounded-lg")).toHaveClass(
      "bg-warning/10",
      "border-warning/30",
      "text-warning",
    );
  });

  it("uses theme token classes for key quote states", () => {
    render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
      />
    );

    expect(screen.getByText("97.510000")).toHaveClass("text-primary");
    expect(screen.getByText("0.005%")).toHaveClass("text-success");
  });

  it("should calculate slippage protection correctly", () => {
    render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={1.0} // 1% slippage
      />
    );

    // Slippage protection should be 98 - (98 * 0.99) = 0.98
    expect(screen.getByText("-0.980000 from slippage")).toBeInTheDocument();
  });

  it("should update calculations when slippage changes", () => {
    const { rerender } = render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
      />
    );

    // Initial calculation with 0.5% slippage
    expect(screen.getByText("97.510000")).toBeInTheDocument();
    expect(screen.getByText("0.5% slippage")).toBeInTheDocument();

    // Update slippage to 2%
    rerender(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={2.0}
      />
    );

    // Recalculated with 2% slippage: 98 * (1 - 0.02) = 96.04
    expect(screen.getByText("96.040000")).toBeInTheDocument();
    expect(screen.getByText("2% slippage")).toBeInTheDocument();
  });

  it("should update calculations when amount changes", () => {
    const { rerender } = render(
      <SimulationPanel
        payAmount="100"
        expectedOutput="98"
        slippage={0.5}
      />
    );

    // Initial calculation
    expect(screen.getByText("97.510000")).toBeInTheDocument();
    expect(screen.getByText("1 XLM ≈ 0.980000 USDC")).toBeInTheDocument();

    // Update amount
    rerender(
      <SimulationPanel
        payAmount="200"
        expectedOutput="196"
        slippage={0.5}
      />
    );

    // Updated calculation: 196 * (1 - 0.005) = 195.02
    expect(screen.getByText("195.020000")).toBeInTheDocument();
    // Effective rate should remain the same: 196 / 200 = 0.98
    expect(screen.getByText("1 XLM ≈ 0.980000 USDC")).toBeInTheDocument();
  });

  it("should handle very small amounts", () => {
    render(
      <SimulationPanel
        payAmount="0.001"
        expectedOutput="0.00098"
        slippage={0.5}
      />
    );

    expect(screen.getByText("0.000980")).toBeInTheDocument();
    expect(screen.getByText("0.000975")).toBeInTheDocument(); // Min received
  });

  it("should handle very large amounts", () => {
    render(
      <SimulationPanel
        payAmount="1000000"
        expectedOutput="980000"
        slippage={0.5}
      />
    );

    expect(screen.getByText("980000.000000")).toBeInTheDocument();
    expect(screen.getByText("975100.000000")).toBeInTheDocument(); // Min received
  });
});
