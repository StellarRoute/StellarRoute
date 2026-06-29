/**
 * Screen reader announcement helpers for quote refresh outcomes.
 */

export type QuoteRefreshAnnouncementKind = 'success' | 'hard_failure';

export interface QuoteRefreshAnnouncementState {
  /** When false, suppress all announcements (invalid or missing inputs). */
  canAnnounce: boolean;
  loading: boolean;
  error: Error | null;
  isRecovering: boolean;
  hasPendingRetry: boolean;
  lastQuotedAtMs: number | null;
}

export function isHardQuoteRefreshFailure(
  state: Pick<
    QuoteRefreshAnnouncementState,
    'loading' | 'error' | 'isRecovering' | 'hasPendingRetry'
  >,
): boolean {
  return (
    state.error !== null &&
    !state.loading &&
    !state.isRecovering &&
    !state.hasPendingRetry
  );
}

export function shouldAnnounceQuoteRefreshSuccess(
  state: QuoteRefreshAnnouncementState,
  lastAnnouncedSuccessAtMs: number | null,
): boolean {
  return (
    state.canAnnounce &&
    !state.loading &&
    state.error === null &&
    state.lastQuotedAtMs !== null &&
    state.lastQuotedAtMs !== lastAnnouncedSuccessAtMs
  );
}

export function shouldAnnounceQuoteRefreshFailure(
  state: QuoteRefreshAnnouncementState,
  lastAnnouncedErrorKey: string | null,
  errorKey: string,
): boolean {
  return (
    state.canAnnounce &&
    isHardQuoteRefreshFailure(state) &&
    errorKey !== lastAnnouncedErrorKey
  );
}

export function getQuoteRefreshErrorKey(error: Error): string {
  return error.message;
}
