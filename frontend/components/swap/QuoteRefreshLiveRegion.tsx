'use client';

import type { QuoteRefreshAnnouncements } from '@/hooks/useQuoteRefreshAnnouncements';

export type QuoteRefreshLiveRegionProps = QuoteRefreshAnnouncements;

/**
 * Visually hidden aria-live regions for quote refresh outcomes.
 * Does not move focus — announcements are read in the background.
 */
export function QuoteRefreshLiveRegion({
  politeMessage,
  assertiveMessage,
}: QuoteRefreshLiveRegionProps) {
  return (
    <>
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {politeMessage ?? ''}
      </div>
      <div
        role="alert"
        aria-live="assertive"
        aria-atomic="true"
        className="sr-only"
      >
        {assertiveMessage ?? ''}
      </div>
    </>
  );
}
