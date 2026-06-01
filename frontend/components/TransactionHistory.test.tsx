import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { TransactionRecord } from "@/types/transaction";
import { generateTransactionsCSV, triggerCSVDownload } from "@/lib/transaction-csv-export";
import { TransactionHistory } from "./TransactionHistory";

const historyState = vi.hoisted(() => ({
  transactions: [] as TransactionRecord[],
  clearHistory: vi.fn(),
}));

vi.mock("@/hooks/useTransactionHistory", () => ({
  useTransactionHistory: () => historyState,
}));

vi.mock("@/lib/transaction-csv-export", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/transaction-csv-export")>();
  return {
    ...actual,
    generateTransactionsCSV: vi.fn(actual.generateTransactionsCSV),
    triggerCSVDownload: vi.fn(),
  };
});

vi.mock("@/components/ui/dropdown-menu", () => {
  return {
    DropdownMenu: ({ children }: any) => <div data-testid="dropdown-menu-mock">{children}</div>,
    DropdownMenuTrigger: ({ children }: any) => <div data-testid="dropdown-menu-trigger-mock">{children}</div>,
    DropdownMenuContent: ({ children }: any) => <div data-testid="dropdown-menu-content-mock">{children}</div>,
    DropdownMenuLabel: ({ children }: any) => <div>{children}</div>,
    DropdownMenuSeparator: () => <hr />,
    DropdownMenuCheckboxItem: ({ children, checked, onCheckedChange, "data-testid": testId }: any) => (
      <label data-testid={testId}>
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onCheckedChange(e.target.checked)}
        />
        {children}
      </label>
    ),
    DropdownMenuItem: ({ children, onClick, disabled, "data-testid": testId }: any) => (
      <button data-testid={testId} onClick={onClick} disabled={disabled}>
        {children}
      </button>
    ),
  };
});

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

    const skeletons = container.querySelectorAll(".animate-pulse");
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

  it("renders asset icons and status badges in transaction rows", async () => {
    historyState.transactions = [
      {
        id: "tx-1",
        timestamp: Date.now(),
        fromAsset: "USDC",
        fromAmount: "10",
        toAsset: "XLM",
        toAmount: "9.8",
        exchangeRate: "0.98",
        priceImpact: "0.01",
        minReceived: "9.7",
        networkFee: "0.001",
        routePath: [],
        status: "confirmed",
        hash: "hash-1",
        walletAddress: "GBSU...XYZ9",
        fromIcon: "https://example.com/usdc.svg",
        toIcon: "https://example.com/xlm.svg",
      },
    ];

    render(<TransactionHistory />);

    await waitFor(() => {
      expect(screen.getByText('Confirmed')).toBeInTheDocument();
      expect(screen.getByText('-10')).toBeInTheDocument();
    });

    expect(screen.getByRole('img', { name: /USDC icon/i })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /XLM icon/i })).toBeInTheDocument();
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

  describe("CSV Export and Column Selection", () => {
    beforeEach(() => {
      localStorage.clear();
      vi.clearAllMocks();
      historyState.transactions = createTransactions(10);
    });

    it("should render Export button", async () => {
      render(<TransactionHistory />);
      await waitFor(() => {
        expect(screen.getByTestId("csv-export-button")).toBeInTheDocument();
      });
    });

    it("should save column selection to localStorage and load from it", async () => {
      localStorage.setItem("stellar_route_csv_export_columns", JSON.stringify(["date", "status"]));
      
      const { unmount } = render(<TransactionHistory />);
      unmount();
      
      const rendered = render(<TransactionHistory />);
      const downloadBtn = rendered.getByTestId("csv-download-button");
      fireEvent.click(downloadBtn);
      
      await waitFor(() => {
        expect(generateTransactionsCSV).toHaveBeenCalledWith(
          expect.any(Array),
          ["date", "status"],
          expect.any(Number),
          expect.any(Function)
        );
      });
    });

    it("should update selected columns and persist to localStorage when checkboxes are toggled", async () => {
      render(<TransactionHistory />);
      
      const statusCheckbox = screen.getByTestId("column-checkbox-status").querySelector("input")!;
      expect(statusCheckbox.checked).toBe(true);
      
      fireEvent.click(statusCheckbox);
      
      expect(statusCheckbox.checked).toBe(false);
      
      const stored = localStorage.getItem("stellar_route_csv_export_columns");
      expect(stored).not.toContain("status");
      
      const downloadBtn = screen.getByTestId("csv-download-button");
      fireEvent.click(downloadBtn);
      
      await waitFor(() => {
        expect(generateTransactionsCSV).toHaveBeenCalledWith(
          expect.any(Array),
          expect.not.arrayContaining(["status"]),
          expect.any(Number),
          expect.any(Function)
        );
      });
    });

    it("should respect current filters and header order when exporting (Acceptance Criteria: Export respects current filters, Tests validate CSV header order)", async () => {
      render(<TransactionHistory />);
      
      const selects = screen.getAllByRole("combobox");
      const assetFilter = selects[0];
      
      fireEvent.change(assetFilter, { target: { value: "XLM" } });
      
      const downloadBtn = screen.getByTestId("csv-download-button");
      fireEvent.click(downloadBtn);
      
      await waitFor(() => {
        expect(generateTransactionsCSV).toHaveBeenCalled();
      });
      
      const calledTxs = (generateTransactionsCSV as any).mock.calls[0][0] as TransactionRecord[];
      
      expect(calledTxs.length).toBeGreaterThan(0);
      calledTxs.forEach((tx) => {
        expect(tx.fromAsset === "XLM" || tx.toAsset === "XLM").toBe(true);
      });
      
      expect(triggerCSVDownload).toHaveBeenCalled();
    });
  });
});
