import { describe, it, expect, vi } from 'vitest';
import { Networks, StrKey } from '@stellar/stellar-base';
import { buildSendPaymentXdr } from './tools/send';
import {
  advanceDueDate,
  confirmDuePayment,
  isDue,
  type Subscription,
} from './subscriptions';

const SOURCE = StrKey.encodeEd25519PublicKey(Buffer.alloc(32, 1));
const PAYEE = StrKey.encodeEd25519PublicKey(Buffer.alloc(32, 2));
const NOW = Date.UTC(2026, 0, 15, 12);
const HASH = 'a'.repeat(64);

function makeSub(overrides: Partial<Subscription> = {}): Subscription {
  return {
    id: 'sub-1',
    payee: PAYEE,
    asset: 'native',
    amount: '5',
    interval: 'monthly',
    maxPayments: 12,
    paidCount: 0,
    nextDueAt: NOW,
    payments: [],
    ...overrides,
  };
}

const base = {
  sourceAddress: SOURCE,
  networkPassphrase: Networks.TESTNET,
  horizonUrl: 'https://horizon-testnet.stellar.org',
  sequenceOverride: 100n,
  now: NOW,
};

describe('confirmDuePayment', () => {
  it('signs the same unsigned Payment the send tool builds', async () => {
    const signAndSubmit = vi.fn().mockResolvedValue(HASH);
    await confirmDuePayment({ ...base, subscription: makeSub(), signAndSubmit });

    const expected = await buildSendPaymentXdr({
      sourceAddress: SOURCE,
      destination: PAYEE,
      asset: 'native',
      amount: '5',
      networkPassphrase: Networks.TESTNET,
      horizonUrl: base.horizonUrl,
      sequenceOverride: 100n,
    });
    expect(signAndSubmit).toHaveBeenCalledTimes(1);
    expect(signAndSubmit).toHaveBeenCalledWith(expected);
  });

  it('success stores the hash, increments the count, and advances the date', async () => {
    const sub = makeSub();
    const updated = await confirmDuePayment({
      ...base,
      subscription: sub,
      signAndSubmit: vi.fn().mockResolvedValue(HASH),
    });

    expect(updated.paidCount).toBe(1);
    expect(updated.payments).toEqual([{ hash: HASH, paidAt: NOW }]);
    expect(updated.nextDueAt).toBe(Date.UTC(2026, 1, 15, 12));
    expect(sub.paidCount).toBe(0);
  });

  it('the 12th payment of a 12-cap schedule does not schedule another', async () => {
    const updated = await confirmDuePayment({
      ...base,
      subscription: makeSub({ paidCount: 11 }),
      signAndSubmit: vi.fn().mockResolvedValue(HASH),
    });

    expect(updated.paidCount).toBe(12);
    expect(updated.nextDueAt).toBeNull();
    expect(isDue(updated, NOW + 10 * 365 * 24 * 3600 * 1000)).toBe(false);
  });

  it('rejection does not advance', async () => {
    const sub = makeSub({ paidCount: 3 });
    await expect(
      confirmDuePayment({
        ...base,
        subscription: sub,
        signAndSubmit: vi.fn().mockRejectedValue(new Error('User declined')),
      }),
    ).rejects.toThrow('User declined');

    expect(sub.paidCount).toBe(3);
    expect(sub.nextDueAt).toBe(NOW);
    expect(sub.payments).toEqual([]);
  });

  it('refuses a subscription that is not due yet without building or signing', async () => {
    const signAndSubmit = vi.fn();
    await expect(
      confirmDuePayment({
        ...base,
        subscription: makeSub({ nextDueAt: NOW + 1000 }),
        signAndSubmit,
      }),
    ).rejects.toThrow('not due');
    expect(signAndSubmit).not.toHaveBeenCalled();
  });
});

describe('advanceDueDate', () => {
  it('advances by the interval', () => {
    expect(advanceDueDate(NOW, 'daily')).toBe(Date.UTC(2026, 0, 16, 12));
    expect(advanceDueDate(NOW, 'weekly')).toBe(Date.UTC(2026, 0, 22, 12));
    expect(advanceDueDate(NOW, 'yearly')).toBe(Date.UTC(2027, 0, 15, 12));
  });
});
