/**
 * CardDailyLimitProgress (CARD-49)
 *
 * Displays a visual progress bar for the user's rolling 24-hour card spending
 * relative to their configured daily limit.
 *
 * Colour semantics:
 *  - normal  : < 80 % utilisation — neutral slate bar
 *  - amber   : ≥ 80 % utilisation — amber warning bar
 *  - red     : 100 % utilisation  — red "limit reached" bar + alert banner
 *
 * The component is purely presentational.  Spending calculations are delegated
 * to {@link calculateDailySpendingProgress} in `lib/card/limits.ts`.
 */

import type { DailySpendingProgress } from '@/lib/card/limits';

export interface CardDailyLimitProgressProps {
  /** Pre-computed spending progress from `calculateDailySpendingProgress`. */
  progress: DailySpendingProgress;
  /**
   * Optional CSS class applied to the outermost element.
   * Use for spacing / layout overrides from the parent.
   */
  className?: string;
}

/** Map from colour state to Tailwind bar-fill classes. */
const BAR_COLOR_CLASSES: Record<DailySpendingProgress['color'], string> = {
  normal: 'bg-slate-600 dark:bg-slate-400',
  amber: 'bg-amber-500 dark:bg-amber-400',
  red: 'bg-red-600 dark:bg-red-500',
};

/** Map from colour state to Tailwind label text classes. */
const LABEL_COLOR_CLASSES: Record<DailySpendingProgress['color'], string> = {
  normal: 'text-slate-700 dark:text-slate-300',
  amber: 'text-amber-700 dark:text-amber-300',
  red: 'text-red-700 dark:text-red-400',
};

/**
 * Formats a USDC amount to two decimal places with a `$` prefix, e.g.
 * `formatUsdc(42.5)` → `"$42.50"`.
 */
function formatUsdc(value: number): string {
  return `$${value.toFixed(2)}`;
}

/**
 * CardDailyLimitProgress renders the daily spending meter.
 *
 * @example
 * ```tsx
 * const progress = calculateDailySpendingProgress(transactions, 500);
 * <CardDailyLimitProgress progress={progress} />
 * ```
 */
export function CardDailyLimitProgress({
  progress,
  className = '',
}: CardDailyLimitProgressProps) {
  const { spentUsdc, dailyLimitUsdc, percentUsed, color } = progress;

  const barFillClass = BAR_COLOR_CLASSES[color];
  const labelClass = LABEL_COLOR_CLASSES[color];

  /** Tooltip text shown on the progress track (midnight UTC reset). */
  const tooltipText = 'Spending limit resets at midnight UTC';

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      {/* ── Header row: label + amounts ──────────────────────────────── */}
      <div className="flex items-baseline justify-between gap-2">
        <span className={`text-sm font-medium ${labelClass}`}>
          Daily Spending Limit
        </span>
        <span
          className={`text-xs tabular-nums ${labelClass}`}
          aria-label={`${formatUsdc(spentUsdc)} spent of ${formatUsdc(dailyLimitUsdc)} daily limit`}
        >
          {formatUsdc(spentUsdc)}{' '}
          <span className="text-slate-400 dark:text-slate-500">of</span>{' '}
          {formatUsdc(dailyLimitUsdc)} daily limit
        </span>
      </div>

      {/* ── Progress track ────────────────────────────────────────────── */}
      <div
        role="progressbar"
        aria-valuenow={Math.round(percentUsed)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Daily spending limit progress"
        title={tooltipText}
        className="relative h-2 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800"
      >
        <div
          className={`h-full rounded-full transition-[width] duration-300 ease-out ${barFillClass}`}
          style={{ width: `${percentUsed}%` }}
        />
      </div>

      {/* ── Percentage label + reset tooltip hint ────────────────────── */}
      <div className="flex items-center justify-between gap-2">
        <span className={`text-xs tabular-nums ${labelClass}`}>
          {percentUsed.toFixed(1)}% used
        </span>
        <span
          className="text-xs text-slate-400 dark:text-slate-500"
          title={tooltipText}
        >
          Resets at midnight UTC
        </span>
      </div>

      {/* ── "Daily Limit Reached" alert banner (100 % only) ─────────── */}
      {color === 'red' && (
        <div
          role="alert"
          className="mt-1 flex items-center gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300"
        >
          <span aria-hidden="true">⚠</span>
          Daily Limit Reached — new card purchases will be declined until the
          limit resets at midnight UTC.
        </div>
      )}
    </div>
  );
}
