import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act, cleanup } from '@testing-library/react';
import { SwapCard } from '@/components/swap/SwapCard';
import { SESSION_RECOVERY_THRESHOLD_MS, STORAGE_KEY } from '@/hooks/useTradeFormStorage';
import { AppShell } from '@/components/layout/app-shell';
import { SessionRecoveryProvider } from '@/components/providers/session-recovery-provider';
import { StellarRouteApiError, stellarRouteClient } from '@/lib/api/client';
import { SettingsProvider } from '@/components/providers/settings-provider';
import { WalletProvider } from '@/components/providers/wallet-provider';

vi.unmock('@/components/providers/settings-provider');
vi.unmock('@/components/providers/wallet-provider');

vi.mock('@/lib/wallet', () => ({
  connectWallet: vi.fn(async () => ({
    address: 'GBRP56DYB6M6635Z57375OUXGBRP56DYB6M6635Z57375OUX',
    isConnected: true,
    walletId: 'freighter',
    network: 'testnet',
  })),
  disconnectWallet: vi.fn(),
  getAvailableWallets: vi.fn(async () => [
    { id: 'freighter', label: 'Freighter', installed: true }
  ]),
  refreshWalletSession: vi.fn(),
  signTransactionWithWallet: vi.fn(),
}));

function renderWithProviders(ui: React.ReactElement) {
  return render(
    <SettingsProvider>
      <SessionRecoveryProvider>
        <WalletProvider>{ui}</WalletProvider>
      </SessionRecoveryProvider>
    </SettingsProvider>
  );
}

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    prefetch: vi.fn(),
    back: vi.fn(),
  }),
  useSearchParams: () => new URLSearchParams(),
  usePathname: () => "/",
}));

function createQuoteResponse(overrides?: Partial<Record<string, unknown>>) {
  return {
    base_asset: { asset_type: 'native' },
    quote_asset: {
      asset_type: 'credit_alphanum4',
      asset_code: 'USDC',
      asset_issuer: 'GQUOTE',
    },
    amount: '10',
    total: '9.5',
    price: '0.95',
    price_impact: '0.5',
    path: [],
    quote_type: 'sell',
    timestamp: Date.now(),
    ...overrides,
  };
}

function createResponse(data: unknown) {
  return {
    ok: true,
    headers: new Headers(),
    json: async () => data,
  } as Response;
}

function setVisibilityState(state: DocumentVisibilityState) {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    get: () => state,
  });
}

describe('Session Recovery Integration', () => {
  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    setVisibilityState('visible');
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('should detect stale session after tab sleep and prompt recovery', async () => {
    // Mock fetch for API calls
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        return createResponse(createQuoteResponse());
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    // Connect wallet and enter trade details
    fireEvent.click(screen.getByRole('button', { name: /connect wallet/i }));
    fireEvent.change(screen.getByLabelText(/you pay/i), {
      target: { value: '100' },
    });

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    // Simulate tab going to sleep
    setVisibilityState('hidden');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // Advance time beyond threshold
    await act(async () => {
      vi.advanceTimersByTime(SESSION_RECOVERY_THRESHOLD_MS + 1000);
    });

    // Simulate tab waking up
    setVisibilityState('visible');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // Should show recovery modal
    expect(screen.getByText(/resume in-progress trade\?/i)).toBeInTheDocument();
    expect(screen.getByTestId('session-recovery-summary')).toBeInTheDocument();
  });

  it('should detect stale session after page refresh and prompt recovery', async () => {
    // Set up saved form state
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: '50',
        slippage: 1.5,
        deadline: 45,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now(),
      })
    );

    // Mock fetch
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        return createResponse(createQuoteResponse());
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Should show recovery modal for refresh
    expect(screen.getByText(/restore previous trade\?/i)).toBeInTheDocument();
    
    const summary = screen.getByTestId('session-recovery-summary');
    expect(summary).toHaveTextContent('50');
    expect(summary).toHaveTextContent('1.5%');
    expect(summary).toHaveTextContent('45 min');
  });

  it('should restore session and re-fetch quotes before enabling swap', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: '75',
        slippage: 2.0,
        deadline: 60,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now(),
      })
    );

    let quoteCalls = 0;
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        quoteCalls++;
        return createResponse(createQuoteResponse({ total: '71.25' }));
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Restore session
    fireEvent.click(screen.getByRole('button', { name: /restore session/i }));

    // Form should be restored
    expect(screen.getAllByLabelText(/you pay/i)[0]).toHaveValue('75');

    // Connect wallet
    fireEvent.click(screen.getByRole('button', { name: /connect wallet/i }));

    // Wait for state updates
    await act(async () => {
      await Promise.resolve();
    });

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    // Quote should be fetched and swap enabled
    expect(quoteCalls).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /swap/i })).toBeEnabled();
  });

  it('should handle recovery errors gracefully', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: '100',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now(),
      })
    );

    // Mock fetch to fail for quotes
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        throw new StellarRouteApiError(400, 'invalid_amount' as any, 'Network error');
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Try to restore session
    fireEvent.click(screen.getByRole('button', { name: /restore session/i }));

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    // Should show error
    expect(screen.getAllByText(/network error/i)[0]).toBeInTheDocument();
  });

  it('should not show recovery modal without recoverable context', async () => {
    // Clear any existing storage
    localStorage.clear();

    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    // Simulate tab sleep/wake without any saved context
    setVisibilityState('hidden');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    await act(async () => {
      vi.advanceTimersByTime(SESSION_RECOVERY_THRESHOLD_MS + 1000);
    });

    setVisibilityState('visible');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    // Should not show recovery modal
    expect(screen.queryByText(/resume in-progress trade\?/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/restore previous trade\?/i)).not.toBeInTheDocument();
  });

  it('should allow discarding recovery and starting fresh', async () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        amount: '100',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now(),
      })
    );

    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithProviders(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Discard recovery
    fireEvent.click(screen.getByRole('button', { name: /start fresh/i }));

    // Modal should close and form should be reset
    expect(screen.queryByText(/restore previous trade\?/i)).not.toBeInTheDocument();
    expect(screen.getAllByLabelText(/you pay/i)[0]).toHaveValue('');
  });

  it('should display the stale session banner in AppShell and trigger quote refresh with stored values when clicked', async () => {
    localStorage.setItem(
      'stellar-route-trade-form',
      JSON.stringify({
        amount: '125',
        slippage: 1.0,
        deadline: 30,
        fromToken: 'native',
        toToken: 'USDC:GQUOTE',
        savedAt: Date.now(),
      })
    );

    let resolveQuotePromise!: (value: any) => void;
    const quotePromise = new Promise<any>((resolve) => {
      resolveQuotePromise = resolve;
    });

    const getQuoteSpy = vi.spyOn(stellarRouteClient, 'getQuote').mockReturnValue(quotePromise);

    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/accounts/')) {
        return createResponse({
          balances: [
            {
              balance: '1000.0000000',
              asset_type: 'native',
            },
          ],
        });
      }
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <SettingsProvider>
        <WalletProvider>
          <SessionRecoveryProvider>
            <AppShell>
              <div>Main content</div>
            </AppShell>
          </SessionRecoveryProvider>
        </WalletProvider>
      </SettingsProvider>
    );

    expect(screen.queryByTestId('session-recovery-banner')).not.toBeInTheDocument();

    await act(async () => {
      await Promise.resolve();
    });

    setVisibilityState('hidden');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    await act(async () => {
      vi.advanceTimersByTime(SESSION_RECOVERY_THRESHOLD_MS + 1000);
    });

    setVisibilityState('visible');
    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    const banner = screen.getByTestId('session-recovery-banner');
    expect(banner).toBeInTheDocument();
    expect(
      screen.getByText(/your session is stale. would you like to restore/i)
    ).toBeInTheDocument();

    const restoreBtn = screen.getByRole('button', { name: /^restore$/i });
    
    // Start restoration
    await act(async () => {
      fireEvent.click(restoreBtn);
    });

    // Verify it displays Restoring...
    expect(screen.getByRole('button', { name: /restoring\.\.\./i })).toBeInTheDocument();
    expect(getQuoteSpy).toHaveBeenCalledWith('native', 'USDC:GQUOTE', 125, 'sell');

    // Resolve the quote promise
    await act(async () => {
      resolveQuotePromise(createQuoteResponse({ amount: '125', total: '118.75' }));
      await Promise.resolve();
    });

    await act(async () => {
      vi.advanceTimersByTime(1500);
    });

    // Banner should be removed
    expect(screen.queryByTestId('session-recovery-banner')).not.toBeInTheDocument();

    getQuoteSpy.mockRestore();
  });
});

