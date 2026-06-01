import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, act, waitFor, cleanup } from '@testing-library/react';
import { WalletProvider, useWallet } from '../wallet-provider';
import { WalletSyncBanner } from '../../shared/WalletSyncBanner';
import * as walletLib from '@/lib/wallet';

// Mock the wallet library
vi.mock('@/lib/wallet', () => ({
  getAvailableWallets: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
}));

const mockWalletLib = vi.mocked(walletLib);

// Helper component to trigger wallet states in test
function WalletController() {
  const { isConnected, address, connect, disconnect, setTransactionPending } = useWallet();

  return (
    <div>
      <span data-testid="connected">{String(isConnected)}</span>
      <span data-testid="address">{address ?? 'none'}</span>
      <button onClick={() => connect('freighter')}>Connect Freighter</button>
      <button onClick={disconnect}>Disconnect Wallet</button>
      <button onClick={() => setTransactionPending(true)}>Start Transaction</button>
      <button onClick={() => setTransactionPending(false)}>End Transaction</button>
    </div>
  );
}

function TestApp() {
  return (
    <WalletProvider>
      <WalletSyncBanner />
      <WalletController />
    </WalletProvider>
  );
}

describe('Multi-tab Wallet State Sync Detection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
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

  afterEach(() => {
    cleanup();
  });

  it('should show sync banner when wallet is connected in another tab', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      network: 'testnet',
      isConnected: true,
    });

    render(<TestApp />);

    // Starts disconnected
    expect(screen.queryByTestId('wallet-sync-banner')).not.toBeInTheDocument();

    // Mock cross-tab connection by writing to localStorage and firing storage event
    act(() => {
      window.localStorage.setItem('stellarroute.wallet.address', 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
      window.localStorage.setItem('stellarroute.wallet.walletId', 'freighter');
      
      const storageEvent = new StorageEvent('storage', {
        key: 'stellarroute.wallet.address',
        newValue: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      });
      window.dispatchEvent(storageEvent);
    });

    // Sync banner should be visible
    expect(screen.getByTestId('wallet-sync-banner')).toBeInTheDocument();
    expect(screen.getByTestId('wallet-sync-message')).toHaveTextContent(
      'Wallet change detected in another tab'
    );
    expect(screen.getByTestId('wallet-sync-message')).toHaveTextContent('GABC12...UVWXYZ');

    // Click Sync Wallet
    fireEvent.click(screen.getByTestId('wallet-sync-button'));

    await waitFor(() => {
      expect(mockWalletLib.connectWallet).toHaveBeenCalledWith('freighter');
    });

    await waitFor(() => {
      expect(screen.getByTestId('address')).toHaveTextContent(
        'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ'
      );
    });

    // Sync banner should disappear after successful sync
    expect(screen.queryByTestId('wallet-sync-banner')).not.toBeInTheDocument();
  });

  it('should show sync banner when wallet is disconnected in another tab', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      network: 'testnet',
      isConnected: true,
    });

    render(<TestApp />);

    // First connect locally
    fireEvent.click(screen.getByText('Connect Freighter'));
    await waitFor(() => {
      expect(screen.getByTestId('address')).toHaveTextContent('GABC123');
    });

    // Mock cross-tab disconnect by clearing localStorage and firing storage event
    act(() => {
      window.localStorage.removeItem('stellarroute.wallet.address');
      window.localStorage.removeItem('stellarroute.wallet.walletId');
      
      const storageEvent = new StorageEvent('storage', {
        key: 'stellarroute.wallet.address',
        newValue: null,
      });
      window.dispatchEvent(storageEvent);
    });

    // Sync banner should show disconnection message
    expect(screen.getByTestId('wallet-sync-banner')).toBeInTheDocument();
    expect(screen.getByTestId('wallet-sync-message')).toHaveTextContent(
      'Wallet disconnected in another tab'
    );

    // Click Sync Wallet (reconnect/disconnect action)
    fireEvent.click(screen.getByTestId('wallet-sync-button'));

    await waitFor(() => {
      expect(mockWalletLib.disconnectWallet).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(screen.getByTestId('connected')).toHaveTextContent('false');
      expect(screen.getByTestId('address')).toHaveTextContent('none');
    });

    // Sync banner should disappear
    expect(screen.queryByTestId('wallet-sync-banner')).not.toBeInTheDocument();
  });

  it('should allow dismissing the sync banner without changing connection state', async () => {
    render(<TestApp />);

    // Mock cross-tab connection
    act(() => {
      window.localStorage.setItem('stellarroute.wallet.address', 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
      window.localStorage.setItem('stellarroute.wallet.walletId', 'freighter');
      
      const storageEvent = new StorageEvent('storage', {
        key: 'stellarroute.wallet.address',
        newValue: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      });
      window.dispatchEvent(storageEvent);
    });

    expect(screen.getByTestId('wallet-sync-banner')).toBeInTheDocument();

    // Dismiss banner
    fireEvent.click(screen.getByTestId('wallet-dismiss-button'));

    // Banner is dismissed immediately
    expect(screen.queryByTestId('wallet-sync-banner')).not.toBeInTheDocument();
    
    // Core state is unchanged
    expect(screen.getByTestId('connected')).toHaveTextContent('false');
    expect(mockWalletLib.connectWallet).not.toHaveBeenCalled();
  });

  it('should prevent resync action and disable button during a pending transaction', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      network: 'testnet',
      isConnected: true,
    });

    render(<TestApp />);

    // Start transaction in active tab
    fireEvent.click(screen.getByText('Start Transaction'));

    // Mock cross-tab connection
    act(() => {
      window.localStorage.setItem('stellarroute.wallet.address', 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
      window.localStorage.setItem('stellarroute.wallet.walletId', 'freighter');
      
      const storageEvent = new StorageEvent('storage', {
        key: 'stellarroute.wallet.address',
        newValue: 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      });
      window.dispatchEvent(storageEvent);
    });

    expect(screen.getByTestId('wallet-sync-banner')).toBeInTheDocument();
    
    // Sync button should be disabled during transaction
    const syncButton = screen.getByTestId('wallet-sync-button');
    expect(syncButton).toBeDisabled();

    // Click it (should not proceed to call connectWallet)
    fireEvent.click(syncButton);
    expect(mockWalletLib.connectWallet).not.toHaveBeenCalled();

    // Finish transaction
    fireEvent.click(screen.getByText('End Transaction'));
    expect(syncButton).not.toBeDisabled();

    // Click it again, should now connect
    fireEvent.click(syncButton);
    await waitFor(() => {
      expect(mockWalletLib.connectWallet).toHaveBeenCalledWith('freighter');
    });
  });
});
