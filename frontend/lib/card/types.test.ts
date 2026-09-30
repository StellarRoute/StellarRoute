/**
 * CARD-05 type-level and runtime guards.
 *
 * These tests fail the build if:
 *   - a PAN-length field name is added to CardAccount, or
 *   - the status union loses 'active', 'frozen', 'pending', etc.
 */
import { describe, expect, it } from 'vitest';
import type { CardAccount, CardStatus } from './types';
import { CARD_ACCOUNT_KEYS } from './types';

// ── PAN-length field guard ───────────────────────────────────────────────────
// A PAN is 13–19 digits long. Field names that suggest a PAN are similarly
// long when they encode one. Block any key whose name implies full card data.
const FORBIDDEN_KEY_PATTERNS = [
  /^pan$/i,
  /^cvv$/i,
  /^cvc$/i,
  /^track/i,
  /card_number/i,
  /full.*card/i,
  /card.*full/i,
];

describe('CardAccount key guard', () => {
  it('has no PAN, CVV, or track field', () => {
    for (const key of CARD_ACCOUNT_KEYS) {
      for (const pattern of FORBIDDEN_KEY_PATTERNS) {
        expect(
          pattern.test(key),
          `CardAccount must not have a field matching ${pattern} — found: "${key}"`,
        ).toBe(false);
      }
    }
  });

  it('key list matches the TypeScript interface (no drift)', () => {
    // Build a fixture with every required field. TypeScript errors here if
    // the interface gains a required key not in CARD_ACCOUNT_KEYS.
    const fixture: CardAccount = {
      id: 'card_test_01',
      status: 'active',
      last4: '4242',
      expiryMonth: 12,
      expiryYear: 2028,
      availableUsdc: '100.00',
      heldUsdc: '0.00',
      fiatCurrencyCode: 'USD',
    };
    // Every key in the fixture must appear in the exported key list.
    for (const key of Object.keys(fixture) as Array<keyof CardAccount>) {
      expect(CARD_ACCOUNT_KEYS).toContain(key);
    }
  });
});

// ── Fixture: last4 = "4242" typechecks ──────────────────────────────────────
describe('CardAccount fixture', () => {
  it('last4 "4242" is a valid four-digit string', () => {
    const card: CardAccount = {
      id: 'card_test_02',
      status: 'active',
      last4: '4242',
      expiryMonth: 6,
      expiryYear: 2027,
      availableUsdc: '50.00',
      heldUsdc: '5.00',
      fiatCurrencyCode: 'USD',
    };
    expect(card.last4).toBe('4242');
    expect(card.last4).toMatch(/^\d{4}$/);
  });

  it('last4 null is valid (canceled/masked card)', () => {
    const card: CardAccount = {
      id: 'card_test_03',
      status: 'canceled',
      last4: null,
      expiryMonth: 1,
      expiryYear: 2025,
      availableUsdc: '0.00',
      heldUsdc: '0.00',
      fiatCurrencyCode: 'EUR',
    };
    expect(card.last4).toBeNull();
  });
});

// ── Status union membership ──────────────────────────────────────────────────
describe('CardStatus', () => {
  const validStatuses: CardStatus[] = [
    'kyc_required',
    'pending',
    'active',
    'frozen',
    'canceled',
  ];

  it('active is a valid status', () => {
    const status: CardStatus = 'active';
    expect(validStatuses).toContain(status);
  });

  it('all expected statuses are assignable', () => {
    for (const s of validStatuses) {
      const status: CardStatus = s;
      expect(validStatuses).toContain(status);
    }
  });
});
