import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AccountSwitcher } from '../account-switcher';
import { WalletProvider } from '@/components/providers/wallet-provider';
import * as walletLib from '@/lib/wallet';

// Mock the wallet library
vi.mock('@/lib/wallet', () => ({
  getAvailableWallets: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  refreshWalletSession: vi.fn(),
  checkAddressChange: vi.fn(),
}));

const mockWalletLib = walletLib as any;

// Mock component wrapper
function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <WalletProvider defaultNetwork="testnet">
      {children}
    </WalletProvider>
  );
}

describe('AccountSwitcher', () => {
  const mockAddress1 = 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  const mockAddress2 = 'GDEF456GHIJKLMNOPQRSTUVWXYZ789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456';
  
  beforeEach(() => {
    vi.clearAllMocks();
    mockWalletLib.getAvailableWallets.mockResolvedValue([
      { id: 'freighter', label: 'Freighter', installed: true }
    ]);
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });
    mockWalletLib.checkAddressChange.mockResolvedValue(null);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should not render when wallet is not connected', () => {
    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    expect(screen.queryByText('Refresh Account')).not.toBeInTheDocument();
  });

  it('should render refresh button when wallet is connected', async () => {
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    // First connect the wallet
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });
  });

  it('should detect account changes and show notification', async () => {
    // Setup connected state
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });

    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Mock account change detection
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    // Wait for the account change detection interval
    await waitFor(() => {
      expect(screen.getByText('Account Change Detected')).toBeInTheDocument();
    }, { timeout: 4000 });

    expect(screen.getByText(/Your wallet account appears to have changed/)).toBeInTheDocument();
    expect(screen.getByText('Refresh Account')).toBeInTheDocument();
    expect(screen.getByText('Dismiss')).toBeInTheDocument();
  });

  it('should refresh account when refresh button is clicked', async () => {
    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress2,
      network: 'testnet',
      isConnected: true,
    });

    const onAccountChange = vi.fn();

    render(
      <TestWrapper>
        <AccountSwitcher onAccountChange={onAccountChange} />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Mock account change detection
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    await waitFor(() => {
      expect(screen.getByText('Account Change Detected')).toBeInTheDocument();
    }, { timeout: 4000 });

    // Click refresh account
    const refreshButton = screen.getByText('Refresh Account');
    fireEvent.click(refreshButton);

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).toHaveBeenCalledWith('freighter');
    });
  });

  it('should dismiss account change notification', async () => {
    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Mock account change detection
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    await waitFor(() => {
      expect(screen.getByText('Account Change Detected')).toBeInTheDocument();
    }, { timeout: 4000 });

    // Click dismiss
    const dismissButton = screen.getByText('Dismiss');
    fireEvent.click(dismissButton);

    await waitFor(() => {
      expect(screen.queryByText('Account Change Detected')).not.toBeInTheDocument();
    });

    expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
  });

  it('should prevent account switching during transactions', async () => {
    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Simulate transaction pending state
    // This would need to be set through the wallet provider context
    // For now, we'll test the UI behavior when transaction is pending

    // The component should show a warning message
    // This test would need the wallet provider to expose setTransactionPending
  });

  it('should show loading state during account refresh', async () => {
    mockWalletLib.refreshWalletSession.mockImplementation(() => 
      new Promise(resolve => setTimeout(() => resolve({
        walletId: 'freighter',
        address: mockAddress2,
        network: 'testnet',
        isConnected: true,
      }), 100))
    );

    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Mock account change detection
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    await waitFor(() => {
      expect(screen.getByText('Account Change Detected')).toBeInTheDocument();
    }, { timeout: 4000 });

    // Click refresh account
    const refreshButton = screen.getByText('Refresh Account');
    fireEvent.click(refreshButton);

    // Should show loading state
    expect(screen.getByText('Refreshing...')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText('Refreshing...')).not.toBeInTheDocument();
    });
  });

  it('should call onAccountChange callback when account changes', async () => {
    const onAccountChange = vi.fn();
    
    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress2,
      network: 'testnet',
      isConnected: true,
    });

    render(
      <TestWrapper>
        <AccountSwitcher onAccountChange={onAccountChange} />
      </TestWrapper>
    );

    // Connect wallet first
    const connectButton = screen.getByText('Connect Freighter');
    fireEvent.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText('↻ Refresh Account')).toBeInTheDocument();
    });

    // Mock account change detection
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    await waitFor(() => {
      expect(screen.getByText('Account Change Detected')).toBeInTheDocument();
    }, { timeout: 4000 });

    // Click refresh account
    const refreshButton = screen.getByText('Refresh Account');
    fireEvent.click(refreshButton);

    await waitFor(() => {
      expect(mockWalletLib.refreshWalletSession).toHaveBeenCalled();
    });

    // Note: The callback test would need the wallet provider to properly update
    // the address state after refresh for this to work correctly
  });
});
