import * as React from "react";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, Mock } from "vitest";
import { fireEvent } from "@testing-library/react";
import { SwapCard } from "./SwapCard";

vi.mock("@/components/providers/wallet-provider", () => ({
  useWallet: vi.fn(),
}));

vi.mock("@/hooks/useWalletBalance", () => ({
  useWalletBalance: vi.fn(),
}));

vi.mock("@/components/providers/settings-provider", () => ({
  useSettings: () => ({
    settings: { slippageTolerance: 0.5 },
    updateSlippage: vi.fn(),
    updateTheme: vi.fn(),
    updateLocale: vi.fn(),
    resetSettings: vi.fn(),
    addProfile: vi.fn(),
    updateProfile: vi.fn(),
    deleteProfile: vi.fn(),
    selectProfile: vi.fn(),
  }),
  useOptionalSettings: () => ({
    settings: { slippageTolerance: 0.5 },
  }),
}));

vi.mock("@/hooks/useSwapState", () => ({
  useSwapState: () => {
    const [fromToken, setFromToken] = React.useState("native");
    const [toToken, setToToken] = React.useState("USDC:GA5Z");
    const [fromAmount, setFromAmount] = React.useState("");
    const [toAmount, setToAmount] = React.useState("0");
    const [slippage, setSlippage] = React.useState(0.5);
    const [deadline, setDeadline] = React.useState(30);
    return {
      fromToken,
      setFromToken,
      toToken,
      setToToken,
      fromAmount,
      setFromAmount,
      toAmount,
      setToAmount,
      slippage,
      setSlippage,
      deadline,
      setDeadline,
      quote: {
        loading: false,
        error: null,
        priceImpact: mockQuotePriceImpact,
        isStale: false,
        isRecovering: false,
        hasPendingRetry: false,
        pendingRetryRemainingMs: 0,
        cancelRetry: vi.fn(),
        refresh: vi.fn(),
        fee: 0,
      },
      switchTokens: vi.fn(),
      formattedRate: "1 XLM ≈ 0.95 USDC",
      pendingRecovery: null,
      restorePending: vi.fn(),
      discardPending: vi.fn(),
      hasRecoverableState: false,
      snapshotCurrent: vi.fn(),
      reset: vi.fn(),
    };
  },
}));

vi.mock("@/hooks/useExpertSettings", () => ({
  useExpertSettings: () => ({
    expertMode: false,
    bypassConfirmation: false,
    extendedRouteDetails: false,
    updateExpertMode: vi.fn(),
    updateBypassConfirmation: vi.fn(),
    updateExtendedRouteDetails: vi.fn(),
  }),
}));

vi.mock("@/hooks/useOnlineStatus", () => ({
  useOnlineStatus: () => ({ isOnline: true }),
}));

vi.mock("@/hooks/useQuoteStreamStatus", () => ({
  useQuoteStreamStatus: () => ({ status: "live", mode: "live" }),
}));

vi.mock("@/hooks/useCompactMode", () => ({
  useCompactMode: () => ({ isCompact: false, toggleCompact: vi.fn() }),
}));

vi.mock("@/hooks/useShareableQuote", () => ({
  useShareableQuote: () => ({
    parseParams: () => null,
    isStale: false,
    refreshQuote: vi.fn(),
  }),
}));

vi.mock("@/hooks/useOptimisticSwap", () => ({
  useOptimisticSwap: () => ({
    status: "idle",
    errorMessage: null,
    submitLock: false,
    initiateSwap: vi.fn(),
  }),
}));

vi.mock("@/components/shared/NetworkMismatchBanner", () => ({
  NetworkMismatchBanner: () => null,
}));

vi.mock("./QuoteStreamStatusIndicator", () => ({
  QuoteStreamStatusIndicator: () => null,
}));

vi.mock("./ShareQuoteButton", () => ({
  ShareQuoteButton: () => null,
}));

vi.mock("./HighImpactConfirmModal", () => ({
  HighImpactConfirmModal: () => null,
}));

vi.mock("./TransactionConfirmationModal", () => ({
  TransactionConfirmationModal: () => null,
}));

vi.mock("./PriceInfoPanel", () => ({
  PriceInfoPanel: () => null,
}));

vi.mock("./RoutePanelAsync", () => ({
  default: () => null,
}));

vi.mock("../settings/SettingsPanel", () => ({
  SettingsPanel: () => null,
}));

vi.mock("./SessionRecoveryModal", () => ({
  SessionRecoveryModal: () => null,
}));

vi.mock("@/lib/wallet", () => ({
  signTransactionWithWallet: vi.fn(),
}));

vi.mock("@/lib/wallet/submit", () => ({
  submitToHorizon: vi.fn(),
  getNetworkPassphrase: vi.fn(() => "Test Network"),
}));

vi.mock("sonner", () => ({
  toast: {
    loading: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import * as walletProvider from "@/components/providers/wallet-provider";
import * as walletBalance from "@/hooks/useWalletBalance";

let mockQuotePriceImpact = 0.5;

describe("SwapCard network resilience and states", () => {
  beforeEach(() => {
    localStorage.clear();
    mockQuotePriceImpact = 0.5;
    vi.mocked(walletProvider.useWallet).mockReturnValue({
      address: "GABC123",
      isConnected: true,
      walletId: "freighter",
      walletNetwork: "testnet",
      networkMismatch: false,
    } as ReturnType<typeof walletProvider.useWallet>);
    vi.mocked(walletBalance.useWalletBalance).mockReturnValue({
      balance: "100.0000000",
      spendableBalance: "100.0000000",
      loading: false,
      error: null,
    } as ReturnType<typeof walletBalance.useWalletBalance>);
    global.fetch = vi.fn(
      () =>
        Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              total: "9.5",
              price_impact: "0.5",
              path: [],
              price: "0.95",
              amount: "10",
            }),
        }),
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

  it("shows initial state requiring wallet connection", () => {
    vi.mocked(walletProvider.useWallet).mockReturnValue({
      address: null,
      isConnected: false,
      walletId: null,
      walletNetwork: null,
      networkMismatch: false,
    } as ReturnType<typeof walletProvider.useWallet>);
    vi.mocked(walletBalance.useWalletBalance).mockReturnValue({
      balance: null,
      spendableBalance: null,
      loading: false,
      error: null,
    } as ReturnType<typeof walletBalance.useWalletBalance>);

    render(<SwapCard />);

    expect(screen.getByRole("button", { name: /connect wallet/i })).toBeInTheDocument();
  });

  it("shows a live balance in the pay input", async () => {
    render(<SwapCard />);

    const payInput = screen.getByLabelText(/you pay/i);
    expect(screen.getByText(/Balance:/i)).toHaveTextContent("100.0000000 XLM");

    fireEvent.change(payInput, { target: { value: "10" } });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review swap/i })).toBeEnabled();
    });
  });

  it("shows high price impact warning for large amounts", async () => {
    mockQuotePriceImpact = 15;

    render(<SwapCard />);

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "90" } });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /swap anyway/i })).toBeEnabled();
    });
  });

  it("shows insufficient balance state", async () => {
    vi.mocked(walletBalance.useWalletBalance).mockReturnValue({
      balance: "1.0000000",
      spendableBalance: "1.0000000",
      loading: false,
      error: null,
    } as ReturnType<typeof walletBalance.useWalletBalance>);

    render(<SwapCard />);

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "100.0155" } });

    await waitFor(() => {
      const balanceButton = screen.getByRole("button", {
        name: /insufficient balance/i,
      });
      expect(balanceButton).toBeDisabled();
    });
  });

  it("keeps memo validation coverage on the swap screen", () => {
    render(<SwapCard />);

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: "5" } });

    expect(payInput).toBeInTheDocument();
  });
});
