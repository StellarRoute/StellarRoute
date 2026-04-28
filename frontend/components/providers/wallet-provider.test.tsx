import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WalletProvider, useWallet } from '../wallet-provider';
import * as walletLib from '@/lib/wallet';

// Mock the wallet library
vi.mock('@/lib/wallet', () => ({
  getAvailableWallets: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  refreshWalletSession: vi.fn(),
}));

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
    networkMismatch,
    stubSpendableBalance,
    autoReconnectPreferred,
    connect,
    reconnect,
    disconnect,
    setAutoReconnectPreferred,
  } = useWallet();

  return (
    <div>
      <span data-testid="connected">{String(isConnected)}</span>
      <span data-testid="address">{address ?? "none"}</span>
      <span data-testid="network">{network}</span>
      <span data-testid="walletId">{walletId ?? "none"}</span>
      <span data-testid="error">{error?.message ?? "none"}</span>
      <span data-testid="loading">{String(isLoading)}</span>
      <span data-testid="mismatch">{String(networkMismatch)}</span>
      <span data-testid="balance">{stubSpendableBalance ?? "none"}</span>
      <span data-testid="autoReconnect">{String(autoReconnectPreferred)}</span>
      <button onClick={() => connect("freighter")}>Connect Freighter</button>
      <button onClick={reconnect}>Reconnect</button>
      <button onClick={disconnect}>Disconnect</button>
      <button onClick={() => setAutoReconnectPreferred(false)}>Disable auto reconnect</button>
      <button onClick={() => setAutoReconnectPreferred(true)}>Enable auto reconnect</button>
    </div>
  );
}

describe('WalletProvider Account Switching', () => {
  const mockAddress1 = 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  const mockAddress2 = 'GDEF456GHIJKLMNOPQRSTUVWXYZ789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456';

  beforeEach(() => {
    vi.clearAllMocks();
    mockWalletLib.getAvailableWallets.mockResolvedValue([
      { id: 'freighter', label: 'Freighter', installed: true }
    ]);
    mockWalletLib.disconnectWallet.mockReturnValue({
      walletId: null,
      address: null,
      network: null,
      isConnected: false,
    });
  });

  it('should prevent connection during pending transaction', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    render(
      <WalletProvider>
        <TestComponent />
      </WalletProvider>
    );

    // Start a transaction
    fireEvent.click(screen.getByText('Start Transaction'));
    expect(screen.getByTestId('transaction-pending')).toHaveTextContent('Pending');

    // Try to connect during transaction
    fireEvent.click(screen.getByText('Connect'));

    await waitFor(() => {
      expect(mockWalletLib.connectWallet).not.toHaveBeenCalled();
    });

    expect(screen.getByTestId('connected')).toHaveTextContent('Disconnected');
  });

  it('should prevent disconnection during pending transaction', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
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

    // Start a transaction
    fireEvent.click(screen.getByText('Start Transaction'));
    expect(screen.getByTestId('transaction-pending')).toHaveTextContent('Pending');

    // Try to disconnect during transaction
    fireEvent.click(screen.getByText('Disconnect'));

    // Should still be connected
    expect(screen.getByTestId('connected')).toHaveTextContent('Connected');
  });

  it('should prevent account refresh during pending transaction', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress2,
      network: 'testnet',
      isConnected: true,
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

    // Start a transaction
    fireEvent.click(screen.getByText('Start Transaction'));
    expect(screen.getByTestId('transaction-pending')).toHaveTextContent('Pending');

    // Try to refresh account during transaction
    fireEvent.click(screen.getByText('Refresh Account'));

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).not.toHaveBeenCalled();
    });
  });

  it('should successfully refresh account when no transaction is pending', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress2,
      network: 'testnet',
      isConnected: true,
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
      expect(screen.getByTestId('address')).toHaveTextContent(mockAddress1);
    });

    // Refresh account
    fireEvent.click(screen.getByText('Refresh Account'));

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).toHaveBeenCalledWith('freighter');
    });

    await waitFor(() => {
      expect(screen.getByTestId('address')).toHaveTextContent(mockAddress2);
    });
  });

  it('should handle refresh account errors gracefully', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    mockWalletLib.refreshWalletSession.mockRejectedValue(new Error('Refresh failed'));

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

    // Try to refresh account (should fail)
    fireEvent.click(screen.getByText('Refresh Account'));

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).toHaveBeenCalled();
    });

    // Should still be connected with original address
    expect(screen.getByTestId('connected')).toHaveTextContent('Connected');
    expect(screen.getByTestId('address')).toHaveTextContent(mockAddress1);
  });

  it('should reset account switch state after successful refresh', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress2,
      network: 'testnet',
      isConnected: true,
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

    // Refresh account
    fireEvent.click(screen.getByText('Refresh Account'));

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).toHaveBeenCalled();
    });

    // Account switch state should be reset (this would need to be exposed in the test component)
    // For now, we verify that the refresh completed successfully
    await waitFor(() => {
      expect(screen.getByTestId('address')).toHaveTextContent(mockAddress2);
    });
  });

  it('should allow normal operations after transaction ends', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
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

    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({
      address: "GABCDEFGHIJKLMNOPWXYZ",
    });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({
      address: "GABCDEFGHIJKLMNOPWXYZ",
    });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    renderWithProvider();

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });
    expect(screen.getByTestId("walletId").textContent).toBe("freighter");
  });

  it("does not auto reconnect on mount when preference is disabled", async () => {
    window.localStorage.setItem("stellarroute.wallet.autoReconnect", "false");
    window.localStorage.setItem("stellarroute.wallet.lastWalletId", "freighter");

    renderWithProvider();

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("false");
    });
    expect(freighter.requestAccess).not.toHaveBeenCalled();
  });

  it("recovers disconnected session when reconnect is triggered", async () => {
    window.localStorage.setItem("stellarroute.wallet.lastWalletId", "freighter");

    vi.mocked(freighter.requestAccess).mockResolvedValueOnce({
      address: "GABCDEFGHIJKLMNOPWXYZ",
    });
    vi.mocked(freighter.getAddress).mockResolvedValueOnce({
      address: "GABCDEFGHIJKLMNOPWXYZ",
    });
    vi.mocked(freighter.getNetworkDetails).mockResolvedValueOnce({
      network: "testnet",
      networkUrl: "",
      networkPassphrase: "",
    });

    const user = userEvent.setup();
    renderWithProvider();

    await user.click(screen.getByRole("button", { name: "Reconnect" }));

    await waitFor(() => {
      expect(screen.getByTestId("connected").textContent).toBe("true");
    });
    expect(screen.getByTestId("walletId").textContent).toBe("freighter");
  });
});
