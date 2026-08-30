/**
 * Lifecycle → SwapCard wiring: structured API errors must reach modal/toast
 * curated copy (not stripped via `new Error(message)`).
 */
import type { ReactElement } from 'react';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { toast } from 'sonner';
import { SwapCard } from './SwapCard';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { StellarRouteApiError } from '@/lib/api/client';

const {
  featureFlags,
  flagLoading,
  prepareSwapMock,
  mockWalletState,
  mockQuoteRefresh,
} = vi.hoisted(() => ({
  featureFlags: { real_xdr: true } as Record<string, boolean>,
  flagLoading: { value: false },
  prepareSwapMock: vi.fn(),
  mockWalletState: {
    capabilities: {
      checkedAt: Date.now(),
      statuses: [{ capability: 'sign_transaction', allowed: true }],
    } as {
      checkedAt: number;
      statuses: Array<{
        capability: string;
        allowed: boolean;
        reason?: string;
        resolution?: string;
      }>;
    } | null,
  },
  mockQuoteRefresh: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
  },
}));

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn() }),
  useSearchParams: () => ({ get: vi.fn() }),
}));

vi.mock('@/hooks/useFeatureFlag', () => ({
  useFeatureFlag: (flag: string) => ({
    enabled: !!featureFlags[flag],
    loading: flagLoading.value,
  }),
  useFeatureFlags: (flags: string[]) =>
    Object.fromEntries(flags.map((f) => [f, !!featureFlags[f]])),
  invalidateFlagCache: vi.fn(),
}));

vi.mock('@/lib/api/client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/api/client')>();
  const makeMockClient = () => {
    const client = actual.createStellarRouteClient();
    client.prepareSwap = (...args: unknown[]) => prepareSwapMock(...args);
    client.submitSwap = vi.fn();
    return client;
  };
  return {
    ...actual,
    createStellarRouteClient: makeMockClient,
    stellarRouteClient: makeMockClient(),
  };
});

vi.mock('./ShareQuoteButton', () => ({
  ShareQuoteButton: () => (
    <button data-testid="mock-share-quote-button">Share</button>
  ),
}));

vi.mock('@/hooks/useQuoteRefresh', () => ({
  useQuoteRefresh: (...args: unknown[]) => mockQuoteRefresh(...args),
}));

vi.mock('@/hooks/useApi', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/hooks/useApi')>();
  return {
    ...actual,
    usePairs: () => ({
      data: [
        {
          base: 'XLM',
          base_asset: 'native',
          counter: 'USDC',
          counter_asset:
            'USDC:GATEMHCCKCY67ZUCKTROYN24ZYT5GK4EQZ65JJLDHKHRUZI3EUEKMTCH',
        },
      ],
      loading: false,
      error: null,
      refresh: vi.fn(),
    }),
    useQuoteStream: () => ({
      data: undefined,
      isConnected: false,
      error: null,
      wsAvailable: false,
    }),
    useRoutes: () => ({
      data: undefined,
      loading: false,
      error: null,
      refresh: vi.fn(),
    }),
    useBatchQuote: () => ({
      data: undefined,
      loading: false,
      error: null,
      refresh: vi.fn(),
    }),
  };
});

vi.mock('@/components/providers/wallet-provider', () => {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const React = require('react');
  return {
    WalletProvider: ({ children }: { children: React.ReactNode }) => (
      <>{children}</>
    ),
    useWallet: () => {
      const [connected, setConnected] = React.useState(false);
      const [address, setAddress] = React.useState<string | null>(null);

      const connect = React.useCallback(async () => {
        setConnected(true);
        setAddress(
          'GABC123DEFGHIJKLMNOPQRSTUVWXYZ456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
        );
      }, []);

      return {
        address,
        isConnected: connected,
        walletId: connected ? 'freighter' : null,
        network: 'testnet',
        walletNetwork: 'testnet',
        networkMismatch: false,
        connect,
        disconnect: React.useCallback(() => {
          setConnected(false);
          setAddress(null);
        }, []),
        reconnect: React.useCallback(async () => {}, []),
        setNetwork: React.useCallback(() => {}, []),
        autoReconnectPreferred: true,
        setAutoReconnectPreferred: React.useCallback(() => {}, []),
        refreshWallets: React.useCallback(async () => {}, []),
        refreshAccount: React.useCallback(async () => {}, []),
        accountSwitchState: {
          isDetecting: false,
          hasChanged: false,
          previousAddress: null,
        },
        isTransactionPending: false,
        setTransactionPending: React.useCallback(() => {}, []),
        capabilities: mockWalletState.capabilities,
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
  signTransactionWithWallet: vi.fn().mockResolvedValue('AAAAtest_signed_xdr'),
}));

const classicPath = [
  {
    from_asset: { asset_type: 'native' as const },
    to_asset: {
      asset_type: 'credit_alphanum4' as const,
      asset_code: 'USDC',
      asset_issuer: 'GABC',
    },
    source: 'sdex',
    fee_bps: 30,
    price: '0.95',
  },
];

function renderWithProviders(ui: ReactElement) {
  return render(<SettingsProvider>{ui}</SettingsProvider>);
}

function configureClassicQuote(amount?: number) {
  const hasAmount =
    amount !== undefined && Number.isFinite(amount) && amount > 0;
  mockQuoteRefresh.mockImplementation(
    (_base: string, _quote: string, amt?: number) => {
      const active = hasAmount
        ? amount
        : amt !== undefined && Number.isFinite(amt) && amt > 0
          ? amt
          : undefined;
      return {
        data: active
          ? {
              base_asset: { asset_type: 'native' },
              quote_asset: {
                asset_type: 'credit_alphanum4',
                asset_code: 'USDC',
                asset_issuer: 'GABC',
              },
              amount: String(active),
              price: '0.95',
              total: '9.5',
              price_impact: '0.5',
              path: classicPath,
              quote_type: 'sell' as const,
              timestamp: Date.now(),
            }
          : undefined,
        loading: false,
        error: null,
        isStale: false,
        refresh: vi.fn(),
        lastQuotedAtMs: active ? Date.now() : null,
        requestId: active ? 'test-req-id' : null,
        isRecovering: false,
        retryAttempt: 0,
        hasPendingRetry: false,
        pendingRetryRemainingMs: 0,
        cancelRetry: vi.fn(),
        manualRefreshCoolingDown: false,
        autoRefreshEnabled: false,
        setAutoRefreshEnabled: vi.fn(),
        rateLimitRemainingMs: 0,
      };
    },
  );
}

async function reviewSwapUntilFailed(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: /connect wallet/i }));
  await waitFor(() => {
    expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
  });

  const payInput = screen.getByLabelText(/you pay/i);
  fireEvent.change(payInput, { target: { value: '10' } });

  await waitFor(() => {
    expect(
      screen.getByRole('button', { name: /review swap/i }),
    ).toBeEnabled();
  });

  await user.click(screen.getByRole('button', { name: /review swap/i }));

  await waitFor(() => {
    expect(screen.getByTestId('swap-confirm-dialog')).toBeInTheDocument();
  });

  await waitFor(() => {
    expect(prepareSwapMock).toHaveBeenCalled();
  });
}

describe('SwapCard flag-loading fail-closed', () => {
  beforeEach(() => {
    localStorage.clear();
    featureFlags.real_xdr = false;
    flagLoading.value = true;
    prepareSwapMock.mockReset();
    configureClassicQuote();
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              sequence: '12345',
              balances: [{ balance: '50.0000000', asset_type: 'native' }],
            }),
        });
      }
      return Promise.reject(new Error(`Unexpected fetch: ${url}`));
    }) as typeof fetch;
  });

  afterEach(() => {
    cleanup();
    flagLoading.value = false;
    featureFlags.real_xdr = true;
    vi.clearAllMocks();
  });

  it('loading with enabled=false keeps CTA fail-closed (no prepare / no alternate path)', async () => {
    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));
    await waitFor(() => {
      expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
    });

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '10' } });

    // Disabled execution mode maps to error CTA — never Review Swap / client-XDR.
    await waitFor(() => {
      const cta = screen.getByRole('button', {
        name: /error fetching quote|unable to fetch quote|error/i,
      });
      expect(cta).toBeDisabled();
    });
    expect(
      screen.queryByRole('button', { name: /review swap/i }),
    ).not.toBeInTheDocument();
    expect(prepareSwapMock).not.toHaveBeenCalled();
  });
});

describe('SwapCard lifecycle → curated API errors (real_xdr)', () => {
  beforeEach(() => {
    localStorage.clear();
    featureFlags.real_xdr = true;
    flagLoading.value = false;
    prepareSwapMock.mockReset();
    configureClassicQuote();
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/accounts/')) {
        return Promise.resolve({
          ok: true,
          json: () =>
            Promise.resolve({
              sequence: '12345',
              balances: [
                { balance: '50.0000000', asset_type: 'native' },
              ],
            }),
        });
      }
      return Promise.reject(new Error(`Unexpected fetch: ${url}`));
    }) as typeof fetch;
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it('surfaces unsupported_route curated copy via lifecycle → modal', async () => {
    prepareSwapMock.mockRejectedValue(
      new StellarRouteApiError(422, 'unsupported_route', 'multi-hop blocked', {
        internal: 'leak-me',
      }),
    );

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    await reviewSwapUntilFailed(user);

    const dialog = screen.getByTestId('swap-confirm-dialog');
    await waitFor(() => {
      expect(dialog.textContent).toMatch(
        /route shape is not supported|classic one-hop/i,
      );
    });
    expect(dialog.textContent).not.toMatch(/leak-me/i);
    expect(dialog.textContent).not.toMatch(/multi-hop blocked/i);
  });

  it('surfaces active_prepare_exists conflict copy via lifecycle → modal', async () => {
    prepareSwapMock.mockRejectedValue(
      new StellarRouteApiError(409, 'duplicate_quote', 'conflict', {
        status: 'active_prepare_exists',
        quote_id: 'q_secret',
      }),
    );

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    await reviewSwapUntilFailed(user);

    const dialog = screen.getByTestId('swap-confirm-dialog');
    await waitFor(() => {
      expect(dialog.textContent).toMatch(/active prepare/i);
    });
    expect(dialog.textContent).not.toMatch(/q_secret/i);
  });

  it('surfaces quote_expired curated copy via lifecycle → modal', async () => {
    prepareSwapMock.mockRejectedValue(
      new StellarRouteApiError(422, 'quote_expired', 'expired server detail', {
        expires_at: 'never-show',
      }),
    );

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);
    await reviewSwapUntilFailed(user);

    const dialog = screen.getByTestId('swap-confirm-dialog');
    await waitFor(() => {
      expect(dialog.textContent).toMatch(/quote expired/i);
    });
    expect(dialog.textContent).not.toMatch(/never-show/i);
    expect(dialog.textContent).not.toMatch(/expired server detail/i);
  });

  it('surfaces curated API copy via bypassConfirmation toast path', async () => {
    localStorage.setItem('stellarroute.settings.expertMode', 'true');
    localStorage.setItem('stellarroute.settings.bypassConfirmation', 'true');
    prepareSwapMock.mockRejectedValue(
      new StellarRouteApiError(422, 'unsupported_route', 'multi-hop blocked', {
        internal: 'leak-me',
      }),
    );

    const user = userEvent.setup();
    renderWithProviders(<SwapCard />);

    // useExpertSettings hydrates bypassConfirmation via queueMicrotask.
    await waitFor(() => {
      expect(
        localStorage.getItem('stellarroute.settings.bypassConfirmation'),
      ).toBe('true');
    });
    await Promise.resolve();
    await Promise.resolve();

    await user.click(screen.getByRole('button', { name: /connect wallet/i }));
    await waitFor(() => {
      expect(screen.getByText(/50\.0000000/)).toBeInTheDocument();
    });

    const payInput = screen.getByLabelText(/you pay/i);
    fireEvent.change(payInput, { target: { value: '10' } });

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: /review swap/i }),
      ).toBeEnabled();
    });

    await user.click(screen.getByRole('button', { name: /review swap/i }));

    await waitFor(() => expect(prepareSwapMock).toHaveBeenCalled());
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringMatching(/route shape is not supported/i),
        expect.objectContaining({ id: 'swap-toast' }),
      );
    });

    expect(screen.queryByTestId('swap-confirm-dialog')).not.toBeInTheDocument();
    const toastLine = String(vi.mocked(toast.error).mock.calls[0]?.[0] ?? '');
    expect(toastLine).not.toMatch(/leak-me|multi-hop blocked/i);
  });
});
