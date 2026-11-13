import { act, render, renderHook, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  SwapWarningCenter,
  useSwapWarningCenter,
  type SwapWarning,
} from './SwapWarningCenter';

const STORAGE_KEY = 'stellarroute_swap_warnings';

describe('SwapWarningCenter & useSwapWarningCenter', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('loads valid recent warnings from localStorage and drops expired entries', () => {
    const now = Date.now();
    const validRecent: SwapWarning = {
      id: 'w-recent',
      type: 'warning',
      title: 'High slippage',
      message: 'Pool depth is thin',
      timestamp: now - 1_000,
    };
    const expired: SwapWarning = {
      id: 'w-expired',
      type: 'error',
      title: 'Old error',
      message: 'More than 24h ago',
      timestamp: now - 25 * 60 * 60 * 1_000,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify([validRecent, expired]));

    const { result } = renderHook(() => useSwapWarningCenter());
    expect(result.current.warnings).toEqual([validRecent]);
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull();
  });

  it('falls back to empty list and removes key when stored JSON is not an array', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ id: 'not-an-array' }));

    const { result } = renderHook(() => useSwapWarningCenter());
    expect(result.current.warnings).toEqual([]);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('falls back to empty list and removes key when array contains wrong-shaped items', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify([{ id: 123, type: 'invalid', timestamp: 'bad' }]),
    );

    const { container } = render(<SwapWarningCenter />);
    expect(container.firstChild).toBeNull();
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('falls back to empty list and removes key when stored value is malformed JSON', () => {
    localStorage.setItem(STORAGE_KEY, '<<<corrupt-json');

    const { result } = renderHook(() => useSwapWarningCenter());
    expect(result.current.warnings).toEqual([]);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('adds and clears warnings cleanly after recovering from corrupt storage', () => {
    localStorage.setItem(STORAGE_KEY, '{}');

    const { result } = renderHook(() => useSwapWarningCenter());
    expect(result.current.warnings).toEqual([]);

    act(() => {
      result.current.addWarning({
        type: 'error',
        title: 'Route unavailable',
        message: 'No liquidity path found',
      });
    });

    expect(result.current.warnings).toHaveLength(1);
    render(<SwapWarningCenter warnings={result.current.warnings} />);
    expect(screen.getByText(/Recent Swap Issues \(1\)/)).toBeInTheDocument();
  });
});
