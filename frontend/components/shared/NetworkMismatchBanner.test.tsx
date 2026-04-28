import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { NetworkMismatchBanner } from './NetworkMismatchBanner';
import * as WalletProvider from '@/components/providers/wallet-provider';

vi.mock('@/components/providers/wallet-provider', () => ({
  useWallet: vi.fn(),
}));

describe('NetworkMismatchBanner', () => {
  it('does not render when no network mismatch', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: false,
      network: 'testnet',
      walletNetwork: 'testnet',
      walletId: 'freighter',
      disconnect: vi.fn(),
    } as any);

    const { container } = render(<NetworkMismatchBanner />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders banner when network mismatch detected', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'freighter',
      disconnect: vi.fn(),
    } as any);

    render(<NetworkMismatchBanner />);
    expect(screen.getByText(/network mismatch detected/i)).toBeInTheDocument();
    expect(screen.getByText(/mainnet/i)).toBeInTheDocument();
    expect(screen.getByText(/testnet/i)).toBeInTheDocument();
  });

  it('shows wallet docs link when available', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'freighter',
      disconnect: vi.fn(),
    } as any);

    render(<NetworkMismatchBanner />);
    const link = screen.getByRole('link', { name: /how to switch network/i });
    expect(link).toHaveAttribute('href', 'https://docs.freighter.app/docs/guide/gettingStarted');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('calls disconnect when disconnect button clicked', async () => {
    const user = userEvent.setup();
    const disconnect = vi.fn();
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'freighter',
      disconnect,
    } as any);

    render(<NetworkMismatchBanner />);
    await user.click(screen.getByRole('button', { name: /disconnect wallet/i }));
    expect(disconnect).toHaveBeenCalled();
  });

  it('calls disconnect when close button clicked', async () => {
    const user = userEvent.setup();
    const disconnect = vi.fn();
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'freighter',
      disconnect,
    } as any);

    render(<NetworkMismatchBanner />);
    await user.click(screen.getByRole('button', { name: /dismiss and disconnect/i }));
    expect(disconnect).toHaveBeenCalled();
  });

  it('has proper ARIA attributes', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'freighter',
      disconnect: vi.fn(),
    } as any);

    render(<NetworkMismatchBanner />);
    const alert = screen.getByRole('alert');
    expect(alert).toHaveAttribute('aria-live', 'assertive');
  });

  it('handles xbull wallet docs link', () => {
    vi.mocked(WalletProvider.useWallet).mockReturnValue({
      networkMismatch: true,
      network: 'testnet',
      walletNetwork: 'mainnet',
      walletId: 'xbull',
      disconnect: vi.fn(),
    } as any);

    render(<NetworkMismatchBanner />);
    const link = screen.getByRole('link', { name: /how to switch network/i });
    expect(link).toHaveAttribute('href', 'https://xbull.app/docs');
  });
});
