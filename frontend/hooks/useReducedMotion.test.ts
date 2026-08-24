import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import fc from 'fast-check';
import { useReducedMotion } from './useReducedMotion';

// ---------------------------------------------------------------------------
// Helper: override window.matchMedia per-test
// ---------------------------------------------------------------------------

type ChangeHandler = (e: MediaQueryListEvent) => void;

function setReducedMotion(value: boolean) {
  const listeners: ChangeHandler[] = [];
  const mql = {
    matches: value,
    media: '(prefers-reduced-motion: reduce)',
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn((event: string, handler: ChangeHandler) => {
      if (event === 'change') listeners.push(handler);
    }),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(() => false),
    // Expose for tests that need to fire a change event
    _fire: (matches: boolean) => {
      listeners.forEach((h) => h({ matches } as MediaQueryListEvent));
    },
  };

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: () => mql,
  });

  return mql;
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

describe('useReducedMotion', () => {
  beforeEach(() => {
    // Reset to default (motion allowed) before each test
    setReducedMotion(false);
  });

  it('returns false when matchMedia is unavailable (SSR fallback)', () => {
    const original = window.matchMedia;
    // @ts-expect-error intentionally removing matchMedia
    delete window.matchMedia;

    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);

    // Restore
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      configurable: true,
      value: original,
    });
  });

  it('returns true when matchMedia reports matches: true on initial render', () => {
    setReducedMotion(true);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);
  });

  it('returns false when matchMedia reports matches: false on initial render', () => {
    setReducedMotion(false);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
  });

  it('updates to true when a change event fires with matches: true', () => {
    const mql = setReducedMotion(false);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);

    act(() => {
      mql._fire(true);
    });

    expect(result.current).toBe(true);
  });

  it('updates to false when a change event fires with matches: false', () => {
    const mql = setReducedMotion(true);
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);

    act(() => {
      mql._fire(false);
    });

    expect(result.current).toBe(false);
  });

  it('calls removeEventListener on unmount (listener cleanup)', () => {
    const mql = setReducedMotion(false);
    const { unmount } = renderHook(() => useReducedMotion());

    unmount();

    expect(mql.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });
});

// ---------------------------------------------------------------------------
// Property-based tests
// ---------------------------------------------------------------------------

describe('useReducedMotion — property tests', () => {
  afterEach(() => {
    // Restore default mock after each property run
    setReducedMotion(false);
  });

  it(
    // Feature: reduced-motion-swap-animations, Property 1: Hook reflects matchMedia state
    'Property 1: for any boolean b, hook returns b when matchMedia.matches is b',
    () => {
      fc.assert(
        fc.property(fc.boolean(), (matches) => {
          setReducedMotion(matches);
          const { result, unmount } = renderHook(() => useReducedMotion());
          const value = result.current;
          unmount();
          return value === matches;
        }),
        { numRuns: 100 }
      );
    }
  );

  it(
    // Feature: reduced-motion-swap-animations, Property 2: Hook reacts to runtime preference changes
    'Property 2: for any boolean b emitted by a change event, hook returns b after the event',
    () => {
      fc.assert(
        fc.property(fc.boolean(), fc.boolean(), (initial, emitted) => {
          const mql = setReducedMotion(initial);
          const { result, unmount } = renderHook(() => useReducedMotion());

          act(() => {
            mql._fire(emitted);
          });

          const value = result.current;
          unmount();
          return value === emitted;
        }),
        { numRuns: 100 }
      );
    }
  );
});
