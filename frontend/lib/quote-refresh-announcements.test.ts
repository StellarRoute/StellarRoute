import { describe, expect, it } from 'vitest';

import {
  isHardQuoteRefreshFailure,
  shouldAnnounceQuoteRefreshFailure,
  shouldAnnounceQuoteRefreshSuccess,
} from './quote-refresh-announcements';

describe('quote-refresh-announcements', () => {
  const baseState = {
    canAnnounce: true,
    loading: false,
    error: null,
    isRecovering: false,
    hasPendingRetry: false,
    lastQuotedAtMs: 1_000,
  };

  it('detects hard failures after retries stop', () => {
    expect(
      isHardQuoteRefreshFailure({
        loading: false,
        error: new Error('boom'),
        isRecovering: false,
        hasPendingRetry: false,
      }),
    ).toBe(true);
  });

  it('does not treat transient retries as hard failures', () => {
    expect(
      isHardQuoteRefreshFailure({
        loading: false,
        error: new Error('boom'),
        isRecovering: true,
        hasPendingRetry: true,
      }),
    ).toBe(false);
  });

  it('announces success only once per completed quote timestamp', () => {
    expect(
      shouldAnnounceQuoteRefreshSuccess(baseState, null),
    ).toBe(true);
    expect(
      shouldAnnounceQuoteRefreshSuccess(baseState, baseState.lastQuotedAtMs),
    ).toBe(false);
  });

  it('skips success announcements while loading', () => {
    expect(
      shouldAnnounceQuoteRefreshSuccess(
        { ...baseState, loading: true },
        null,
      ),
    ).toBe(false);
  });

  it('announces hard failures once per error message', () => {
    const error = new Error('Invalid amount');
    const state = {
      ...baseState,
      lastQuotedAtMs: null,
      error,
    };

    expect(
      shouldAnnounceQuoteRefreshFailure(state, null, error.message),
    ).toBe(true);
    expect(
      shouldAnnounceQuoteRefreshFailure(state, error.message, error.message),
    ).toBe(false);
  });
});
