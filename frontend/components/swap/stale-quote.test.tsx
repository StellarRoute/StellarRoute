/**
 * stale-quote.test.tsx
 *
 * Tests for Issue #1000 — Enforce stale-quote UX before Confirm is enabled.
 *
 * Coverage:
 *  1. useStaleQuote hook — expired via expires_at
 *  2. useStaleQuote hook — expired via isStale fallback
 *  3. useStaleQuote hook — price impact changed > 0.5%
 *  4. useStaleQuote hook — price impact changed exactly at threshold (boundary)
 *  5. useStaleQuote hook — price impact change below threshold (fresh)
 *  6. useStaleQuote hook — resets baseline when new quote arrives
 *  7. SwapButton renders "Update Quote" CTA in stale_quote state (disabled = false)
 *  8. SwapButton "Update Quote" is disabled while loading
 *  9. SwapButton calls onUpdateQuote when clicked
 * 10. SwapButton Confirm is NOT rendered in stale_quote state (only Update Quote)
 * 11. SwapButton does NOT call onSwap in stale_quote state
 */

import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
} from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT,
  useStaleQuote,
} from '@/hooks/useStaleQuote';
import { SwapButton } from './SwapButton';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Advance fake timers AND flush all pending React state updates in one shot.
 * This avoids the waitFor + fake-timers deadlock (waitFor uses real setTimeout
 * internally, which is shadowed by vi.useFakeTimers).
 */
function tickMs(ms: number) {
  act(() => {
    vi.advanceTimersByTime(ms);
  });
}

// ---------------------------------------------------------------------------
// useStaleQuote hook tests
// ---------------------------------------------------------------------------

describe('useStaleQuote', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('reports expired when now >= expires_at', () => {
    const expiresAtMs = Date.now() + 3_000; // expires 3s from now

    const { result } = renderHook(() =>
      useStaleQuote({
        expiresAtMs,
        currentPriceImpact: 1.0,
        isStale: false,
        lastQuotedAtMs: Date.now(),
      }),
    );

    // Before expiry — fresh
    expect(result.current.isExpired).toBe(false);
    expect(result.current.isQuoteStale).toBe(false);

    // Advance past expiry (hook ticks every 1 000ms)
    tickMs(4_000);

    expect(result.current.isExpired).toBe(true);
    expect(result.current.isQuoteStale).toBe(true);
  });

  it('falls back to isStale flag when expires_at is not provided', () => {
    const { result, rerender } = renderHook(
      ({ isStale }: { isStale: boolean }) =>
        useStaleQuote({
          expiresAtMs: undefined,
          currentPriceImpact: 1.0,
          isStale,
          lastQuotedAtMs: Date.now(),
        }),
      { initialProps: { isStale: false } },
    );

    expect(result.current.isExpired).toBe(false);
    expect(result.current.isQuoteStale).toBe(false);

    act(() => {
      rerender({ isStale: true });
    });

    expect(result.current.isExpired).toBe(true);
    expect(result.current.isQuoteStale).toBe(true);
  });

  it('reports stale when price impact changes more than 0.5%', () => {
    const now = Date.now();
    const { result, rerender } = renderHook(
      ({ currentPriceImpact }: { currentPriceImpact: number }) =>
        useStaleQuote({
          expiresAtMs: now + 60_000, // far in future — not expired
          currentPriceImpact,
          isStale: false,
          lastQuotedAtMs: now,
        }),
      { initialProps: { currentPriceImpact: 1.0 } },
    );

    // Baseline set on first mount
    expect(result.current.baselinePriceImpact).toBe(1.0);

    // Simulate market movement: impact jumps by > 0.5%
    act(() => {
      rerender({ currentPriceImpact: 1.0 + STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT + 0.01 });
    });

    expect(result.current.isPriceImpactChanged).toBe(true);
    expect(result.current.isQuoteStale).toBe(true);
  });

  it('treats a change exactly equal to the threshold (0.5%) as NOT stale', () => {
    const now = Date.now();
    const { result, rerender } = renderHook(
      ({ currentPriceImpact }: { currentPriceImpact: number }) =>
        useStaleQuote({
          expiresAtMs: now + 60_000,
          currentPriceImpact,
          isStale: false,
          lastQuotedAtMs: now,
        }),
      { initialProps: { currentPriceImpact: 1.0 } },
    );

    expect(result.current.baselinePriceImpact).toBe(1.0);

    // Change exactly equal to threshold — should NOT be stale (strictly greater than)
    act(() => {
      rerender({ currentPriceImpact: 1.0 + STALE_QUOTE_PRICE_IMPACT_THRESHOLD_PCT });
    });

    expect(result.current.isPriceImpactChanged).toBe(false);
    expect(result.current.isQuoteStale).toBe(false);
  });

  it('remains fresh when price impact change is below threshold', () => {
    const now = Date.now();
    const { result, rerender } = renderHook(
      ({ currentPriceImpact }: { currentPriceImpact: number }) =>
        useStaleQuote({
          expiresAtMs: now + 60_000,
          currentPriceImpact,
          isStale: false,
          lastQuotedAtMs: now,
        }),
      { initialProps: { currentPriceImpact: 1.0 } },
    );

    expect(result.current.baselinePriceImpact).toBe(1.0);

    act(() => {
      rerender({ currentPriceImpact: 1.3 }); // delta = 0.3 < 0.5
    });

    expect(result.current.isPriceImpactChanged).toBe(false);
    expect(result.current.isQuoteStale).toBe(false);
  });

  it('resets the baseline when a new quote arrives (lastQuotedAtMs advances)', () => {
    const t0 = Date.now();
    const t1 = t0 + 10_000;

    const { result, rerender } = renderHook(
      ({
        currentPriceImpact,
        lastQuotedAtMs,
      }: {
        currentPriceImpact: number;
        lastQuotedAtMs: number;
      }) =>
        useStaleQuote({
          expiresAtMs: t0 + 60_000,
          currentPriceImpact,
          isStale: false,
          lastQuotedAtMs,
        }),
      { initialProps: { currentPriceImpact: 1.0, lastQuotedAtMs: t0 } },
    );

    // First baseline
    expect(result.current.baselinePriceImpact).toBe(1.0);

    // New quote arrived at t1 with a higher impact — baseline should update
    act(() => {
      rerender({ currentPriceImpact: 2.0, lastQuotedAtMs: t1 });
    });

    expect(result.current.baselinePriceImpact).toBe(2.0);
    // isPriceImpactChanged should be false because baseline was just reset
    expect(result.current.isPriceImpactChanged).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SwapButton stale_quote state tests
// ---------------------------------------------------------------------------

describe('SwapButton stale_quote state', () => {
  afterEach(() => cleanup());

  it('renders an "Update Quote" button that is enabled', () => {
    render(
      <SwapButton
        state="stale_quote"
        onSwap={() => {}}
        onUpdateQuote={() => {}}
      />,
    );

    const btn = screen.getByRole('button', { name: /update quote/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it('disables "Update Quote" while isLoading=true', () => {
    render(
      <SwapButton
        state="stale_quote"
        onSwap={() => {}}
        onUpdateQuote={() => {}}
        isLoading
      />,
    );

    expect(screen.getByRole('button', { name: /update quote/i })).toBeDisabled();
  });

  it('calls onUpdateQuote when the button is clicked', () => {
    const onUpdateQuote = vi.fn();
    render(
      <SwapButton
        state="stale_quote"
        onSwap={() => {}}
        onUpdateQuote={onUpdateQuote}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /update quote/i }));
    expect(onUpdateQuote).toHaveBeenCalledTimes(1);
  });

  it('does NOT render a "Review Swap" / Confirm button in stale_quote state', () => {
    render(
      <SwapButton
        state="stale_quote"
        onSwap={() => {}}
        onUpdateQuote={() => {}}
      />,
    );

    expect(screen.queryByRole('button', { name: /review swap/i })).toBeNull();
    expect(screen.queryByRole('button', { name: /confirm/i })).toBeNull();
  });

  it('does NOT call onSwap when in stale_quote state', () => {
    const onSwap = vi.fn();
    render(
      <SwapButton
        state="stale_quote"
        onSwap={onSwap}
        onUpdateQuote={() => {}}
      />,
    );

    // Click the Update Quote button — onSwap should never fire
    fireEvent.click(screen.getByRole('button', { name: /update quote/i }));
    expect(onSwap).not.toHaveBeenCalled();
  });
});
