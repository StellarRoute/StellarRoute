import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { PriceHistorySparkline } from "./PriceHistorySparkline";

const POINTS = [
  { timestamp: Date.UTC(2024, 2, 25, 12, 0, 0), price: "0.1050000" },
  { timestamp: Date.UTC(2024, 2, 25, 13, 0, 0), price: "0.1054000" },
  { timestamp: Date.UTC(2024, 2, 25, 14, 0, 0), price: "0.1051000" },
];

describe("PriceHistorySparkline", () => {
  it("renders a graceful empty state when no points are available", () => {
    render(<PriceHistorySparkline points={[]} />);

    expect(
      screen.getByText(/no 24h price data available yet/i),
    ).toBeInTheDocument();
  });

  it("shows a tooltip with time and approximate price on hover", async () => {
    render(<PriceHistorySparkline points={POINTS} />);

    const pointButton = screen.getByRole("button", {
      name: /approx 0\.1054/i,
    });

    fireEvent.mouseEnter(pointButton);

    expect(screen.getByText(/approx 0\.1054/i)).toBeInTheDocument();
  });

  it("shows the latest approximate price in the header", () => {
    render(<PriceHistorySparkline points={POINTS} />);

    expect(screen.getByText(/0\.10510/i)).toBeInTheDocument();
  });
});
