/**
 * Institution-code contract for the NGN offramp destination.
 *
 * The chosen NGN payout partner settles on 8-character institution codes.
 * That is a different identifier from the legacy 3-digit CBN bank code that
 * `NIGERIAN_BANKS` still lists, so the two are modelled separately here and
 * must never be interchangeable.
 *
 * This module is pure and additive: it validates/normalizes a string and
 * touches no existing destination, quote, or swap path.
 */

/** Paycrest-style institution codes are exactly 8 characters. */
export const NIGERIAN_INSTITUTION_CODE_LENGTH = 8;

/** Legacy CBN bank codes are exactly 3 digits (e.g. `058`). */
export const NIGERIAN_CBN_BANK_CODE_LENGTH = 3;

const INSTITUTION_CODE_PATTERN = /^[A-Z0-9]{8}$/;
const CBN_BANK_CODE_PATTERN = /^\d{3}$/;

/**
 * Uppercase + strip whitespace, then cap at the institution-code length.
 * Used to mask the input field so a typed value can never exceed 8 chars.
 */
export function normalizeNigerianInstitutionCode(value: string): string {
  return value
    .trim()
    .toUpperCase()
    .replace(/\s+/g, '')
    .slice(0, NIGERIAN_INSTITUTION_CODE_LENGTH);
}

/** True for an 8-character alphanumeric institution code (case-insensitive). */
export function isValidNigerianInstitutionCode(value: string): boolean {
  return INSTITUTION_CODE_PATTERN.test(value.trim().toUpperCase());
}

/**
 * True for a legacy 3-digit CBN bank code. Such a value is explicitly NOT a
 * valid institution code — the payout rail does not accept it.
 */
export function isNigerianCbnBankCode(value: string): boolean {
  return CBN_BANK_CODE_PATTERN.test(value.trim());
}
