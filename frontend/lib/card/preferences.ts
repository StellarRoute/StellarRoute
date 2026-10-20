/**
 * CARD-52 - Local card threshold and auto-top-up preferences.
 *
 * Cardholders spend at very different velocities, so a single hardcoded
 * threshold and top-up amount fits nobody well. These preferences are stored
 * in the browser only, scoped to the active Stellar public key, and never
 * leave the device. StellarRoute holds no keys and no card PANs.
 */

export interface CardTopUpPreferences {
  /** Available USDC below this value triggers the low balance warning. */
  alertThresholdUsdc: number;
  /** Amount pre-filled into the card top-up flow. */
  defaultTopUpUsdc: number;
}

export const DEFAULT_CARD_ALERT_THRESHOLD_USDC = 25;
export const DEFAULT_CARD_TOP_UP_USDC = 50;

export const CARD_TOP_UP_PRESETS: readonly number[] = [10, 25, 50, 100];

export const CARD_TOP_UP_PREFERENCES_STORAGE_PREFIX =
  'stellar_route_card_top_up_preferences';

export const defaultCardTopUpPreferences = (): CardTopUpPreferences => ({
  alertThresholdUsdc: DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  defaultTopUpUsdc: DEFAULT_CARD_TOP_UP_USDC,
});

function isUsdcAmount(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value > 0;
}

/** Amounts are USDC balances, so anything non-positive is unusable. */
export function isValidCardUsdcAmount(value: unknown): value is number {
  return isUsdcAmount(value);
}

export function cardTopUpPreferencesStorageKey(stellarAddress: string): string {
  const address = stellarAddress.trim();
  if (address.length === 0) {
    throw new TypeError(
      'Card preferences must be scoped to a non-empty Stellar public key.',
    );
  }
  return `${CARD_TOP_UP_PREFERENCES_STORAGE_PREFIX}:${address}`;
}

function readPreferences(value: unknown): CardTopUpPreferences | null {
  if (typeof value !== 'object' || value === null) return null;

  const prefs = value as Partial<CardTopUpPreferences>;
  if (
    !isUsdcAmount(prefs.alertThresholdUsdc) ||
    !isUsdcAmount(prefs.defaultTopUpUsdc)
  ) {
    return null;
  }

  return {
    alertThresholdUsdc: prefs.alertThresholdUsdc,
    defaultTopUpUsdc: prefs.defaultTopUpUsdc,
  };
}

export function saveCardTopUpPreferences(
  stellarAddress: string,
  preferences: CardTopUpPreferences,
): CardTopUpPreferences {
  const next = readPreferences(preferences);
  if (next === null) {
    throw new TypeError(
      'Card preferences need a positive alertThresholdUsdc and defaultTopUpUsdc.',
    );
  }
  if (typeof window === 'undefined') return next;

  window.localStorage.setItem(
    cardTopUpPreferencesStorageKey(stellarAddress),
    JSON.stringify(next),
  );
  return next;
}

export function loadCardTopUpPreferences(
  stellarAddress: string,
): CardTopUpPreferences | null {
  if (typeof window === 'undefined') return null;

  try {
    const stored = window.localStorage.getItem(
      cardTopUpPreferencesStorageKey(stellarAddress),
    );
    if (!stored) return null;

    return readPreferences(JSON.parse(stored));
  } catch {
    return null;
  }
}

/** Saved values only - falls back to the shipped defaults when unset. */
export function getCardTopUpPreferences(
  stellarAddress: string,
): CardTopUpPreferences {
  return loadCardTopUpPreferences(stellarAddress) ?? defaultCardTopUpPreferences();
}

export function resolveAlertThresholdUsdc(stellarAddress: string): number {
  return getCardTopUpPreferences(stellarAddress).alertThresholdUsdc;
}

export function getPreferredTopUpAmount(stellarAddress: string): number {
  return getCardTopUpPreferences(stellarAddress).defaultTopUpUsdc;
}

/**
 * Mirrors CARD-45's rule: warn strictly below the threshold, and only when a
 * balance is actually known.
 */
export function shouldShowLowBalanceAlert(
  availableUsdc: number | null | undefined,
  thresholdUsdc: number,
): boolean {
  if (!Number.isFinite(availableUsdc ?? Number.NaN)) return false;
  if (!Number.isFinite(thresholdUsdc) || thresholdUsdc <= 0) return false;
  return (availableUsdc as number) < thresholdUsdc;
}

/** Writes the defaults back so a reset is still a persisted choice. */
export function resetCardTopUpPreferences(
  stellarAddress: string,
): CardTopUpPreferences {
  return saveCardTopUpPreferences(stellarAddress, defaultCardTopUpPreferences());
}

export function clearCardTopUpPreferences(stellarAddress: string): void {
  if (typeof window === 'undefined') return;
  window.localStorage.removeItem(cardTopUpPreferencesStorageKey(stellarAddress));
}
