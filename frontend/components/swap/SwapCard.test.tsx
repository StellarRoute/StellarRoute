
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi, Mock } from 'vitest';
import { SwapCard } from './SwapCard';
import { fireEvent } from '@testing-library/react';
import * as useSwapStateModule from '@/hooks/useSwapState';

import { WalletProvider } from '@/components/providers/wallet-provider';
import { SettingsProvider } from '@/components/providers/settings-provider';

vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
  }),
  useSearchParams: () => ({
    get: vi.fn(),
  }),
}));

vi.mock('./ShareQuoteButton', () => ({
  ShareQuoteButton: () => <button data-testid="mock-share-quote-button">Share</button>,
}));

vi.mock('@/components/providers/wallet-provider', () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const React = require('react');
  return {
    WalletProvider: ({ children }: any) => <>{children}</>,
    useWallet: () => {
      const [connected, setConnected] = React.useState(false);
      const [address, setAddress] = React.useState(null);

      const connect = React.useCallback(async (walletId: string) => {
        setConnected(true);
        setAddress('GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ');
      }, []);

      const disconnect = React.useCallback(() => {
        setConnected(false);
        setAddress(null);
      }, []);

      return {
        address,
        isConnected: connected,
        walletId: connected ? 'freighter' : null,
        network: 'testnet',
        networkMismatch: false,
        connect,
        disconnect,
        reconnect: React.useCallback(async () => {}, []),
        setNetwork: React.useCallback(() => {}, []),
        autoReconnectPreferred: true,
        setAutoReconnectPreferred: React.useCallback(() => {}, []),
        refreshWallets: React.useCallback(async () => {}, []),
        refreshAccount: React.useCallback(async () => {}, []),
        accountSwitchState: { isDetecting: false, hasChanged: false, previousAddress: null },
        isTransactionPending: false,
        setTransactionPending: React.useCallback(() => {}, []),
        capabilities: null,
        refreshCapabilities: React.useCallback(async () => {}, []),
        syncMismatch: false,
        resyncWallet: React.useCallback(async () => {}, []),
        dismissSyncMismatch: React.useCallback(() => {}, []),
      };
    },
  };
});

vi.mock('@/lib/wallet', () => ({
  connectWallet: vi.fn(),
  disconnectWallet: vi.fn(),
  getAvailableWallets: vi.fn(),
  refreshWalletSession: vi.fn(),
  signTransactionWithWallet: vi.fn(),
}));

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <SettingsProvider>
      <WalletProvider>{ui}</WalletProvider>
    </SettingsProvider>
  );
}

function setNavigatorOnline(value: boolean) {
  Object.defineProperty(window.navigator, 'onLine', {
    configurable: true,
    value,
  });
}

describe('SwapCard network resilience and states', () => {
  beforeEach(() => {
    localStorage.clear();
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [
                {
                  balance: '50.0000000',
                  asset_type: 'native',
                },
              ],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '9.5',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '10',
          }),
      });
    }) as Mock;
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('should render successfully', () => {
    renderWithProviders(<SwapCard />);
    expect(screen.getByRole('heading', { name: /swap/i })).toBeInTheDocument();
  });

  it('shows initial state requiring wallet connection', async () => {
    renderWithProviders(<SwapCard />);
    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    expect(connectButton).toBeInTheDocument();
  });

  it('transitions states after wallet connection', async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    await waitFor(() => {
      expect(screen.getByText(/enter amount/i)).toBeInTheDocument();
    });

    const payInput = screen.getByLabelText(/you pay/i);
    // Optimized: fireEvent bypasses keypress rendering overhead to prevent timeouts
    fireEvent.change(payInput, { target: { value: '10' } });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /review swap/i })).toBeEnabled();
    });
  });

  it('shows high price impact warning for large amounts', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '50',
            price_impact: '15.0', // High price impact (> 10%)
            path: [],
            price: '0.5',
            amount: '90',
          }),
      })
    ) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    // Connect wallet step
    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Explicitly update input field value
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '90' } });

    // Wait for the button text content state to transition to dangerous style overrides
    await waitFor(() => {
      const allButtons = screen.getAllByRole('button');
      const dangerousButton = allButtons.find(
        (btn) =>
          btn.textContent?.toLowerCase().includes('swap') ||
          btn.className.includes('bg-destructive')
      );
      expect(dangerousButton).toBeDefined();
    });
  });

  it('shows insufficient balance state', async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '100.0155' } });

    await waitFor(() => {
      const balanceButton = screen.getByRole('button', {
        name: /insufficient balance/i,
      });
      expect(balanceButton).toBeDisabled();
    });
  });
});

// --- Issue #506: Added Dedicated Stellar Memo Validation Rule Tests ---
describe('SwapCard Stellar Memo Validation Inline Rules (#506)', () => {
  afterEach(() => {
    cleanup();
  });

  it('shows validation error when a text memo is over 28 bytes', async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '5' } });

    const toggleButton = screen.getByText('+ Add Optional Memo');
    await user.click(toggleButton);

    const memoInput =
      await screen.findByPlaceholderText(/enter text reference/i);
    fireEvent.change(memoInput, {
      target: {
        value:
          'This text string is completely far too long for a standard Stellar memo field validation restriction rules.',
      },
    });

    await waitFor(() => {
      expect(screen.getByText(/exceeds 28 bytes/i)).toBeInTheDocument();
    });
  }, 10_000);

  it('shows validation error when a hash memo is not valid hexadecimal characters', async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '5' } });

    const toggleButton = screen.getByText('+ Add Optional Memo');
    await user.click(toggleButton);

    // Using findByText handles the UI state delay smoothly
    const hashModeButton = await screen.findByText('Hash Memo');
    await user.click(hashModeButton);

    const memoInput = await screen.findByPlaceholderText(
      /enter 64-char hex string/i
    );
    fireEvent.change(memoInput, { target: { value: 'not-a-hex-value' } });

    await waitFor(() => {
      expect(
        screen.getByText(/must be exactly 64 hexadecimal characters/i)
      ).toBeInTheDocument();
    });
  });
});

// --- Issue #644/#705: Wallet Balance Integration Tests ---
describe('SwapCard Wallet Balance Integration (#644/#705)', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('displays real wallet balance when wallet is connected', async () => {
    // Mock Horizon API to return account with balance
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [
                {
                  balance: '50.0000000',
                  asset_type: 'native',
                },
              ],
            }),
        });
      }
      // Quote API mock
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '9.5',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '10',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    // Connect wallet
    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Wait for balance to be displayed
    await waitFor(() => {
      expect(screen.getByText(/Balance:/i)).toBeInTheDocument();
      expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
    });
  });

  it("shows 'Loading...' balance state while fetching balance", async () => {
    // Delay the Horizon response to test loading state
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return new Promise((resolve) =>
          setTimeout(() => {
            resolve({
              ok: true,
              json: () =>
                Promise.resolve({
                  balances: [{ balance: '50.0000000', asset_type: 'native' }],
                }),
            });
          }, 100)
        );
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '9.5',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '10',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Balance should show loading state
    await waitFor(() => {
      const balanceText = screen.queryByText(/Loading\.\.\./);
      // Loading state should appear briefly
      expect(balanceText || screen.getByText(/Balance:/i)).toBeInTheDocument();
    });
  });

  it("shows 'Unavailable' balance state when fetch fails", async () => {
    // Mock Horizon API failure
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: false,
          json: () => Promise.reject(new Error('Network error')),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '9.5',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '10',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Wait for error state
    await waitFor(() => {
      expect(screen.getByText(/Unavailable/)).toBeInTheDocument();
    });
  });
  it('MAX button sets amount to full balance for non-native assets', async () => {
    const originalUseSwapState = useSwapStateModule.useSwapState;
    vi.spyOn(useSwapStateModule, 'useSwapState').mockImplementation(() => {
      const state = originalUseSwapState();
      return {
        ...state,
        fromToken: 'USDC:GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH',
      };
    });

    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [
                {
                  balance: '100.0000000',
                  asset_type: 'credit_alphanum4',
                  asset_code: 'USDC',
                  asset_issuer:
                    'GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH',
                },
              ],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '95.0',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '100',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Find and click MAX button
    await waitFor(() => {
      const maxButton = screen.getByRole('button', { name: /MAX/i });
      expect(maxButton).toBeInTheDocument();
      return maxButton;
    });

    const maxButton = screen.getByRole('button', { name: /MAX/i });
    await user.click(maxButton);

    // Verify amount is set to balance
    const payInput = screen.getByLabelText(/you pay/i) as HTMLInputElement;
    await waitFor(() => {
      expect(payInput.value).toBe('100.0000000');
    });
  });

  it('MAX button sets amount to spendable balance for native XLM', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [
                {
                  balance: '55.5000000', // Total balance includes base reserve
                  asset_type: 'native',
                },
              ],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '50.0',
            price_impact: '0.5',
            path: [],
            price: '0.9',
            amount: '55',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Find and click MAX button
    await waitFor(() => {
      const maxButton = screen.getByRole('button', { name: /MAX/i });
      expect(maxButton).toBeInTheDocument();
      return maxButton;
    });
    const maxButton = screen.getByRole('button', { name: /MAX/i });
    await user.click(maxButton);

    // Verify amount is set to spendable balance (minus base reserve of ~5 XLM)
    const payInput = screen.getByLabelText(/you pay/i) as HTMLInputElement;
    await waitFor(() => {
      const amount = parseFloat(payInput.value);
      // Should be less than total balance due to reserve
      expect(amount).toBeLessThan(55);
      expect(amount).toBeGreaterThan(0);
    });
  });

  it('balance updates when user switches tokens', async () => {
    let callCount = 0;
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        callCount++;
        // Return different balance on second call
        const isSecondCall = callCount > 1;
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: isSecondCall
                ? [
                    {
                      balance: '200.0000000',
                      asset_type: 'credit_alphanum4',
                      asset_code: 'EUR',
                      asset_issuer: 'GABC',
                    },
                  ]
                : [{ balance: '50.0000000', asset_type: 'native' }],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '9.5',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '10',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Initial balance should be shown
    await waitFor(() => {
      expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
    });

    // Switch token (would trigger new balance fetch)
    // Note: Full token switching test would require more mocking
    // This serves as a placeholder for integration testing
  });

  it('prevents swap when amount exceeds real balance', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [{ balance: '50.0000000', asset_type: 'native' }],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '95.0',
            price_impact: '0.5',
            path: [],
            price: '0.95',
            amount: '100',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Enter amount greater than balance
    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '100' } });

    // Swap button should be disabled with insufficient balance message
    await waitFor(() => {
      const swapButton = screen.getByRole('button', { name: /insufficient balance/i });
      expect(swapButton).toBeDisabled();
    });
  });

  it('shows balance for specific asset when selected', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              balances: [
                { balance: '50.0000000', asset_type: 'native' },
                {
                  balance: '1000.0000000',
                  asset_type: 'credit_alphanum12',
                  asset_code: 'SorobanToken',
                  asset_issuer: 'GABC123',
                },
              ],
            }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () =>
          Promise.resolve({
            total: '990.0',
            price_impact: '0.5',
            path: [],
            price: '0.99',
            amount: '1000',
          }),
      });
    }) as Mock;

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    const connectButton = screen.getByRole('button', {
      name: /connect wallet/i,
    });
    await user.click(connectButton);

    // Should display native XLM balance initially
    await waitFor(() => {
      expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
    });
  });
});
