// frontend/components/shared/TradeActivityTable.test.tsx
import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { TradeActivityTable } from "./TradeActivityTable";
import { TradeRecord } from "../../types/trade";

const getSwapActivity = vi.fn();

vi.mock("../../hooks/useStellarRouteClient", () => ({
  useStellarRouteClient: () => ({
    getSwapActivity,
  }),
}));

const mockData: TradeRecord[] = [
  {
    id: "1",
    txHash: "123456789012345",
    timestamp: new Date(2026, 0, 1),
    action: "BUY",
    amount: "100",
    asset: "XLM",
  },
];

describe("TradeActivityTable component", () => {
  beforeEach(() => {
    getSwapActivity.mockReset();
  });

  it("should render offline initialData when address is missing", () => {
    render(<TradeActivityTable initialData={mockData} />);
    expect(screen.getAllByTestId("trade-row").length).toBe(1);
    expect(screen.getByText("BUY")).toBeDefined();
  });

  it("should render empty state after live fetch returns no swaps", async () => {
    getSwapActivity.mockResolvedValue({ swaps: [] });
    render(<TradeActivityTable address="GTESTADDRESS" initialData={[]} />);

    await waitFor(() => {
      expect(screen.getByTestId("empty-state")).toBeDefined();
    });
  });

  it("should render table rows from live swap activity", async () => {
    getSwapActivity.mockResolvedValue({
      swaps: [
        {
          event_id: "1",
          paging_token: "123456789012345",
          ledger_closed_at: "2026-01-01T00:00:00Z",
          sender: "GTESTADDRESS",
          amount_in: "100",
          source_asset: "XLM",
          destination_asset: "USDC",
        },
      ],
    });

    render(<TradeActivityTable address="GTESTADDRESS" />);

    await waitFor(() => {
      expect(screen.getAllByTestId("trade-row").length).toBe(1);
    });
    expect(screen.getByText("SWAP")).toBeDefined();
  });
});
