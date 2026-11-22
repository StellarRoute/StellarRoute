import React, { act } from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WalletConnectionOnboarding } from './WalletConnectionOnboarding';

vi.mock('@/lib/network-policy', async () => {
  const actual = await vi.importActual<typeof import('@/lib/network-policy')>(
    '@/lib/network-policy',
  );
  return {
    ...actual,
    getAllowedNetworks: vi.fn(() => ['testnet']),
    isNetworkAllowed: vi.fn(() => true),
  };
});

const baseProps = {
  open: true,
  onOpenChange: vi.fn(),
  availableWallets: [],
  isLoading: false,
  error: null,
  onConnect: vi.fn(),
  appNetwork: 'testnet' as const,
  walletNetwork: null,
};

describe('WalletConnectionOnboarding focus trap on open (W12-FE-07)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it('focuses the first interactive element when the dialog opens', () => {
    render(<WalletConnectionOnboarding {...baseProps} />);

    act(() => {
      vi.runAllTimers();
    });

    const focused = document.activeElement;
    expect(focused).not.toBe(document.body);
    expect(focused?.tagName.toLowerCase()).toMatch(/^(button|input|select|textarea|a)$/);
  });

  it('dialog content wrapper is present in the DOM when open', () => {
    render(<WalletConnectionOnboarding {...baseProps} />);
    expect(screen.getByTestId('wallet-connect-dialog')).toBeInTheDocument();
  });

  it('no focus shift when dialog is closed', () => {
    render(<WalletConnectionOnboarding {...baseProps} open={false} />);
    act(() => { vi.runAllTimers(); });
    expect(screen.queryByTestId('wallet-connect-dialog')).not.toBeInTheDocument();
  });

  it('focus moves to first button on step re-open', () => {
    const { rerender } = render(<WalletConnectionOnboarding {...baseProps} />);
    act(() => { vi.runAllTimers(); });

    expect(document.activeElement?.tagName.toLowerCase()).toBe('button');

    // close then reopen
    rerender(<WalletConnectionOnboarding {...baseProps} open={false} />);
    rerender(<WalletConnectionOnboarding {...baseProps} open />);
    act(() => { vi.runAllTimers(); });

    expect(document.activeElement?.tagName.toLowerCase()).toBe('button');
  });
});
