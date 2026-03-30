'use client';

/**
 * User-friendly banner shown when the API returns 429 (rate limited).
 *
 * Displays a countdown timer and a manual "Retry Now" button.
 */

import { Clock, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';

export interface RateLimitBannerProps {
  /** Seconds remaining until auto-retry. */
  secondsRemaining: number;
  /** Callback to trigger an immediate retry. */
  retry: () => void;
}

export function RateLimitBanner({
  secondsRemaining,
  retry,
}: RateLimitBannerProps) {
  return (
    <div
      role="alert"
      className="flex flex-col items-center gap-3 rounded-2xl border border-amber-500/30 bg-amber-500/5 p-4 text-center backdrop-blur-sm animate-in fade-in slide-in-from-bottom-2 duration-300"
    >
      {/* Icon */}
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-amber-500/10">
        <Clock className="h-5 w-5 text-amber-500" aria-hidden="true" />
      </div>

      {/* Message */}
      <div className="space-y-1">
        <h3 className="text-sm font-semibold text-foreground">
          Rate limit reached
        </h3>
        <p className="text-xs text-muted-foreground">
          {secondsRemaining > 0
            ? `Too many requests — retrying automatically in ${secondsRemaining}s`
            : 'Too many requests — please wait a moment and try again'}
        </p>
      </div>

      {/* Countdown progress ring */}
      {secondsRemaining > 0 && (
        <div className="relative flex items-center justify-center" aria-hidden="true">
          <svg className="h-8 w-8 -rotate-90" viewBox="0 0 36 36">
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              className="text-amber-500/20"
              strokeWidth="3"
            />
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              className="text-amber-500 transition-all duration-1000 ease-linear"
              strokeWidth="3"
              strokeDasharray="87.96"
              strokeDashoffset={87.96 * (1 - secondsRemaining / 10)}
              strokeLinecap="round"
            />
          </svg>
          <span className="absolute text-[10px] font-bold text-amber-500">
            {secondsRemaining}
          </span>
        </div>
      )}

      {/* Manual retry */}
      <Button
        variant="outline"
        size="sm"
        onClick={retry}
        className="gap-1.5 rounded-xl border-amber-500/30 text-amber-600 hover:bg-amber-500/10 hover:text-amber-500"
      >
        <RefreshCw className="h-3.5 w-3.5" />
        Retry Now
      </Button>
    </div>
  );
}
