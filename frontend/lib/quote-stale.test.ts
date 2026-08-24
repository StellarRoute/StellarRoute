import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  isQuoteStale,
  QUOTE_STALE_AFTER_MS,
} from './quote-stale';

describe('isQuoteStale', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-01T00:00:00.000Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns false when there is no prior successful quote', () => {
    expect(isQuoteStale(null, Date.now(), QUOTE_STALE_AFTER_MS)).toBe(false);
  });

  it('returns false immediately after a successful quote', () => {
    const t = Date.now();
    expect(isQuoteStale(t, t, QUOTE_STALE_AFTER_MS)).toBe(false);
  });

  it('returns false just before the stale threshold', () => {
    const last = Date.now();
    vi.advanceTimersByTime(QUOTE_STALE_AFTER_MS - 1);
    expect(isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS)).toBe(false);
  });

  it('returns true at and after the stale threshold', () => {
    const last = Date.now();
    vi.advanceTimersByTime(QUOTE_STALE_AFTER_MS);
    expect(isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS)).toBe(true);

    vi.advanceTimersByTime(60_000);
    expect(isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS)).toBe(true);
  });

  it('respects a custom stale window', () => {
    const last = Date.now();
    vi.advanceTimersByTime(999);
    expect(isQuoteStale(last, Date.now(), 1000)).toBe(false);
    vi.advanceTimersByTime(1);
    expect(isQuoteStale(last, Date.now(), 1000)).toBe(true);
  });

  it('does not let a short cache expiresAt mark stale before the client window', () => {
    const last = Date.now();
    const expiresAt = last + 2_000;
    expect(isQuoteStale(last, last + 1_000, QUOTE_STALE_AFTER_MS, expiresAt)).toBe(
      false,
    );
    // Cache TTL elapsed, but client floor has not — still fresh for CTA.
    expect(isQuoteStale(last, expiresAt, QUOTE_STALE_AFTER_MS, expiresAt)).toBe(
      false,
    );
    expect(
      isQuoteStale(
        last,
        last + QUOTE_STALE_AFTER_MS,
        QUOTE_STALE_AFTER_MS,
        expiresAt,
      ),
    ).toBe(true);
  });

  it('honors a longer server expiresAt beyond the client window', () => {
    const last = Date.now();
    const expiresAt = last + 30_000;
    vi.advanceTimersByTime(QUOTE_STALE_AFTER_MS);
    expect(isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS, expiresAt)).toBe(
      false,
    );
    vi.advanceTimersByTime(30_000 - QUOTE_STALE_AFTER_MS);
    expect(isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS, expiresAt)).toBe(
      true,
    );
  });

  it('ignores already-expired expiresAt and falls back to client window', () => {
    const last = Date.now();
    const expiredAt = last - 60_000;
    expect(
      isQuoteStale(last, last + 1_000, QUOTE_STALE_AFTER_MS, expiredAt),
    ).toBe(false);
    vi.advanceTimersByTime(QUOTE_STALE_AFTER_MS);
    expect(
      isQuoteStale(last, Date.now(), QUOTE_STALE_AFTER_MS, expiredAt),
    ).toBe(true);
  });
});
