'use client';

import { useEffect, useState } from 'react';
import { AlertTriangle, X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import {
  DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  DEFAULT_CARD_TOP_UP_USDC,
  shouldShowLowBalanceAlert,
} from '@/lib/card/preferences';

export const CARD_BALANCE_ALERT_DISMISS_STORAGE_PREFIX =
  'stellarroute:card:balance-alert-dismissed';

/** Session scope: the dismissal is dropped on reload by design. */
export function cardBalanceAlertDismissKey(
  stellarAddress: string | null | undefined,
  thresholdUsdc: number,
): string {
  return `${CARD_BALANCE_ALERT_DISMISS_STORAGE_PREFIX}:${stellarAddress ?? 'anonymous'}:${thresholdUsdc}`;
}

export interface CardBalanceAlertProps {
  /** Available USDC for the active cardholder; null/undefined = unknown. */
  availableUsdc: number | null | undefined;
  /** Warn strictly below this; defaults to the shipped 25 USDC threshold. */
  thresholdUsdc?: number;
  /** Amount the top-up flow pre-fills; defaults to 50 USDC. */
  recommendedUsdc?: number;
  /** Session-scoping key (address + threshold) for the dismiss state. */
  dismissKey?: string;
  onTopUp?: (recommendedUsdc: number) => void;
  onDismiss?: () => void;
  className?: string;
}

function formatUsdc(value: number): string {
  return value.toFixed(2);
}

function readSessionFlag(key: string): boolean {
  if (typeof window === 'undefined') return false;
  try {
    return window.sessionStorage.getItem(key) === 'true';
  } catch {
    return false;
  }
}

function writeSessionFlag(key: string, value: string): void {
  if (typeof window === 'undefined') return;
  try {
    window.sessionStorage.setItem(key, value);
  } catch {
    // Dismiss is a convenience; a blocked storage API must not throw.
  }
}

/**
 * CARD-45 - Low balance warning with a 1-click bridge top-up trigger.
 *
 * Point-of-sale declines happen when available USDC can't cover a merchant
 * authorization, so this banner surfaces the shortfall before the user taps
 * their card. "Top Up Now" hands the recommended amount to the card top-up
 * modal; the X only hides the banner for the current session, so a reload
 * re-alerts a still-low balance. Renders nothing at or above the threshold
 * and nothing while the balance is unknown.
 */
export function CardBalanceAlert({
  availableUsdc,
  thresholdUsdc = DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  recommendedUsdc = DEFAULT_CARD_TOP_UP_USDC,
  dismissKey,
  onTopUp,
  onDismiss,
  className,
}: CardBalanceAlertProps) {
  const [dismissed, setDismissed] = useState(() =>
    dismissKey ? readSessionFlag(dismissKey) : false,
  );

  useEffect(() => {
    setDismissed(dismissKey ? readSessionFlag(dismissKey) : false);
  }, [dismissKey]);

  if (!shouldShowLowBalanceAlert(availableUsdc, thresholdUsdc)) return null;
  if (dismissed) return null;

  const handleDismiss = () => {
    if (dismissKey) writeSessionFlag(dismissKey, 'true');
    setDismissed(true);
    onDismiss?.();
  };

  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        'flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-200 bg-amber-50 p-4',
        'dark:border-amber-900 dark:bg-amber-950/40',
        className,
      )}
      data-testid="card-balance-alert"
    >
      <div className="flex min-w-0 items-start gap-3">
        <AlertTriangle
          className="mt-0.5 h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400"
          aria-hidden
        />
        <div className="text-sm text-amber-800 dark:text-amber-200">
          <p className="font-semibold" data-testid="card-balance-alert-heading">
            Low balance — top up before your next tap
          </p>
          <p className="mt-1 text-pretty">
            {formatUsdc(availableUsdc as number)} USDC available, below the{' '}
            {formatUsdc(thresholdUsdc)} USDC alert threshold. Merchant payments
            may be declined. A bridge top-up of{' '}
            {formatUsdc(recommendedUsdc)} USDC is ready when you are.
          </p>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <Button
          type="button"
          size="sm"
          className="min-h-9"
          onClick={() => onTopUp?.(recommendedUsdc)}
          data-testid="card-balance-alert-top-up"
        >
          Top Up Now
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Dismiss low balance alert"
          onClick={handleDismiss}
          data-testid="card-balance-alert-dismiss"
        >
          <X className="h-4 w-4 text-amber-700 dark:text-amber-300" aria-hidden />
        </Button>
      </div>
    </div>
  );
}
