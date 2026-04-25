import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi, Mock } from "vitest";
import { SwapCard } from "./SwapCard";

function setNavigatorOnline(value: boolean) {
  Object.defineProperty(window.navigator, "onLine", {
    configurable: true,
    value,
  });
}

describe("SwapCard network resilience and states", () => {
  beforeEach(() => {
    localStorage.clear();
    global.fetch = vi.fn(() => 
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          total: "9.5",
          price_impact: "0.5",
          path: [],
          price: "0.95",
          amount: "10"
        })
      })
    ) as Mock;
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("should render successfully", () => {
    render(<SwapCard />);
    expect(screen.getByRole("heading", { name: /swap/i })).toBeInTheDocument();
  });

  it("shows initial state requiring wallet connection", async () => {
    render(<SwapCard />);
    
    // Check for "Connect Wallet" button
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    expect(connectButton).toBeInTheDocument();
  });

  it("transitions states after wallet connection", async () => {
    const user = userEvent.setup();
    render(<SwapCard />);
    
    // 1. Connect Wallet
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    await user.click(connectButton);
    
    // 2. Should show "Enter Amount"
    await waitFor(() => {
      expect(screen.getByText(/enter amount/i)).toBeInTheDocument();
    });
    
    // 3. Enter amount
    const payInput = screen.getByLabelText(/you pay/i);
    await user.type(payInput, "10");
    
    // 4. Should show "Swap" button
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /^swap$/i })).toBeEnabled();
    }, { timeout: 3000 });
  });

  it("shows high price impact warning for large amounts", async () => {
    // Override fetch mock for this test to return high price impact
    global.fetch = vi.fn(() => 
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          total: "50",
          price_impact: "15.0", // > 10
          path: [],
          price: "0.5",
          amount: "90" // 90 is <= 100 mock balance, so insufficient_balance won't trigger
        })
      })
    ) as Mock;

    const user = userEvent.setup();
    render(<SwapCard />);
    
    // Connect
    await user.click(screen.getByRole("button", { name: /connect wallet/i }));
    
    // Enter amount
    const payInput = screen.getByLabelText(/you pay/i);
    await user.type(payInput, "90");
    
    await waitFor(() => {
      const impactButton = screen.getByRole("button", { name: /swap anyway/i });
      expect(impactButton).toBeEnabled();
      expect(impactButton).toHaveClass("bg-destructive");
    }, { timeout: 3000 });
  });

  it("shows insufficient balance state", async () => {
    const user = userEvent.setup();
    render(<SwapCard />);
    
    // Connect
    await user.click(screen.getByRole("button", { name: /connect wallet/i }));
    
    // Enter amount higher than mock balance (100.00)
    const payInput = screen.getByLabelText(/you pay/i);
    await user.type(payInput, "100.01");
    
    await waitFor(() => {
      const balanceButton = screen.getByRole("button", { name: /insufficient balance/i });
      expect(balanceButton).toBeDisabled();
    }, { timeout: 3000 });
  });
});
