import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SwapCard } from "./SwapCard";
import { SettingsProvider } from "@/components/providers/settings-provider";

  return {
    promise,
    resolve: () => resolve(createResponse(data)),
  };
}

function setVisibilityState(state: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => state,
  });
}

describe("SwapCard session recovery", () => {
  beforeEach(() => {
    localStorage.clear();
    setVisibilityState("visible");
  });

  afterEach(() => {
    cleanup();
    localStorage.clear();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("should render successfully", () => {
    render(<SettingsProvider><SwapCard /></SettingsProvider>);
    expect(screen.getByRole("heading", { name: /swap/i })).toBeInTheDocument();
  });

  it("shows initial state requiring wallet connection", async () => {
    render(<SettingsProvider><SwapCard /></SettingsProvider>);
    
    // Check for "Connect Wallet" button
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    expect(connectButton).toBeInTheDocument();
  });

  it("transitions states after wallet connection", async () => {
    const user = userEvent.setup();
    render(<SettingsProvider><SwapCard /></SettingsProvider>);
    
    // 1. Connect Wallet
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    await user.click(connectButton);
    
    // 2. Should show "Enter Amount"
    await waitFor(() => {
      expect(screen.getByText(/enter amount/i)).toBeInTheDocument();
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
    render(<SettingsProvider><SwapCard /></SettingsProvider>);
    
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
    render(<SettingsProvider><SwapCard /></SettingsProvider>);
    
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
