import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { AccountSwitcher } from './account-switcher';
import { WalletProvider, useWallet } from '@/components/providers/wallet-provider';
import * as walletLib from '@/lib/wallet';
import React from 'react';

vi.mock('@/lib/wallet', () => ({
  getAvailableWallets: vi.fn(),
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  refreshWalletSession: vi.fn(),
  checkAddressChange: vi.fn(),
  checkWalletCapabilities: vi.fn(),
}));

const mockWalletLib = walletLib as any;

function ConnectThenRender({ children }: { children: React.ReactNode }) {
  const { connect, isConnected } = useWallet();

  React.useEffect(() => {
    void connect('freighter');
  }, [connect]);

  if (!isConnected) return <div>connecting</div>;
  return <>{children}</>;
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <WalletProvider defaultNetwork="testnet">
      <ConnectThenRender>{children}</ConnectThenRender>
    </WalletProvider>
  );
}

describe('AccountSwitcher', () => {
  const mockAddress1 = 'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  const mockAddress2 = 'GDEF456GHIJKLMNOPQRSTUVWXYZ789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456';

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockWalletLib.getAvailableWallets.mockResolvedValue([
      { id: 'freighter', label: 'Freighter', installed: true },
    ]);
    mockWalletLib.connectWallet.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });
    mockWalletLib.checkAddressChange.mockResolvedValue(null);
    mockWalletLib.checkWalletCapabilities.mockResolvedValue({
      checkedAt: Date.now(),
      statuses: [],
    });
    mockWalletLib.refreshWalletSession.mockResolvedValue({
      walletId: 'freighter',
      address: mockAddress1,
      network: 'testnet',
      isConnected: true,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('should not render when wallet is not connected', () => {
    render(
      <WalletProvider defaultNetwork="testnet">
        <AccountSwitcher />
      </WalletProvider>
    );

    expect(screen.queryByText(/refresh account/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/account changed/i)).not.toBeInTheDocument();
  });

  it('stays silent when connected and idle', async () => {
    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.queryByText('connecting')).not.toBeInTheDocument();
    });

    expect(screen.queryByText(/refresh account/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/account changed/i)).not.toBeInTheDocument();
  });

  it('should detect account changes and show notification', async () => {
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.queryByText('connecting')).not.toBeInTheDocument();
    });

    await act(async () => {
      vi.advanceTimersByTime(3100);
    });

    await waitFor(() => {
      expect(screen.getByText(/account changed/i)).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /refresh account/i })).toBeInTheDocument();
  });

  it('can dismiss account change notification', async () => {
    mockWalletLib.checkAddressChange.mockResolvedValue(mockAddress2);

    render(
      <TestWrapper>
        <AccountSwitcher />
      </TestWrapper>
    );

    await waitFor(() => {
      expect(screen.queryByText('connecting')).not.toBeInTheDocument();
    });

    await act(async () => {
      vi.advanceTimersByTime(3100);
    });

    await waitFor(() => {
      expect(screen.getByText(/account changed/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole('button', { name: /dismiss/i }));
    expect(screen.queryByText(/account changed/i)).not.toBeInTheDocument();
  });
});
