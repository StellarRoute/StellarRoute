import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import PriceSparkline, { PricePoint, RangeDataMap } from "./PriceSparkline";

// ── helpers ──────────────────────────────────────────────────────────────────

function makePrices(n: number, base = 1.0): PricePoint[] {
  return Array.from({ length: n }, (_, i) => ({
    timestamp: Date.now() + i * 60_000,
    price: base + Math.sin(i / 5) * 0.1,
  }));
}

const FULL_DATA: RangeDataMap = {
  "1h": makePrices(12, 1.0),
  "24h": makePrices(24, 1.1),
  "7d": makePrices(48, 1.2),
};

// ── tests ────────────────────────────────────────────────────────────────────

describe("PriceSparkline – range selector", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  afterEach(() => {
    sessionStorage.clear();
  });

  it("renders three range buttons", () => {
    render(<PriceSparkline rangeData={FULL_DATA} />);
    expect(screen.getByRole("button", { name: /1h/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /24h/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /7d/i })).toBeInTheDocument();
  });

  it("defaults to 24h range", () => {
    render(<PriceSparkline rangeData={FULL_DATA} />);
    const btn = screen.getByRole("button", { name: /24h/i });
    expect(btn).toHaveAttribute("aria-pressed", "true");
  });

  it("switches range when a button is clicked", async () => {
    const user = userEvent.setup();
    render(<PriceSparkline rangeData={FULL_DATA} />);

    await user.click(screen.getByRole("button", { name: /1h/i }));
    expect(screen.getByRole("button", { name: /1h/i })).toHaveAttribute(
      "aria-pressed",
      "true"
    );
    expect(screen.getByRole("button", { name: /24h/i })).toHaveAttribute(
      "aria-pressed",
      "false"
    );
  });

  it("fires onRangeChange callback on switch", async () => {
    const user = userEvent.setup();
    const spy = vi.fn();
    render(<PriceSparkline rangeData={FULL_DATA} onRangeChange={spy} />);

    await user.click(screen.getByRole("button", { name: /7d/i }));
    expect(spy).toHaveBeenCalledWith("7d");
  });

  it("persists selection to sessionStorage", async () => {
    const user = userEvent.setup();
    render(<PriceSparkline rangeData={FULL_DATA} pairKey="XLM/USDC" />);

    await user.click(screen.getByRole("button", { name: /7d/i }));
    expect(sessionStorage.getItem("sparkline_range_XLM/USDC")).toBe("7d");
  });

  it("restores selection from sessionStorage on mount", () => {
    sessionStorage.setItem("sparkline_range_XLM/BTC", "1h");
    render(<PriceSparkline rangeData={FULL_DATA} pairKey="XLM/BTC" />);

    expect(screen.getByRole("button", { name: /1h/i })).toHaveAttribute(
      "aria-pressed",
      "true"
    );
  });
});

describe("PriceSparkline – loading states", () => {
  it("shows loading spinner inside the range button when that range is loading", () => {
    render(
      <PriceSparkline
        rangeData={{}}
        loadingRanges={new Set(["1h"])}
      />
    );
    // The 1h button should have aria-label including "loading"
    const btn = screen.getByRole("button", { name: /1h.*loading/i });
    expect(btn).toBeInTheDocument();
  });

  it("shows loading state in chart area when active range is loading", () => {
    render(
      <PriceSparkline
        rangeData={{}}
        loadingRanges={new Set(["24h"])}
      />
    );
    expect(screen.getByRole("status")).toBeInTheDocument();
    expect(screen.getByText(/loading price data/i)).toBeInTheDocument();
  });
});

describe("PriceSparkline – missing / empty data", () => {
  it("shows 'no price data' message when data array is empty for active range", () => {
    render(<PriceSparkline rangeData={{ "24h": [] }} />);
    expect(screen.getByText(/no price data available for 24h/i)).toBeInTheDocument();
  });

  it("shows 'no price data' when rangeData has no key for active range", () => {
    render(<PriceSparkline rangeData={{}} />);
    expect(screen.getByText(/no price data available for 24h/i)).toBeInTheDocument();
  });

  it("renders the SVG chart when data is present", () => {
    const { container } = render(<PriceSparkline rangeData={FULL_DATA} />);
    expect(container.querySelector("svg")).toBeInTheDocument();
  });
});
