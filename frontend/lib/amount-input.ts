/**
 * Sell-side amount parsing, validation, and formatting for Stellar-scale assets.
 *
 * - Locale-aware decimal parsing (US / EU style thousand separators).
 * - Scientific notation is rejected explicitly.
 * - Precision is enforced using a caller-provided max decimal count (e.g. numDecimals).
 */

/** Stellar native (lumens) uses 7 fractional digits. */
export const STELLAR_NATIVE_MAX_DECIMALS = 7;

/** Default when the API does not yet expose per-asset numDecimals (issued assets). */
export const DEFAULT_ISSUED_MAX_DECIMALS = 7;

export type ParseSellAmountResult =
  | { status: 'empty' }
  | { status: 'invalid'; message: string }
  | { status: 'precision_exceeded'; message: string; exceededBy: number }
  | { status: 'ok'; normalized: string; numeric: number };

const SCIENTIFIC_PATTERN = /[eE][+-]?\d/;

/**
 * Resolve max fractional digits for the asset being sold.
 * Optional explicit decimals from API (`TradingPair`) override heuristics.
 */
export function maxDecimalsForSellAsset(
  canonicalAssetId: string,
  explicitDecimals?: number,
): number {
  if (
    explicitDecimals !== undefined &&
    Number.isInteger(explicitDecimals) &&
    explicitDecimals >= 0 &&
    explicitDecimals <= 255
  ) {
    return explicitDecimals;
  }
  if (canonicalAssetId === 'native') {
    return STELLAR_NATIVE_MAX_DECIMALS;
  }
  return DEFAULT_ISSUED_MAX_DECIMALS;
}

/**
 * Strip common whitespace including NBSP / narrow NBSP used in locale formatting.
 */
export function stripAmountWhitespace(raw: string): string {
  return raw.replace(/[\s\u00A0\u202F]/g, '');
}

/**
 * Reject scientific notation (e.g. 1e-8, 2E10).
 */
export function containsScientificNotation(s: string): boolean {
  return SCIENTIFIC_PATTERN.test(s);
}

/**
 * Normalize pasted / typed numeric strings to a single `.` decimal separator
 * and no thousand separators. Returns null if the string is not a plain decimal number.
 *
 * Rules:
 * - If both `,` and `.` appear, the rightmost separator is the decimal mark;
 *   the other is treated as thousands and removed.
 * - If only `,` appears, it is the decimal separator (common EU input).
 * - If only `.` appears, it is the decimal separator (US input).
 */
export function normalizeDecimalString(raw: string): string | null {
  const s = stripAmountWhitespace(raw);
  if (!s) return null;
  if (containsScientificNotation(s)) return null;

  const lastComma = s.lastIndexOf(',');
  const lastDot = s.lastIndexOf('.');

  let work: string;
  if (lastComma >= 0 && lastDot >= 0) {
    if (lastComma > lastDot) {
      // e.g. 1.234,56
      work = s.replace(/\./g, '').replace(',', '.');
    } else {
      // e.g. 1,234.56
      work = s.replace(/,/g, '');
    }
  } else if (lastComma >= 0) {
    work = s.replace(',', '.');
  } else {
    work = s;
  }

  if (containsScientificNotation(work)) return null;

  const parts = work.split('.');
  if (parts.length > 2) return null;

  const intPart = parts[0] ?? '';
  const fracPart = parts[1] ?? '';

  if (!/^\d*$/.test(intPart) || !/^\d*$/.test(fracPart)) return null;
  if (intPart === '' && fracPart === '') return null;

  return fracPart === '' ? intPart : `${intPart}.${fracPart}`;
}

/**
 * Count fractional digits in a normalized decimal string (no trailing validation).
 */
export function fractionalDigitCount(normalized: string): number {
  const dot = normalized.indexOf('.');
  if (dot === -1) return 0;
  return normalized.length - dot - 1;
}

/**
 * Trim trailing zeros after the decimal for display (keeps "0." vs "0" sensible).
 */
export function trimTrailingZeros(normalized: string): string {
  if (!normalized.includes('.')) return normalized;
  const trimmed = normalized.replace(/\.?0+$/, '');
  return trimmed === '' ? '0' : trimmed;
}

/**
 * Parse user sell amount: empty, invalid, too many decimals, or OK with numeric for API.
 */
export function parseSellAmount(
  raw: string,
  maxDecimals: number,
): ParseSellAmountResult {
  const trimmed = raw.trim();
  if (trimmed === '') {
    return { status: 'empty' };
  }

  const normalized = normalizeDecimalString(raw);
  if (normalized === null) {
    return {
      status: 'invalid',
      message:
        'Enter a valid amount using digits only. Scientific notation is not supported.',
    };
  }

  const fracDigits = fractionalDigitCount(normalized);
  if (fracDigits > maxDecimals) {
    return {
      status: 'precision_exceeded',
      message: `Maximum ${maxDecimals} decimal places for this asset.`,
      exceededBy: fracDigits - maxDecimals,
    };
  }

  const numeric = Number(normalized);
  if (!Number.isFinite(numeric)) {
    return { status: 'invalid', message: 'Amount is not a valid number.' };
  }
  if (numeric <= 0) {
    return { status: 'invalid', message: 'Amount must be greater than zero.' };
  }

  return { status: 'ok', normalized, numeric };
}

/**
 * Clamp a normalized decimal string to maxDecimals (truncate fractional part — no rounding up).
 */
export function clampToMaxDecimals(
  normalized: string,
  maxDecimals: number,
): string {
  if (maxDecimals <= 0) {
    const intPart = normalized.split('.')[0] ?? '0';
    return intPart === '' ? '0' : intPart;
  }
  const dot = normalized.indexOf('.');
  if (dot === -1) return normalized;
  const intPart = normalized.slice(0, dot);
  let frac = normalized.slice(dot + 1, dot + 1 + maxDecimals);
  frac = frac.replace(/0+$/, '');
  if (frac === '') return intPart === '' ? '0' : intPart;
  return `${intPart || '0'}.${frac}`;
}

/**
 * Apply "max balance" string (normalized or raw) into the input, respecting decimals.
 */
export function formatMaxAmountForInput(
  balanceNormalized: string,
  maxDecimals: number,
): string {
  const n = normalizeDecimalString(balanceNormalized);
  if (n === null) return '';
  const clamped = clampToMaxDecimals(n, maxDecimals);
  return trimTrailingZeros(clamped);
}
