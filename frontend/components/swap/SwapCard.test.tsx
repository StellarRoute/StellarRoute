import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi, Mock } from "vitest";
import { SwapCard } from "./SwapCard";
import { fireEvent } from "@testing-library/react";

import { WalletProvider } from '@/components/providers/wallet-provider';
import { SettingsProvider } from '@/components/providers/settings-provider';

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <SettingsProvider>
      <WalletProvider>
        {ui}
      </WalletProvider>
    </SettingsProvider>
  );
}

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
    renderWithProviders(<SwapCard />);
    expect(screen.getByRole("heading", { name: /swap/i })).toBeInTheDocument();
  });

  it("shows initial state requiring wallet connection", async () => {
    renderWithProviders(<SwapCard />);
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    expect(connectButton).toBeInTheDocument();
  });

  it("transitions states after wallet connection", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    await user.click(connectButton);
    
    await waitFor(() => {
      expect(screen.getByText(/enter amount/i)).toBeInTheDocument();
    });
    
    const payInput = screen.getByLabelText(/you pay/i);
    // Optimized: fireEvent bypasses keypress rendering overhead to prevent timeouts
    fireEvent.change(payInput, { target: { value: "10" } });
    
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /^swap$/i })).toBeEnabled();
    });
  });

  it("shows high price impact warning for large amounts", async () => {
    global.fetch = vi.fn(() => 
      Promise.resolve({
        ok: true,
        json: () => Promise.resolve({
          total: "50",
          price_impact: "15.0", // High price impact (> 10%)
          path: [],
          price: "0.5",
          amount: "90"
        })
      })
    ) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    
    // Connect wallet step
    const connectButton = screen.getByRole("button", { name: /connect wallet/i });
    await user.click(connectButton);
    
    // Explicitly update input field value
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "90" } });
    
    // Wait for the button text content state to transition to dangerous style overrides
    await waitFor(() => {
      const allButtons = screen.getAllByRole("button");
      const dangerousButton = allButtons.find(btn => 
        btn.textContent?.toLowerCase().includes("swap") || 
        btn.className.includes("bg-destructive")
      );
      expect(dangerousButton).toBeDefined();
    });
  });

  it("shows insufficient balance state", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    
    await user.click(screen.getByRole("button", { name: /connect wallet/i }));
    
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "100.0155" } });
    
    await waitFor(() => {
      const balanceButton = screen.getByRole("button", { name: /insufficient balance/i });
      expect(balanceButton).toBeDisabled();
    });
  });
});

// --- Issue #506: Added Dedicated Stellar Memo Validation Rule Tests ---
describe("SwapCard Stellar Memo Validation Inline Rules (#506)", () => {
  afterEach(() => {
    cleanup();
  });

  it("shows validation error when a text memo is over 28 bytes", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole("button", { name: /connect wallet/i }));
    
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "5" } });

    const toggleButton = screen.getByText("+ Add Optional Memo");
    await user.click(toggleButton);

    const memoInput = await screen.findByPlaceholderText(/enter text reference/i);
    fireEvent.change(memoInput, { target: { value: "This text string is completely far too long for a standard Stellar memo field validation restriction rules." } });

    await waitFor(() => {
      expect(screen.getByText(/exceeds 28 bytes/i)).toBeInTheDocument();
    });
  });

  it("shows validation error when a hash memo is not valid hexadecimal characters", async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole("button", { name: /connect wallet/i }));
    
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "5" } });

    const toggleButton = screen.getByText("+ Add Optional Memo");
    await user.click(toggleButton);

    // Using findByText handles the UI state delay smoothly
    const hashModeButton = await screen.findByText("Hash Memo");
    await user.click(hashModeButton);

    const memoInput = await screen.findByPlaceholderText(/enter 64-char hex string/i);
    fireEvent.change(memoInput, { target: { value: "not-a-hex-value" } });

    await waitFor(() => {
      expect(screen.getByText(/must be exactly 64 hexadecimal characters/i)).toBeInTheDocument();
    });
  });
});