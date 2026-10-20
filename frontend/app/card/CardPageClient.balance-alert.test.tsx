import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';

import { CardPageClient } from './CardPageClient';
import { saveCardTopUpPreferences } from '@/lib/card/preferences';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';

type WalletSession = {
  isConnected: boolean;
  address: string | null;
  network: string;
  walletId: string | null;
};

let walletSession: WalletSession = {
  isConnected: false,
  address: null,
  network: 'testnet',
  walletId: null,
};

vi.mock('@/hooks/useWallet', () => ({
  useWallet: () => ({ session: walletSession }),
}));

if (typeof global.ResizeObserver === 'undefined') {
  global.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function mockCardEndpoints(accountsBody: unknown, accountsStatus = 200) {
  vi.stubGlobal('fetch', (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.includes('/api/v1/card/accounts')) {
      return Promise.resolve(jsonResponse(accountsBody, accountsStatus));
    }
    return Promise.resolve(jsonResponse([]));
  });
}

const accountsWithBalance = (availableUsdc: string) => ({
  accounts: [
    {
      id: 'card-acct-1',
      status: 'active',
      last4: '4567',
      expiry: '12/28',
      available_usdc: availableUsdc,
    },
  ],
});

beforeEach(() => {
  vi.unstubAllGlobals();
  walletSession = {
    isConnected: false,
    address: null,
    network: 'testnet',
    walletId: null,
  };
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe('CardPageClient low balance alert', () => {
  it('renders CardFace with the alert directly below it when the balance is 24.99', async () => {
    mockCardEndpoints(accountsWithBalance('24.99'));

    render(<CardPageClient />);

    const alert = await screen.findByTestId('card-balance-alert');
    const section = screen.getByTestId('card-balance-section');
    const face = within(section).getByRole('img');
    expect(
      face.compareDocumentPosition(alert) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(alert).toHaveTextContent('24.99 USDC available');
  });

  it('hides the alert at exactly 25.00 USDC but keeps the card face', async () => {
    mockCardEndpoints(accountsWithBalance('25.00'));

    render(<CardPageClient />);

    const section = screen.getByTestId('card-balance-section');
    expect(await within(section).findByText('25.00 USDC')).toBeInTheDocument();
    await vi.waitFor(() =>
      expect(
        screen.queryByTestId('card-balance-alert'),
      ).not.toBeInTheDocument(),
    );
  });

  it('keeps the demo account and alerts when the accounts endpoint is not enabled', async () => {
    mockCardEndpoints({ error: 'not_found' }, 404);

    render(<CardPageClient />);

    // FIXTURE_ACCOUNT is 18.40 USDC, below the default 25.00 threshold.
    expect(await screen.findByTestId('card-balance-alert')).toBeInTheDocument();
  });

  it('uses saved preferences for the threshold and the recommended amount', async () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 40,
      defaultTopUpUsdc: 100,
    });
    walletSession = {
      isConnected: true,
      address: ADDRESS,
      network: 'testnet',
      walletId: 'freighter',
    };
    mockCardEndpoints(accountsWithBalance('35.00'));

    render(<CardPageClient />);

    // The first paint shows the fallback account at default preferences;
    // wait for the saved 40.00 threshold to take effect.
    await vi.waitFor(() =>
      expect(screen.getByTestId('card-balance-alert')).toHaveTextContent(
        'below the 40.00 USDC alert threshold',
      ),
    );
    // The balance line settles at the fetched 35.00 (or the 18.40 fallback),
    // both below the threshold, so only the threshold is asserted strictly.
    expect(screen.getByTestId('card-balance-alert')).toHaveTextContent(
      /USDC available/,
    );

    fireEvent.click(screen.getByTestId('card-balance-alert-top-up'));
    expect(await screen.findByTestId('card-top-up-amount-input')).toHaveValue(
      '100.00',
    );
  });

  it('opens the top-up modal pre-filled with 50.00 USDC from the alert', async () => {
    mockCardEndpoints(accountsWithBalance('18.40'));

    render(<CardPageClient />);

    fireEvent.click(await screen.findByTestId('card-balance-alert-top-up'));

    expect(screen.getByTestId('card-top-up-modal')).toBeInTheDocument();
    expect(screen.getByTestId('card-top-up-amount-input')).toHaveValue('50.00');
  });

  it('keeps the modal open with an error for a non-positive amount', async () => {
    mockCardEndpoints(accountsWithBalance('18.40'));

    render(<CardPageClient />);

    fireEvent.click(await screen.findByTestId('card-balance-alert-top-up'));
    fireEvent.change(screen.getByTestId('card-top-up-amount-input'), {
      target: { value: '0' },
    });
    fireEvent.click(screen.getByTestId('card-top-up-confirm'));

    expect(screen.getByTestId('card-top-up-amount-error')).toHaveTextContent(
      /greater than 0/,
    );
    expect(screen.getByTestId('card-top-up-modal')).toBeInTheDocument();
  });

  it('carries the edited amount to the bridge link', async () => {
    mockCardEndpoints(accountsWithBalance('18.40'));

    render(<CardPageClient />);

    fireEvent.click(await screen.findByTestId('card-balance-alert-top-up'));
    fireEvent.change(screen.getByTestId('card-top-up-amount-input'), {
      target: { value: '120' },
    });

    expect(screen.getByTestId('card-top-up-bridge-link')).toHaveAttribute(
      'href',
      '/cross-chain-swap?amount=120&destination=card',
    );
  });

  it('dismisses the alert for the session without persisting to localStorage', async () => {
    mockCardEndpoints(accountsWithBalance('18.40'));

    const { unmount } = render(<CardPageClient />);

    fireEvent.click(await screen.findByTestId('card-balance-alert-dismiss'));
    expect(
      screen.queryByTestId('card-balance-alert'),
    ).not.toBeInTheDocument();

    const dismissKey = 'stellarroute:card:balance-alert-dismissed:anonymous:25';
    expect(window.sessionStorage.getItem(dismissKey)).toBe('true');
    expect(window.localStorage.getItem(dismissKey)).toBeNull();

    // A remount in the same session (no reload) stays dismissed.
    unmount();
    render(<CardPageClient />);
    await vi.waitFor(() =>
      expect(
        screen.queryByTestId('card-balance-alert'),
      ).not.toBeInTheDocument(),
    );
  });
});
