import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useQuoteRefreshAnnouncements } from './useQuoteRefreshAnnouncements';
import { createSwapTranslator } from '@/lib/swap-i18n';

const { t } = createSwapTranslator('en-US');

describe('useQuoteRefreshAnnouncements', () => {
  it('emits a polite message after a successful refresh', async () => {
    const { result, rerender } = renderHook(
      (props) => useQuoteRefreshAnnouncements(props),
      {
        initialProps: {
          canAnnounce: true,
          loading: true,
          error: null,
          isRecovering: false,
          hasPendingRetry: false,
          lastQuotedAtMs: null,
          requestKey: 'native|USDC|100|sell',
          rateSummary: '1 XLM = 0.12 USDC',
          t,
        },
      },
    );

    expect(result.current.politeMessage).toBeNull();

    rerender({
      canAnnounce: true,
      loading: false,
      error: null,
      isRecovering: false,
      hasPendingRetry: false,
      lastQuotedAtMs: 1_700_000_000_000,
      requestKey: 'native|USDC|100|sell',
      rateSummary: '1 XLM = 0.12 USDC',
      t,
    });

    await waitFor(() => {
      expect(result.current.politeMessage).toBe(
        'Quote updated. 1 XLM = 0.12 USDC',
      );
      expect(result.current.assertiveMessage).toBeNull();
    });
  });

  it('does not duplicate success announcements for the same quote timestamp', async () => {
    const { result, rerender } = renderHook(
      (props) => useQuoteRefreshAnnouncements(props),
      {
        initialProps: {
          canAnnounce: true,
          loading: false,
          error: null,
          isRecovering: false,
          hasPendingRetry: false,
          lastQuotedAtMs: 42,
          requestKey: 'native|USDC|100|sell',
          rateSummary: '1 XLM = 0.12 USDC',
          t,
        },
      },
    );

    await waitFor(() => {
      expect(result.current.politeMessage).toContain('Quote updated');
    });

    const firstMessage = result.current.politeMessage;
    rerender({
      canAnnounce: true,
      loading: true,
      error: null,
      isRecovering: false,
      hasPendingRetry: false,
      lastQuotedAtMs: 42,
      requestKey: 'native|USDC|100|sell',
      rateSummary: '1 XLM = 0.12 USDC',
      t,
    });

    rerender({
      canAnnounce: true,
      loading: false,
      error: null,
      isRecovering: false,
      hasPendingRetry: false,
      lastQuotedAtMs: 42,
      requestKey: 'native|USDC|100|sell',
      rateSummary: '1 XLM = 0.12 USDC',
      t,
    });

    expect(result.current.politeMessage).toBe(firstMessage);
  });

  it('emits an assertive message on hard failures only', async () => {
    const { result, rerender } = renderHook(
      (props) => useQuoteRefreshAnnouncements(props),
      {
        initialProps: {
          canAnnounce: true,
          loading: false,
          error: new Error('Invalid amount'),
          isRecovering: true,
          hasPendingRetry: true,
          lastQuotedAtMs: null,
          requestKey: 'native|USDC|100|sell',
          t,
        },
      },
    );

    expect(result.current.assertiveMessage).toBeNull();

    rerender({
      canAnnounce: true,
      loading: false,
      error: new Error('Invalid amount'),
      isRecovering: false,
      hasPendingRetry: false,
      lastQuotedAtMs: null,
      requestKey: 'native|USDC|100|sell',
      t,
    });

    await waitFor(() => {
      expect(result.current.assertiveMessage).toBe(
        'Quote refresh failed. Invalid amount',
      );
      expect(result.current.politeMessage).toBeNull();
    });
  });

  it('resets dedupe when the request key changes during debounce', async () => {
    const { result, rerender } = renderHook(
      (props) => useQuoteRefreshAnnouncements(props),
      {
        initialProps: {
          canAnnounce: true,
          loading: false,
          error: null,
          isRecovering: false,
          hasPendingRetry: false,
          lastQuotedAtMs: 10,
          requestKey: 'native|USDC|100|sell',
          rateSummary: '1 XLM = 0.10 USDC',
          t,
        },
      },
    );

    await waitFor(() => {
      expect(result.current.politeMessage).toContain('Quote updated');
    });

    rerender({
      canAnnounce: true,
      loading: false,
      error: null,
      isRecovering: false,
      hasPendingRetry: false,
      lastQuotedAtMs: 20,
      requestKey: 'native|USDC|200|sell',
      rateSummary: '1 XLM = 0.20 USDC',
      t,
    });

    await waitFor(() => {
      expect(result.current.politeMessage).toBe(
        'Quote updated. 1 XLM = 0.20 USDC',
      );
    });
  });
});
