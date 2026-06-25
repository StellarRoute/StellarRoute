import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { OrderbookDepthPanel } from "./OrderbookDepthPanel";
import type { Orderbook } from "@/types";

// Mock useOrderbook
vi.mock("@/hooks/useApi", () => ({
  useOrderbook: vi.fn(),
}));

import { useOrderbook } from "@/hooks/useApi";
const mockUseOrderbook = vi.mocked(useOrderbook);

const mockOrderbook: Orderbook = {
  base_asset: { asset_type: "native" },
  quote_asset: { asset_type: "credit_alphanum4", asset_code: "USDC", asset_issuer: "GA5Z" },
  bids: [
    { price: "0.10", amount: "100", total: "10.00" },
    { price: "0.09", amount: "200", total: "18.00" },
  ],
  asks: [
    { price: "0.11", amount: "150", total: "16.50" },
    { price: "0.12", amount: "50", total: "6.00" },
  ],
  timestamp: 1700000000,
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => cleanup());

describe("OrderbookDepthPanel", () => {
  it("renders loading skeletons while fetching", () => {
    mockUseOrderbook.mockReturnValue({
      data: undefined,
      loading: true,
      error: null,
      refresh: vi.fn(),
      midpoint: undefined,
      spreadBps: undefined,
    });
    const { container } = render(<OrderbookDepthPanel base="XLM" quote="USDC" maxRows={3} />);
    // Skeletons rendered (6 rows × 3 cells = 18 skeleton divs)
    expect(container.querySelectorAll("[data-slot='skeleton']").length).toBeGreaterThan(0);
  });

  it("renders bid and ask columns with correct data", () => {
    mockUseOrderbook.mockReturnValue({
      data: mockOrderbook,
      loading: false,
      error: null,
      refresh: vi.fn(),
      midpoint: "0.105",
      spreadBps: 95,
    });
    render(<OrderbookDepthPanel base="XLM" quote="USDC" />);
    expect(screen.getByText("Bids")).toBeInTheDocument();
    expect(screen.getByText("Asks")).toBeInTheDocument();
    expect(screen.getByText("0.10")).toBeInTheDocument();
    expect(screen.getByText("0.11")).toBeInTheDocument();
  });

  it("bids are sorted descending (highest price first)", () => {
    mockUseOrderbook.mockReturnValue({
      data: mockOrderbook,
      loading: false,
      error: null,
      refresh: vi.fn(),
      midpoint: "0.105",
      spreadBps: 95,
    });
    render(<OrderbookDepthPanel base="XLM" quote="USDC" />);
    const bidPrices = screen.getAllByText(/^0\.0[0-9]|^0\.1[0-9]/);
    // First bid price should be 0.10 (highest)
    expect(bidPrices[0].textContent).toBe("0.10");
  });

  it("shows error state on API failure", () => {
    mockUseOrderbook.mockReturnValue({
      data: undefined,
      loading: false,
      error: new Error("Network error"),
      refresh: vi.fn(),
      midpoint: undefined,
      spreadBps: undefined,
    });
    render(<OrderbookDepthPanel base="XLM" quote="USDC" />);
    expect(screen.getByText("Orderbook unavailable")).toBeInTheDocument();
    expect(screen.getByText("Network error")).toBeInTheDocument();
  });

  it("shows empty state when no bids/asks", () => {
    mockUseOrderbook.mockReturnValue({
      data: { ...mockOrderbook, bids: [], asks: [] },
      loading: false,
      error: null,
      refresh: vi.fn(),
      midpoint: undefined,
      spreadBps: undefined,
    });
    render(<OrderbookDepthPanel base="XLM" quote="USDC" />);
    expect(screen.getByText("No bids")).toBeInTheDocument();
    expect(screen.getByText("No asks")).toBeInTheDocument();
  });

  it("has accessible section label", () => {
    mockUseOrderbook.mockReturnValue({
      data: mockOrderbook,
      loading: false,
      error: null,
      refresh: vi.fn(),
      midpoint: "0.105",
      spreadBps: 95,
    });
    render(<OrderbookDepthPanel base="XLM" quote="USDC" />);
    expect(screen.getByRole("region", { name: /orderbook for XLM\/USDC/i })).toBeInTheDocument();
  });
});
