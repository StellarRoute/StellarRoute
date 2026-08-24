import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WalletProvider, useWallet } from "./wallet-provider";
import { WalletCapabilitiesBanner } from "@/components/shared/WalletCapabilitiesBanner";

vi.unmock('@/components/providers/wallet-provider');

// Mock the wallet library
vi.mock('@/lib/wallet', () => ({
  getAvailableWallets: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  refreshWalletSession: vi.fn(),
  checkWalletCapabilities: vi.fn(),
}));

const mockWalletLib = vi.mocked(walletLib);

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
});

afterEach(() => {
  cleanup();
});

// Test component to access wallet context
function TestComponent() {
  const {
    address,
    isConnected,
    network,
    walletId,
    error,
    isLoading,
    capabilities,
    networkMismatch,
    autoReconnectPreferred,
    connect,
    reconnect,
    disconnect,
    setAutoReconnectPreferred,
    isTransactionPending,
    setTransactionPending,
    refreshAccount,
    setNetwork,
    capabilities,
  } = useWallet();

  return (
    <div>
      <WalletCapabilitiesBanner />
      <span data-testid="connected">{String(isConnected)}</span>
      <span data-testid="address">{address ?? "none"}</span>
      <span data-testid="network">{network}</span>
      <span data-testid="walletId">{walletId ?? "none"}</span>
      <span data-testid="canSign">{String(capabilities?.canSign ?? false)}</span>
      <span data-testid="error">{error?.message ?? "none"}</span>
      <span data-testid="loading">{String(isLoading)}</span>
      <span data-testid="mismatch">{String(networkMismatch)}</span>
      <span data-testid="balance">{stubSpendableBalance ?? "none"}</span>
      <button onClick={() => connect("freighter")}>Connect Freighter</button>
      <button onClick={() => connect("xbull")}>Connect xBull</button>
      <button onClick={disconnect}>Disconnect</button>
      <button onClick={() => setAutoReconnectPreferred(false)}>Disable auto reconnect</button>
      <button onClick={() => setAutoReconnectPreferred(true)}>Enable auto reconnect</button>
      <button onClick={() => setTransactionPending(true)}>Start Transaction</button>
      <button onClick={() => setTransactionPending(false)}>End Transaction</button>
      <button onClick={refreshAccount}>Refresh Account</button>
      <button onClick={() => setNetwork('mainnet')}>Set Mainnet</button>
    </div>
  );
}

function renderWithProvider() {
  return render(
    <WalletProvider>
      <TestComponent />
    </WalletProvider>
  );
}

// ── Tests ──────────────────────────────────────────────────────────────────────
describe("WalletCapabilities & Provider", () => {
  it("provides disconnected state by default", () => {
    renderWithProvider();
    expect(screen.getByTestId("connected").textContent).toBe("false");
    expect(screen.getByTestId("address").textContent).toBe("none");
    expect(screen.getByTestId("network").textContent).toBe("testnet");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("connects Freighter with full capabilities", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    );

    // Connect first
    fireEvent.click(screen.getByText('Connect'));
    await waitFor(() => {
      expect(screen.getByTestId('connected')).toHaveTextContent('Connected');
    });

    // Start and end transaction
    fireEvent.click(screen.getByText('Start Transaction'));
    expect(screen.getByTestId('transaction-pending')).toHaveTextContent('Pending');

    fireEvent.click(screen.getByText('End Transaction'));
    expect(screen.getByTestId('transaction-pending')).toHaveTextContent('Not pending');

    // Should now be able to disconnect
    fireEvent.click(screen.getByText('Disconnect'));
    expect(screen.getByTestId('connected')).toHaveTextContent('Disconnected');
  });

  it("persists auto reconnect preference changes", async () => {
    const user = userEvent.setup();
    renderWithProvider();

    expect(screen.getByTestId("autoReconnect").textContent).toBe("true");

    await user.click(
      screen.getByRole("button", { name: "Disable auto reconnect" }),
    );
    expect(screen.getByTestId("autoReconnect").textContent).toBe("false");
    expect(
      window.localStorage.getItem("stellarroute.wallet.autoReconnect"),
    ).toBe("false");

    await user.click(
      screen.getByRole("button", { name: "Enable auto reconnect" }),
    );
    expect(screen.getByTestId("autoReconnect").textContent).toBe("true");
    expect(
      window.localStorage.getItem("stellarroute.wallet.autoReconnect"),
    ).toBe("true");
  });

  it("auto reconnects on mount when preference is enabled and a wallet was previously used", async () => {
    window.localStorage.setItem("stellarroute.wallet.autoReconnect", "true");
    window.localStorage.setItem("stellarroute.wallet.lastWalletId", "freighter");

    mockWalletLib.connectWallet.mockResolvedValueOnce({
      walletId: 'freighter',
      address: "GABCDEFGHIJKLMNOPWXYZ",
      network: 'testnet',
      isConnected: true,
    });

    renderWithProvider();

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("Connected");
    });
    expect(screen.getByTestId("canSign").textContent).toBe("true");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });

  it("warns about missing sign capability when wallet lacks signing support", async () => {
    // Mock window.xbull for connectWallet
    (window as unknown as Record<string, unknown>).xbull = {
      connect: vi.fn().mockResolvedValue({ publicKey: "GXBULLTESTADDRESS123" }),
    };

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole("button", { name: "Connect xBull" }));

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });
    expect(window.localStorage.getItem(NETWORK_STORAGE_KEY)).toBe('mainnet');
    delete process.env.NEXT_PUBLIC_MAINNET_LIMITED;
  });

    expect(screen.getByTestId("canSign").textContent).toBe("false");
    expect(screen.getByTestId("wallet-capabilities-banner")).toBeDefined();
    expect(screen.getByTestId("capability-warning").textContent).toContain(
      "Transaction signing is not supported"
    );
  });

  it("disconnects and clears state", async () => {
    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({ address: "GABCDEFGHIJKLMNOPWXYZ" });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole('button', { name: 'Connect Freighter' }));

    await user.click(screen.getByRole("button", { name: "Disconnect" }));

    expect(screen.getByTestId("connected").textContent).toBe("false");
    expect(screen.getByTestId("address").textContent).toBe("none");
    expect(screen.queryByTestId("wallet-capabilities-banner")).toBeNull();
  });
});
