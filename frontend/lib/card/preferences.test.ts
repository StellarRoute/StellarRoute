import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  CARD_TOP_UP_PRESETS,
  CARD_TOP_UP_PREFERENCES_STORAGE_PREFIX,
  DEFAULT_CARD_ALERT_THRESHOLD_USDC,
  DEFAULT_CARD_TOP_UP_USDC,
  cardTopUpPreferencesStorageKey,
  clearCardTopUpPreferences,
  defaultCardTopUpPreferences,
  getCardTopUpPreferences,
  getPreferredTopUpAmount,
  isValidCardUsdcAmount,
  loadCardTopUpPreferences,
  resetCardTopUpPreferences,
  resolveAlertThresholdUsdc,
  saveCardTopUpPreferences,
  shouldShowLowBalanceAlert,
  type CardTopUpPreferences,
} from './preferences';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';
const OTHER_ADDRESS =
  'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB2';

const preferences: CardTopUpPreferences = {
  alertThresholdUsdc: 10,
  defaultTopUpUsdc: 100,
};

afterEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('card top-up preference storage', () => {
  it('keys storage by the active Stellar public key', () => {
    expect(cardTopUpPreferencesStorageKey(ADDRESS)).toBe(
      `${CARD_TOP_UP_PREFERENCES_STORAGE_PREFIX}:${ADDRESS}`,
    );
    expect(cardTopUpPreferencesStorageKey(`  ${ADDRESS}  `)).toBe(
      cardTopUpPreferencesStorageKey(ADDRESS),
    );
  });

  it('rejects a blank address instead of sharing one global record', () => {
    expect(() => cardTopUpPreferencesStorageKey('   ')).toThrow(TypeError);
    expect(() =>
      saveCardTopUpPreferences('', preferences),
    ).toThrow(TypeError);
    expect(window.localStorage.length).toBe(0);
  });

  it('writes then reads back the saved preferences', () => {
    saveCardTopUpPreferences(ADDRESS, preferences);

    expect(
      window.localStorage.getItem(cardTopUpPreferencesStorageKey(ADDRESS)),
    ).toBe(JSON.stringify(preferences));
    expect(loadCardTopUpPreferences(ADDRESS)).toEqual(preferences);
  });

  it('makes no network request when persisting', () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch');

    saveCardTopUpPreferences(ADDRESS, preferences);

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('isolates preferences per address', () => {
    saveCardTopUpPreferences(ADDRESS, preferences);

    expect(loadCardTopUpPreferences(OTHER_ADDRESS)).toBeNull();
    expect(getCardTopUpPreferences(OTHER_ADDRESS)).toEqual(
      defaultCardTopUpPreferences(),
    );

    saveCardTopUpPreferences(OTHER_ADDRESS, {
      alertThresholdUsdc: 50,
      defaultTopUpUsdc: 250,
    });

    expect(loadCardTopUpPreferences(ADDRESS)).toEqual(preferences);
    expect(resolveAlertThresholdUsdc(OTHER_ADDRESS)).toBe(50);
  });

  it('falls back to the 25 USDC threshold and 50 USDC top-up defaults', () => {
    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();
    expect(resolveAlertThresholdUsdc(ADDRESS)).toBe(
      DEFAULT_CARD_ALERT_THRESHOLD_USDC,
    );
    expect(getPreferredTopUpAmount(ADDRESS)).toBe(DEFAULT_CARD_TOP_UP_USDC);
    expect(CARD_TOP_UP_PRESETS).toContain(DEFAULT_CARD_ALERT_THRESHOLD_USDC);
  });

  it('ignores malformed or non-positive stored data', () => {
    const key = cardTopUpPreferencesStorageKey(ADDRESS);

    window.localStorage.setItem(key, 'not-json');
    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();

    window.localStorage.setItem(
      key,
      JSON.stringify({ alertThresholdUsdc: 0, defaultTopUpUsdc: 50 }),
    );
    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();

    window.localStorage.setItem(
      key,
      JSON.stringify({ alertThresholdUsdc: 25, defaultTopUpUsdc: -5 }),
    );
    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();

    window.localStorage.setItem(key, JSON.stringify([25, 50]));
    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();

    expect(resolveAlertThresholdUsdc(ADDRESS)).toBe(
      DEFAULT_CARD_ALERT_THRESHOLD_USDC,
    );
  });

  it('rejects non-positive amounts without overwriting a valid record', () => {
    saveCardTopUpPreferences(ADDRESS, preferences);

    expect(() =>
      saveCardTopUpPreferences(ADDRESS, { ...preferences, alertThresholdUsdc: 0 }),
    ).toThrow(TypeError);
    expect(() =>
      saveCardTopUpPreferences(ADDRESS, {
        ...preferences,
        defaultTopUpUsdc: Number.NaN,
      }),
    ).toThrow(TypeError);
    expect(() =>
      saveCardTopUpPreferences(ADDRESS, {
        ...preferences,
        defaultTopUpUsdc: '50' as unknown as number,
      }),
    ).toThrow(TypeError);

    expect(loadCardTopUpPreferences(ADDRESS)).toEqual(preferences);
  });

  it('rounds nothing: fractional thresholds persist exactly', () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 24.99,
      defaultTopUpUsdc: 12.5,
    });

    expect(resolveAlertThresholdUsdc(ADDRESS)).toBe(24.99);
    expect(getPreferredTopUpAmount(ADDRESS)).toBe(12.5);
  });

  it('reset writes the defaults back as an explicit choice', () => {
    saveCardTopUpPreferences(ADDRESS, preferences);

    expect(resetCardTopUpPreferences(ADDRESS)).toEqual(
      defaultCardTopUpPreferences(),
    );
    expect(loadCardTopUpPreferences(ADDRESS)).toEqual({
      alertThresholdUsdc: 25,
      defaultTopUpUsdc: 50,
    });
  });

  it('clearing storage removes only that address record', () => {
    saveCardTopUpPreferences(ADDRESS, preferences);
    saveCardTopUpPreferences(OTHER_ADDRESS, {
      alertThresholdUsdc: 5,
      defaultTopUpUsdc: 500,
    });

    clearCardTopUpPreferences(ADDRESS);

    expect(loadCardTopUpPreferences(ADDRESS)).toBeNull();
    expect(loadCardTopUpPreferences(OTHER_ADDRESS)).toEqual({
      alertThresholdUsdc: 5,
      defaultTopUpUsdc: 500,
    });
  });
});

describe('low balance alert evaluation', () => {
  it('warns strictly below the threshold, matching the 24.99 / 25.00 boundary', () => {
    expect(shouldShowLowBalanceAlert(24.99, 25)).toBe(true);
    expect(shouldShowLowBalanceAlert(25, 25)).toBe(false);
    expect(shouldShowLowBalanceAlert(25.01, 25)).toBe(false);
    expect(shouldShowLowBalanceAlert(0, 25)).toBe(true);
  });

  it('follows the saved user threshold rather than the default', () => {
    saveCardTopUpPreferences(ADDRESS, {
      alertThresholdUsdc: 100,
      defaultTopUpUsdc: 50,
    });

    const threshold = resolveAlertThresholdUsdc(ADDRESS);
    expect(shouldShowLowBalanceAlert(50, threshold)).toBe(true);
    expect(shouldShowLowBalanceAlert(100, threshold)).toBe(false);
  });

  it('stays quiet for unknown balances and invalid thresholds', () => {
    expect(shouldShowLowBalanceAlert(null, 25)).toBe(false);
    expect(shouldShowLowBalanceAlert(undefined, 25)).toBe(false);
    expect(shouldShowLowBalanceAlert(Number.NaN, 25)).toBe(false);
    expect(shouldShowLowBalanceAlert(10, 0)).toBe(false);
    expect(shouldShowLowBalanceAlert(10, Number.NaN)).toBe(false);
  });

  it('validates amounts for callers building custom inputs', () => {
    expect(isValidCardUsdcAmount(25)).toBe(true);
    expect(isValidCardUsdcAmount(0.01)).toBe(true);
    expect(isValidCardUsdcAmount(0)).toBe(false);
    expect(isValidCardUsdcAmount(-1)).toBe(false);
    expect(isValidCardUsdcAmount(Number.POSITIVE_INFINITY)).toBe(false);
    expect(isValidCardUsdcAmount('25')).toBe(false);
    expect(isValidCardUsdcAmount(undefined)).toBe(false);
  });
});
