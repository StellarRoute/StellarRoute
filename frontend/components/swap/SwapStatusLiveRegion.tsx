'use client';

export type SwapStatus = 'idle' | 'pending' | 'finalized' | 'error';

export interface SwapStatusLiveRegionProps {
  /** Current lifecycle state of the swap submission. */
  status: SwapStatus;
  /** Optional detail appended to the announcement (tx hash, error copy, rate). */
  detail?: string | null;
  /** Keyboard-reachable confirm action. Omitted when the swap is not confirmable. */
  onConfirm?: () => void;
  /** Keyboard-reachable quote refresh action. */
  onUpdateQuote?: () => void;
}

/**
 * Announces swap submission state to screen readers.
 *
 * `pending` and `finalized` are polite (`role="status"`) so they never interrupt
 * the user mid-word; `error` is assertive (`role="alert"`) because it blocks the
 * flow. Announcements never move focus — the confirm / update-quote controls stay
 * in the natural tab order so a keyboard-only user can act on what they just heard.
 */
export function SwapStatusLiveRegion({
  status,
  detail,
  onConfirm,
  onUpdateQuote,
}: SwapStatusLiveRegionProps) {
  const suffix = detail ? ` ${detail}` : '';

  const politeMessage =
    status === 'pending'
      ? `Swap submitted. Waiting for confirmation.${suffix}`
      : status === 'finalized'
        ? `Swap confirmed.${suffix}`
        : '';

  const assertiveMessage =
    status === 'error' ? `Swap failed.${suffix} Update the quote and try again.` : '';

  return (
    <>
      <div role="status" aria-live="polite" aria-atomic="true" className="sr-only">
        {politeMessage}
      </div>
      <div role="alert" aria-live="assertive" aria-atomic="true" className="sr-only">
        {assertiveMessage}
      </div>

      {(onUpdateQuote || onConfirm) && (
        <div className="flex gap-2" aria-label="Swap actions" role="group">
          {onUpdateQuote && (
            <button
              type="button"
              onClick={onUpdateQuote}
              aria-describedby="swap-status-hint"
              disabled={status === 'pending'}
            >
              Update quote
            </button>
          )}
          {onConfirm && (
            <button
              type="button"
              onClick={onConfirm}
              aria-describedby="swap-status-hint"
              disabled={status === 'pending'}
            >
              Confirm swap
            </button>
          )}
          <span id="swap-status-hint" className="sr-only">
            Swap status: {status}
          </span>
        </div>
      )}
    </>
  );
}
