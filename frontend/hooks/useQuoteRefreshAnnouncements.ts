'use client';

import { useEffect, useRef, useState } from 'react';

import {
  getQuoteRefreshErrorKey,
  shouldAnnounceQuoteRefreshFailure,
  shouldAnnounceQuoteRefreshSuccess,
  type QuoteRefreshAnnouncementState,
} from '@/lib/quote-refresh-announcements';
import type { SwapTranslationKey } from '@/lib/swap-i18n';

export interface UseQuoteRefreshAnnouncementsInput
  extends QuoteRefreshAnnouncementState {
  /** Changes when quote request inputs change; resets dedupe tracking. */
  requestKey: string;
  /** Human-readable rate summary for polite success announcements. */
  rateSummary?: string;
  t: (
    key: SwapTranslationKey,
    variables?: Record<string, string | number>,
  ) => string;
}

export interface QuoteRefreshAnnouncements {
  politeMessage: string | null;
  assertiveMessage: string | null;
}

/**
 * Derives aria-live announcement text from quote refresh state.
 * Suppresses duplicate announcements during debounce and transient retries.
 */
export function useQuoteRefreshAnnouncements(
  input: UseQuoteRefreshAnnouncementsInput,
): QuoteRefreshAnnouncements {
  const {
    canAnnounce,
    loading,
    error,
    isRecovering,
    hasPendingRetry,
    lastQuotedAtMs,
    requestKey,
    rateSummary,
    t,
  } = input;

  const [politeMessage, setPoliteMessage] = useState<string | null>(null);
  const [assertiveMessage, setAssertiveMessage] = useState<string | null>(null);
  const lastAnnouncedSuccessAtMs = useRef<number | null>(null);
  const lastAnnouncedErrorKey = useRef<string | null>(null);

  useEffect(() => {
    lastAnnouncedSuccessAtMs.current = null;
    lastAnnouncedErrorKey.current = null;
    setPoliteMessage(null);
    setAssertiveMessage(null);
  }, [requestKey]);

  useEffect(() => {
    const state: QuoteRefreshAnnouncementState = {
      canAnnounce,
      loading,
      error,
      isRecovering,
      hasPendingRetry,
      lastQuotedAtMs,
    };

    if (
      shouldAnnounceQuoteRefreshSuccess(
        state,
        lastAnnouncedSuccessAtMs.current,
      )
    ) {
      lastAnnouncedSuccessAtMs.current = lastQuotedAtMs;
      lastAnnouncedErrorKey.current = null;
      setAssertiveMessage(null);
      setPoliteMessage(
        rateSummary?.trim()
          ? t('swap.a11y.quoteRefreshed', { rate: rateSummary.trim() })
          : t('swap.a11y.quoteRefreshedGeneric'),
      );
      return;
    }

    if (!error) {
      return;
    }

    const errorKey = getQuoteRefreshErrorKey(error);
    if (
      !shouldAnnounceQuoteRefreshFailure(
        state,
        lastAnnouncedErrorKey.current,
        errorKey,
      )
    ) {
      return;
    }

    lastAnnouncedErrorKey.current = errorKey;
    setPoliteMessage(null);
    setAssertiveMessage(
      t('swap.a11y.quoteRefreshFailed', { message: error.message }),
    );
  }, [
    canAnnounce,
    loading,
    error,
    isRecovering,
    hasPendingRetry,
    lastQuotedAtMs,
    rateSummary,
    t,
  ]);

  return { politeMessage, assertiveMessage };
}
