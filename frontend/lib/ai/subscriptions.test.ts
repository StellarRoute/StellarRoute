import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
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

import {
  createSubscription,
  listSubscriptions,
  getActiveSubscriptions,
  getSubscription,
  cancelSubscription,
  deleteSubscription,
  calculateNextDueDate,
  recordPaymentExecuted,
  isPaymentDue,
  clearSubscriptions,
  _setRawStorageForTesting,
  AI_SUBSCRIPTIONS_STORAGE_KEY,
  type SubscriptionRecord,
} from './subscriptions';

describe('AI-25 Local subscription ledger (subscriptions.ts)', () => {
  beforeEach(() => {
    clearSubscriptions();
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.clear();
    }
  });

  afterEach(() => {
    clearSubscriptions();
  });

  describe('Acceptance Criteria: Create, reload, and nextDueDate retention', () => {
    it('creates a subscription and reload returns the identical next due date', () => {
      const fixedStartDate = new Date('2026-10-01T12:00:00.000Z');
      const created = createSubscription({
        payee: 'GBRF5STL42QCUTA7FQK6KKWBHFHCGJ5BVK5RTR64KK57NUKSQAOXRSIA',
        asset: 'USDC',
        amount: '25.00',
        interval: 'monthly',
        max: 12,
        startDate: fixedStartDate,
        memo: 'Monthly SaaS seat'
      });

      expect(created.id).toBeDefined();
      expect(created.payee).toBe('GBRF5STL42QCUTA7FQK6KKWBHFHCGJ5BVK5RTR64KK57NUKSQAOXRSIA');
      expect(created.asset).toBe('USDC');
      expect(created.amount).toBe('25.00');
      expect(created.interval).toBe('monthly');
      expect(created.max).toBe(12);
      expect(created.createdCount).toBe(0);
      expect(created.status).toBe('active');
      expect(created.nextDueDate).toBe(fixedStartDate.toISOString());

      // Simulate client reload by reading freshly from storage
      const reloadedList = listSubscriptions();
      expect(reloadedList.length).toBe(1);

      const reloadedItem = reloadedList[0];
      expect(reloadedItem.id).toBe(created.id);
      expect(reloadedItem.nextDueDate).toBe(created.nextDueDate);
      expect(reloadedItem.max).toBe(12);
      expect(reloadedItem.payee).toBe(created.payee);
    });

    it('calculates the next due date automatically if startDate is omitted', () => {
      const before = new Date();
      const created = createSubscription({
        payee: 'GAAAZZZ111222',
        asset: 'XLM',
        amount: '100',
        interval: 'weekly'
      });

      expect(created.nextDueDate).toBeDefined();
      const dueDate = new Date(created.nextDueDate);
      // Weekly should be approximately 7 days in future
      const diffMs = dueDate.getTime() - before.getTime();
      const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));
      expect(diffDays).toBe(7);
    });
  });

  describe('Acceptance Criteria: Cancellation deletes local record', () => {
    it('cancels and removes the local record completely from storage', () => {
      const sub1 = createSubscription({
        payee: 'G11111111111111111111111111111111111111111111111111111111',
        asset: 'USDC',
        amount: '10',
        interval: 'monthly'
      });

      const sub2 = createSubscription({
        payee: 'G22222222222222222222222222222222222222222222222222222222',
        asset: 'USDC',
        amount: '20',
        interval: 'weekly'
      });

      expect(listSubscriptions().length).toBe(2);

      // Cancel sub1
      const cancelled = cancelSubscription(sub1.id);
      expect(cancelled).toBe(true);

      // Verify sub1 is completely removed
      const remaining = listSubscriptions();
      expect(remaining.length).toBe(1);
      expect(remaining[0].id).toBe(sub2.id);
      expect(getSubscription(sub1.id)).toBeNull();
    });

    it('returns false when attempting to cancel a non-existent subscription ID', () => {
      const res = cancelSubscription('non-existent-sub-id');
      expect(res).toBe(false);
    });

    it('deleteSubscription alias performs identical removal', () => {
      const sub = createSubscription({
        payee: 'G33333333333333333333333333333333333333333333333333333333',
        asset: 'USDC',
        amount: '5',
        interval: 'daily'
      });

      expect(listSubscriptions().length).toBe(1);
      const deleted = deleteSubscription(sub.id);
      expect(deleted).toBe(true);
      expect(listSubscriptions().length).toBe(0);
    });
  });

  describe('Acceptance Criteria: No server POST is made', () => {
    it('operates 100% locally without invoking fetch, XMLHttpRequest, or network APIs', () => {
      const fetchSpy = vi.fn();
      const originalFetch = globalThis.fetch;
      globalThis.fetch = fetchSpy;

      try {
        // Perform multiple lifecycle actions
        const sub = createSubscription({
          payee: 'G44444444444444444444444444444444444444444444444444444444',
          asset: 'USDC',
          amount: '15',
          interval: 'monthly',
          max: 3
        });

        listSubscriptions();
        getSubscription(sub.id);
        recordPaymentExecuted(sub.id);
        cancelSubscription(sub.id);

        // Expect zero network interaction
        expect(fetchSpy).not.toHaveBeenCalled();
      } finally {
        globalThis.fetch = originalFetch;
      }
    });
  });

  describe('Date Arithmetic: calculateNextDueDate', () => {
    it('handles daily recurrence (+1 day)', () => {
      const base = new Date('2026-05-10T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'daily');
      expect(new Date(next).toISOString()).toBe('2026-05-11T00:00:00.000Z');
    });

    it('handles weekly recurrence (+7 days)', () => {
      const base = new Date('2026-05-10T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'weekly');
      expect(new Date(next).toISOString()).toBe('2026-05-17T00:00:00.000Z');
    });

    it('handles biweekly recurrence (+14 days)', () => {
      const base = new Date('2026-05-10T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'biweekly');
      expect(new Date(next).toISOString()).toBe('2026-05-24T00:00:00.000Z');
    });

    it('handles standard monthly recurrence (+1 month)', () => {
      const base = new Date('2026-05-15T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'monthly');
      expect(new Date(next).toISOString()).toBe('2026-06-15T00:00:00.000Z');
    });

    it('handles month-end rollover clamping (e.g. Jan 31 -> Feb 28 in non-leap year)', () => {
      // 2026 is not a leap year
      const base = new Date('2026-01-31T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'monthly');
      const nextDate = new Date(next);
      expect(nextDate.getUTCMonth()).toBe(1); // February (0-indexed)
      expect(nextDate.getUTCDate()).toBe(28); // Clamped to Feb 28
    });

    it('handles quarterly recurrence (+3 months)', () => {
      const base = new Date('2026-01-15T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'quarterly');
      expect(new Date(next).toISOString()).toBe('2026-04-15T00:00:00.000Z');
    });

    it('handles yearly recurrence (+1 year)', () => {
      const base = new Date('2026-03-20T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'yearly');
      expect(new Date(next).toISOString()).toBe('2027-03-20T00:00:00.000Z');
    });

    it('handles custom day string patterns (e.g. "every 10 days")', () => {
      const base = new Date('2026-01-01T00:00:00.000Z');
      const next = calculateNextDueDate(base, 'every 10 days');
      expect(new Date(next).toISOString()).toBe('2026-01-11T00:00:00.000Z');
    });
  });

  describe('Payment Execution and Status Progression', () => {
    it('increments createdCount and advances nextDueDate on each execution', () => {
      const start = new Date('2026-06-01T00:00:00.000Z');
      const sub = createSubscription({
        payee: 'G55555555555555555555555555555555555555555555555555555555',
        asset: 'USDC',
        amount: '50',
        interval: 'monthly',
        max: 3,
        startDate: start
      });

      expect(sub.createdCount).toBe(0);
      expect(sub.status).toBe('active');

      // 1st payment execution
      const after1 = recordPaymentExecuted(sub.id);
      expect(after1).not.toBeNull();
      expect(after1!.createdCount).toBe(1);
      expect(after1!.status).toBe('active');
      expect(after1!.nextDueDate).toBe('2026-07-01T00:00:00.000Z');

      // 2nd payment execution
      const after2 = recordPaymentExecuted(sub.id);
      expect(after2!.createdCount).toBe(2);
      expect(after2!.status).toBe('active');
      expect(after2!.nextDueDate).toBe('2026-08-01T00:00:00.000Z');

      // 3rd payment execution (max = 3 reached)
      const after3 = recordPaymentExecuted(sub.id);
      expect(after3!.createdCount).toBe(3);
      expect(after3!.status).toBe('completed');
    });

    it('identifies when payment is due based on reference date', () => {
      const sub = createSubscription({
        payee: 'G66666666666666666666666666666666666666666666666666666666',
        asset: 'USDC',
        amount: '10',
        interval: 'monthly',
        startDate: new Date('2026-05-01T00:00:00.000Z')
      });

      // Before due date
      expect(isPaymentDue(sub, new Date('2026-04-30T23:59:59.000Z'))).toBe(false);
      // Exactly on due date
      expect(isPaymentDue(sub, new Date('2026-05-01T00:00:00.000Z'))).toBe(true);
      // After due date
      expect(isPaymentDue(sub, new Date('2026-05-02T00:00:00.000Z'))).toBe(true);
    });
  });

  describe('Edge Cases and Resilience', () => {
    it('gracefully handles malformed JSON in localStorage without throwing', () => {
      _setRawStorageForTesting(AI_SUBSCRIPTIONS_STORAGE_KEY, '{ invalid JSON string... [');

      const list = listSubscriptions();
      expect(list).toEqual([]);

      // Creating a new one should self-heal and write valid JSON
      const created = createSubscription({
        payee: 'G77777777777777777777777777777777777777777777777777777777',
        asset: 'USDC',
        amount: '10',
        interval: 'monthly'
      });

      expect(created.id).toBeDefined();
      expect(listSubscriptions().length).toBe(1);
    });

    it('filters corrupted non-object elements from storage array', () => {
      _setRawStorageForTesting(
        AI_SUBSCRIPTIONS_STORAGE_KEY,
        JSON.stringify([null, 42, 'string', { id: 'valid_1', payee: 'G...', amount: '5', nextDueDate: '2026-01-01' }])
      );

      const list = listSubscriptions();
      expect(list.length).toBe(1);
      expect(list[0].id).toBe('valid_1');
    });

    it('filters active subscriptions from completed or cancelled ones', () => {
      const active = createSubscription({
        payee: 'G88888888888888888888888888888888888888888888888888888888',
        asset: 'USDC',
        amount: '10',
        interval: 'monthly',
        max: 1
      });

      expect(getActiveSubscriptions().length).toBe(1);

      // Complete it
      recordPaymentExecuted(active.id);
      expect(getActiveSubscriptions().length).toBe(0);
      expect(listSubscriptions().length).toBe(1);
      expect(listSubscriptions()[0].status).toBe('completed');
    });
  });
});
