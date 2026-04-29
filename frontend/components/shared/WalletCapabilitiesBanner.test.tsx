import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WalletCapabilitiesBanner } from './WalletCapabilitiesBanner';
import * as WalletProvider from '@/components/providers/wallet-provider';

vi.mock('@/components/providers/wallet-provider', () => ({
  useWallet: vi.fn(),
}));

describe('WalletCapabilitiesBanner', () => {
  it('does not render when capabilities are null', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: null,
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    const { container } = render(<WalletCapabilitiesBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  it('does not render when all capabilities are allowed', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'request_access', allowed: true },
          { capability: 'view_address', allowed: true },
          { capability: 'view_network', allowed: true },
          { capability: 'sign_transaction', allowed: true },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    const { container } = render(<WalletCapabilitiesBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders banner when capability is denied', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'request_access', allowed: true },
          { capability: 'view_address', allowed: false, reason: 'Access denied', resolution: 'Reconnect wallet' },
          { capability: 'view_network', allowed: true },
          { capability: 'sign_transaction', allowed: false },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    expect(screen.getByText(/wallet permissions required/i)).toBeInTheDocument();
    expect(screen.getByText(/view address/i)).toBeInTheDocument();
    expect(screen.getByText(/access denied/i)).toBeInTheDocument();
    expect(screen.getByText(/reconnect wallet/i)).toBeInTheDocument();
  });

  it('shows sign_transaction denial with resolution', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'request_access', allowed: true },
          { capability: 'view_address', allowed: true },
          { capability: 'view_network', allowed: false, reason: 'Network mismatch', resolution: 'Switch wallet network' },
          { capability: 'sign_transaction', allowed: false, reason: 'Network mismatch', resolution: 'Switch wallet network' },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    expect(screen.getByText(/sign transactions/i)).toBeInTheDocument();
    expect(screen.getByText(/network mismatch/i)).toBeInTheDocument();
    expect(screen.getByText(/switch wallet network/i)).toBeInTheDocument();
  });

  it('calls refreshCapabilities when check again button clicked', async () => {
    const user = userEvent.setup();
    const refreshCapabilities = vi.fn();
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'sign_transaction', allowed: false, resolution: 'Reconnect wallet' },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities,
    } as any);

    render(<WalletCapabilitiesBanner />);
    await user.click(screen.getByRole('button', { name: /check again/i }));
    expect(refreshCapabilities).toHaveBeenCalled();
  });

  it('shows wallet docs link when available', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'sign_transaction', allowed: false, resolution: 'Allow signing' },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    const link = screen.getByRole('link', { name: /wallet docs/i });
    expect(link).toHaveAttribute('href', 'https://docs.freighter.app/docs/guide/gettingStarted');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('shows xbull wallet docs', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'sign_transaction', allowed: false },
        ],
      },
      walletId: 'xbull',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    const link = screen.getByRole('link', { name: /wallet docs/i });
    expect(link).toHaveAttribute('href', 'https://xbull.app/docs');
  });

  it('has proper ARIA attributes', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'sign_transaction', allowed: false },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    const alert = screen.getByRole('alert');
    expect(alert).toHaveAttribute('aria-live', 'polite');
  });

  it('shows multiple denied capabilities', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      capabilities: {
        checkedAt: Date.now(),
        statuses: [
          { capability: 'request_access', allowed: false, reason: 'Not granted' },
          { capability: 'view_address', allowed: false, reason: 'Address hidden' },
          { capability: 'view_network', allowed: true },
          { capability: 'sign_transaction', allowed: false },
        ],
      },
      walletId: 'freighter',
      refreshCapabilities: vi.fn(),
    } as any);

    render(<WalletCapabilitiesBanner />);
    expect(screen.getByText(/wallet access/i)).toBeInTheDocument();
    expect(screen.getByText(/view address/i)).toBeInTheDocument();
    expect(screen.getByText(/sign transactions/i)).toBeInTheDocument();
  });
});