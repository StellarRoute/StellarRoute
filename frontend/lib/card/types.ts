/**
 * Card program types (CARD-05).
 *
 * This file intentionally has no PAN, CVV, or track field.
 * StellarRoute must never hold or represent full card numbers.
 */

/**
 * Lifecycle status for a card account.
 * Derived from the compliance-state map (docs/card/compliance-states.md).
 */
export type CardStatus =
  | 'kyc_required'
  | 'pending'
  | 'active'
  | 'frozen'
  | 'canceled';

/**
 * Fiat currency codes supported by the card program.
 */
export type FiatCurrencyCode = 'USD' | 'EUR' | 'GBP' | 'NGN';

/**
 * A card account as the UI layer sees it — no PAN, CVV, or track data.
 *
 * `last4` is exactly four ASCII digits or null (never 13–19 digits).
 */
export interface CardAccount {
  /** Server-issued card account identifier. */
  id: string;

  /** Current lifecycle state. */
  status: CardStatus;

  /**
   * Last four digits of the card number, or null when unavailable
   * (e.g. card canceled or masked by server policy).
   * Always exactly four ASCII decimal digits when present.
   */
  last4: string | null;

  /** Card expiry month (1–12). */
  expiryMonth: number;

  /** Card expiry year (four-digit, e.g. 2028). */
  expiryYear: number;

  /** USDC balance available for spending. Decimal string. */
  availableUsdc: string;

  /** USDC balance held / reserved (pending authorizations). Decimal string. */
  heldUsdc: string;

  /** Fiat currency code for display purposes. */
  fiatCurrencyCode: FiatCurrencyCode;
}

// ── Key list — used by the PAN-guard test ────────────────────────────────────
/**
 * Exhaustive list of allowed top-level keys on {@link CardAccount}.
 * The build-time test in types.test.ts asserts no PAN-length key is present.
 */
export const CARD_ACCOUNT_KEYS: ReadonlyArray<keyof CardAccount> = [
  'id',
  'status',
  'last4',
  'expiryMonth',
  'expiryYear',
  'availableUsdc',
  'heldUsdc',
  'fiatCurrencyCode',
] as const;
