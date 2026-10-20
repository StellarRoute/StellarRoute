// Card limit utilities — spending cap enforcement and daily progress tracking.

export interface CardLimitApplication {
  programMax: number;
  userCap?: number | null;
  holds?: number;
}

export interface CardLimitUpdate {
  application: CardLimitApplication;
  nextUserCap: number | null | undefined;
}

export function normalizeCardLimit(value: number | null | undefined): number {
  if (value === null || value === undefined || Number.isNaN(Number(value))) {
    return 0;
  }

  return Number(value);
}

export function getEffectiveUserCap(application: Pick<CardLimitApplication, 'programMax' | 'userCap'>): number {
  const programMax = normalizeCardLimit(application.programMax);
  const userCap = normalizeCardLimit(application.userCap);

  return Math.min(userCap, programMax);
}

export function isUserCapWithinProgramMax(
  application: Pick<CardLimitApplication, 'programMax' | 'userCap'>,
): boolean {
  const programMax = normalizeCardLimit(application.programMax);
  const userCap = normalizeCardLimit(application.userCap);

  return userCap <= programMax;
}

export function applyUserCap(
  application: CardLimitApplication,
  nextUserCap: number | null | undefined,
): CardLimitApplication {
  const programMax = normalizeCardLimit(application.programMax);
  const nextCap = normalizeCardLimit(nextUserCap);

  if (nextUserCap !== undefined && nextUserCap !== null && nextCap > programMax) {
    throw new Error('User cap cannot exceed the program max.');
  }

  return {
    ...application,
    userCap: nextUserCap === undefined || nextUserCap === null ? application.userCap : nextCap,
    holds: application.holds ?? 0,
  };
}

export function getCardLimitStatus(application: CardLimitApplication): {
  effectiveCap: number;
  userCap: number;
  programMax: number;
  isWithinProgramMax: boolean;
} {
  const programMax = normalizeCardLimit(application.programMax);
  const userCap = normalizeCardLimit(application.userCap);
  const effectiveCap = getEffectiveUserCap(application);

  return {
    effectiveCap,
    userCap,
    programMax,
    isWithinProgramMax: userCap <= programMax,
  };
}

export function lowerUserCapWithoutChangingHolds(
  application: CardLimitApplication,
  nextUserCap: number | null | undefined,
): CardLimitApplication {
  const updated = applyUserCap(application, nextUserCap);

  return {
    ...updated,
    holds: application.holds ?? 0,
  };
}

// ── Daily Spending Limit (CARD-49) ────────────────────────────────────────────

/**
 * A settled merchant debit transaction from the card activity feed.
 * Only `pos_debit` entries with `state === 'settled'` count toward the
 * rolling 24-hour spending window.
 */
export interface DailySpendingTransaction {
  /** Transaction type.  Only `'pos_debit'` debits are counted. */
  type: string;
  /** Lifecycle state.  Only `'settled'` transactions are counted. */
  state: string;
  /** Settlement timestamp (ISO-8601 or any value accepted by `Date`). */
  settledAt: string | number | Date;
  /** Debited amount in USDC, expressed in stroops (1 USDC = 10 000 000 stroops). */
  amount_stroops: number;
}

/** Utilisation colour returned by {@link calculateDailySpendingProgress}. */
export type DailyLimitColor = 'normal' | 'amber' | 'red';

/** Result returned by {@link calculateDailySpendingProgress}. */
export interface DailySpendingProgress {
  /** Total USDC spent (settled pos_debit) in the rolling 24-hour window. */
  spentUsdc: number;
  /** Configured daily limit in USDC. */
  dailyLimitUsdc: number;
  /**
   * Percentage of the daily limit consumed (0–100).
   * Clamped to 100 even when spent exceeds the limit.
   */
  percentUsed: number;
  /** Colour state: normal (<80 %), amber (80–99 %), red (100 %). */
  color: DailyLimitColor;
}

/** Threshold above which the meter turns amber (inclusive). */
const AMBER_THRESHOLD = 80;

/**
 * Calculates rolling 24-hour spending progress from a list of card
 * transactions.
 *
 * Only `pos_debit` transactions whose `state` is `'settled'` and whose
 * `settledAt` timestamp falls within the last 24 hours relative to `now`
 * are counted.  Top-up deposits are excluded by the type filter.
 *
 * @param transactions - Full list of card activity rows.
 * @param dailyLimitUsdc - The user's configured daily spending limit in USDC.
 * @param now - Reference timestamp (defaults to `Date.now()`).  Injected for
 *              deterministic unit testing.
 * @returns {@link DailySpendingProgress} with spending totals and colour state.
 */
export function calculateDailySpendingProgress(
  transactions: DailySpendingTransaction[],
  dailyLimitUsdc: number,
  now: number = Date.now(),
): DailySpendingProgress {
  const windowStart = now - 24 * 60 * 60 * 1000;

  const spentStroops = transactions
    .filter(
      (tx) =>
        tx.type === 'pos_debit' &&
        tx.state === 'settled' &&
        new Date(tx.settledAt).getTime() >= windowStart,
    )
    .reduce((sum, tx) => sum + tx.amount_stroops, 0);

  const STROOPS_PER_USDC = 10_000_000;
  const spentUsdc = spentStroops / STROOPS_PER_USDC;

  const limit = dailyLimitUsdc > 0 ? dailyLimitUsdc : 0;
  const rawPercent = limit > 0 ? (spentUsdc / limit) * 100 : 0;
  const percentUsed = Math.min(100, rawPercent);

  let color: DailyLimitColor = 'normal';
  if (percentUsed >= 100) {
    color = 'red';
  } else if (percentUsed >= AMBER_THRESHOLD) {
    color = 'amber';
  }

  return { spentUsdc, dailyLimitUsdc: limit, percentUsed, color };
}
