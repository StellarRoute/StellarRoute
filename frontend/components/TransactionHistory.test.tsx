import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { TransactionRecord } from "@/types/transaction";

import { TransactionHistory } from "./TransactionHistory";

const historyState = vi.hoisted(() => ({
  transactions: [] as TransactionRecord[],
  clearHistory: vi.fn(),
}));

vi.mock("@/hooks/useTransactionHistory", () => ({
  useTransactionHistory: () => historyState,
}));

function createTransactions(count: number): TransactionRecord[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `tx-${index}`,
    timestamp: Date.now() - index * 60_000,
    fromAsset: index % 2 === 0 ? "XLM" : "USDC",
    fromAmount: `${index + 1}`,
    toAsset: index % 2 === 0 ? "USDC" : "XLM",
    toAmount: `${(index + 1) * 0.98}`,
    exchangeRate: "0.98",
    priceImpact: "0.01",
    minReceived: "0.97",
    networkFee: "0.001",
    routePath: [],
    status: "confirmed",
    hash: `hash-${index}`,
    walletAddress: "GBSU...XYZ9",
  }));
}

describe("TransactionHistory", () => {
  beforeEach(() => {
    historyState.transactions = [];
    historyState.clearHistory = vi.fn();
  });

  afterEach(() => cleanup());

  it("should show skeleton loader initially", async () => {
    const { container } = render(<TransactionHistory />);

    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("should replace skeleton with empty state after loading", async () => {
    const { container } = render(<TransactionHistory />);

    let skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);

    await waitFor(
      () => {
        const newSkeletons = container.querySelectorAll(".animate-pulse");
        expect(newSkeletons.length).toBe(0);
      },
      { timeout: 500 }
    );
  });

  it("should render correct header structure", () => {
    render(<TransactionHistory />);

    expect(screen.getByText("Transaction History")).toBeInTheDocument();
  });

  it("should maintain layout stability during loading to loaded transition", async () => {
    const { container } = render(<TransactionHistory />);

    const scrollArea = container.querySelector(".flex-1");
    const initialHeight = scrollArea?.clientHeight;

    await waitFor(
      () => {
        const skeletons = container.querySelectorAll(".animate-pulse");
        expect(skeletons.length).toBe(0);
      },
      { timeout: 500 }
    );

    const finalHeight = scrollArea?.clientHeight;

    if (initialHeight && finalHeight) {
      expect(Math.abs(initialHeight - finalHeight)).toBeLessThan(50);
    }
  });

  it("should not flicker on fast responses", async () => {
    const { container } = render(<TransactionHistory />);

    expect(container.querySelectorAll(".animate-pulse").length).toBeGreaterThan(0);

    await new Promise((resolve) => setTimeout(resolve, 350));

    expect(container.querySelectorAll(".animate-pulse").length).toBe(0);
  });

  it("virtualizes long activity lists and swaps the rendered window on scroll", async () => {
    historyState.transactions = createTransactions(120);

    render(<TransactionHistory />);

    await waitFor(
      () => {
        expect(screen.getByTestId("tx-row-tx-0")).toBeInTheDocument();
      },
      { timeout: 500 }
    );

    const initialRows = screen.getAllByTestId(/tx-row-/);
    expect(initialRows.length).toBeLessThan(historyState.transactions.length);

    const scrollContainer = screen.getByTestId("tx-history-scroll");
    scrollContainer.scrollTop = 4000;
    fireEvent.scroll(scrollContainer);

    await waitFor(() => {
      expect(screen.getByTestId("tx-row-tx-50")).toBeInTheDocument();
    });

    expect(screen.queryByTestId("tx-row-tx-0")).not.toBeInTheDocument();
  });
});
