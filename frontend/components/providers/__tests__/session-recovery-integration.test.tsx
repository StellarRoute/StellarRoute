import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import { SwapCard } from '@/components/swap/SwapCard';
import { SESSION_RECOVERY_THRESHOLD_MS, STORAGE_KEY } from '@/hooks/useTradeFormStorage';

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
    vi.useRealTimers();
    vi.restoreAllMocks();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('should detect stale session after tab sleep and prompt recovery', async () => {
    // Mock fetch for API calls
    const fetchMock = vi.fn(async (input: string | URL | Request) => {
      const url = String(input);
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        return createResponse(createQuoteResponse());
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<SwapCard />);

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
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        return createResponse(createQuoteResponse());
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<SwapCard />);

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

    render(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Restore session
    fireEvent.click(screen.getByRole('button', { name: /restore session/i }));

    // Form should be restored
    expect(screen.getAllByLabelText(/you pay/i)[0]).toHaveValue('75');

    // Connect wallet
    fireEvent.click(screen.getByRole('button', { name: /connect wallet/i }));

    // Should show refreshing state initially
    expect(screen.getByRole('button', { name: /refreshing quote/i })).toBeDisabled();

    await act(async () => {
      vi.advanceTimersByTime(400);
      await Promise.resolve();
    });

    // Quote should be fetched and swap enabled
    expect(quoteCalls).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: /^swap$/i })).toBeEnabled();
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
      if (url.includes('/api/v1/pairs')) {
        return createResponse({ pairs: [], total: 0 });
      }
      if (url.includes('/api/v1/quote/')) {
        throw new Error('Network error');
      }
      return createResponse({});
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Try to restore session
    fireEvent.click(screen.getByRole('button', { name: /restore session/i }));

    // Should show error in modal
    await waitFor(() => {
      expect(screen.getByText(/network error/i)).toBeInTheDocument();
    });
  });

  it('should not show recovery modal without recoverable context', async () => {
    // Clear any existing storage
    localStorage.clear();

    const fetchMock = vi.fn(async () => createResponse({}));
    vi.stubGlobal('fetch', fetchMock);

    render(<SwapCard />);

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

    const fetchMock = vi.fn(async () => createResponse({}));
    vi.stubGlobal('fetch', fetchMock);

    render(<SwapCard />);

    await act(async () => {
      await Promise.resolve();
    });

    // Discard recovery
    fireEvent.click(screen.getByRole('button', { name: /start fresh/i }));

    // Modal should close and form should be reset
    expect(screen.queryByText(/restore previous trade\?/i)).not.toBeInTheDocument();
    expect(screen.getAllByLabelText(/you pay/i)[0]).toHaveValue('');
  });
});
