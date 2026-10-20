'use client';

import { useEffect, useState } from 'react';
import { Clock, RefreshCw } from 'lucide-react';

import { CARD_FX_TTL_SECONDS } from '@/lib/card/fx';

const RING_RADIUS = 15;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

interface CardFxFreshnessBadgeProps {
  /** Indicative rate label, e.g. "1 USDC = 1538.46 NGN". */
  rate: string;
  /** Timestamp of the quote the badge is tracking. Changing it restarts the countdown. */
  retrievedAtMs: number;
  onRefresh: () => void;
  ttlSeconds?: number;
  className?: string;
}

/**
 * Freshness indicator for the card dashboard's indicative FX rate.
 * Counts down a TTL ring, dims the rate and turns amber once the quote is
 * stale, and exposes a manual "Refresh Rate" action.
 */
export function CardFxFreshnessBadge({
  rate,
  retrievedAtMs,
  onRefresh,
  ttlSeconds = CARD_FX_TTL_SECONDS,
  className = '',
}: CardFxFreshnessBadgeProps) {
  const [remainingSeconds, setRemainingSeconds] = useState(ttlSeconds);
  const isExpired = remainingSeconds <= 0;
  const progress = Math.min(1, Math.max(0, remainingSeconds / ttlSeconds));

  useEffect(() => {
    setRemainingSeconds(ttlSeconds);
  }, [retrievedAtMs, ttlSeconds]);

  useEffect(() => {
    let intervalId: ReturnType<typeof setInterval> | null = null;

    const stop = () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    // Ticks are stopped while the tab is backgrounded so an expired countdown
    // costs no CPU cycles until the cardholder comes back.
    const sync = () => {
      if (document.visibilityState === 'hidden') {
        stop();
        return;
      }
      if (intervalId === null) {
        intervalId = setInterval(() => {
          setRemainingSeconds((seconds) => Math.max(0, seconds - 1));
        }, 1000);
      }
    };

    sync();
    document.addEventListener('visibilitychange', sync);

    return () => {
      stop();
      document.removeEventListener('visibilitychange', sync);
    };
  }, []);

  const handleRefresh = () => {
    setRemainingSeconds(ttlSeconds);
    onRefresh();
  };

  return (
    <div
      data-testid="card-fx-freshness-badge"
      className={`
        rounded-xl border p-4 transition-colors duration-200
        ${
          isExpired
            ? 'border-amber-300 bg-amber-50 dark:border-amber-800 dark:bg-amber-950/40'
            : 'border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950'
        }
        ${className}
      `}
    >
      <div className="flex items-center gap-3">
        <svg
          viewBox="0 0 36 36"
          className="h-9 w-9 -rotate-90"
          aria-hidden="true"
          data-testid="card-fx-countdown-ring"
        >
          <circle
            cx="18"
            cy="18"
            r={RING_RADIUS}
            fill="none"
            strokeWidth="3"
            className="stroke-slate-200 dark:stroke-slate-700"
          />
          <circle
            cx="18"
            cy="18"
            r={RING_RADIUS}
            fill="none"
            strokeWidth="3"
            strokeLinecap="round"
            strokeDasharray={RING_CIRCUMFERENCE}
            strokeDashoffset={RING_CIRCUMFERENCE * (1 - progress)}
            className={
              isExpired ? 'stroke-amber-500' : 'stroke-emerald-500'
            }
          />
        </svg>

        <div className="min-w-0">
          <p
            data-testid="card-fx-indicative-rate"
            className={`
              truncate text-sm font-semibold
              ${
                isExpired
                  ? 'text-amber-700 opacity-50 dark:text-amber-300'
                  : 'text-slate-900 dark:text-slate-50'
              }
            `}
          >
            {rate}
          </p>
          <span
            data-testid="card-fx-indicative-disclaimer"
            className="mt-1 inline-flex items-center gap-1 rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-medium text-slate-600 dark:bg-slate-800 dark:text-slate-300"
          >
            indicative: true
          </span>
        </div>
      </div>

      <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
        <p
          className={`
            inline-flex items-center gap-1.5 text-xs font-medium
            ${
              isExpired
                ? 'text-amber-700 dark:text-amber-300'
                : 'text-slate-500 dark:text-slate-400'
            }
          `}
        >
          <Clock className="h-3.5 w-3.5" aria-hidden="true" />
          <span data-testid="card-fx-countdown-label">
            {isExpired
              ? 'Quote Stale — click to update'
              : `Quote valid for ${remainingSeconds}s`}
          </span>
        </p>

        <button
          type="button"
          onClick={handleRefresh}
          className={`
            inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs font-medium transition
            ${
              isExpired
                ? 'bg-amber-600 text-white hover:bg-amber-500'
                : 'border border-slate-300 bg-white text-slate-700 hover:border-slate-400 hover:text-slate-900 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:border-slate-500 dark:hover:text-slate-100'
            }
          `}
        >
          <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" />
          Refresh Rate
        </button>
      </div>

      {isExpired && (
        <p role="status" aria-live="polite" className="sr-only">
          Indicative rate expired. Refresh to update.
        </p>
      )}
    </div>
  );
}

export default CardFxFreshnessBadge;
