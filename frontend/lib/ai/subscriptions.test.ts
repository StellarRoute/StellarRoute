import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  loadSubscriptions,
  saveSubscription,
  cancelSubscription,
  type Subscription,
} from './subscriptions';

function makeSubscription(overrides: Partial<Subscription> = {}): Subscription {
  return {
    id: 'sub_001',
    payee: 'GABC1234567890123456789012345678901234567890123456',
    asset: 'USDC',
    amount: '100',
    interval: 'monthly',
    max: '12',
    createdCount: 1,
    nextDueDate: '2026-10-01T00:00:00Z',
    ...overrides,
  };
}

describe('subscriptions (#1435)', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it('create then reload returns the same next due date', () => {
    const sub = makeSubscription();
    saveSubscription(sub);

    const loaded = loadSubscriptions();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].nextDueDate).toBe('2026-10-01T00:00:00Z');
  });

  it('cancel removes it', () => {
    const sub = makeSubscription();
    saveSubscription(sub);
    expect(loadSubscriptions()).toHaveLength(1);

    cancelSubscription(sub.id);
    expect(loadSubscriptions()).toHaveLength(0);
  });

  it('no server POST is made', async () => {
    const fetchSpy = vi.spyOn(global, 'fetch').mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({}),
    } as Response);

    const sub = makeSubscription();
    saveSubscription(sub);

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('load returns empty array when no subscriptions exist', () => {
    expect(loadSubscriptions()).toEqual([]);
  });

  it('save updates an existing subscription', () => {
    const sub = makeSubscription();
    saveSubscription(sub);

    const updated = { ...sub, amount: '200', nextDueDate: '2026-11-01T00:00:00Z' };
    saveSubscription(updated);

    const loaded = loadSubscriptions();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].amount).toBe('200');
    expect(loaded[0].nextDueDate).toBe('2026-11-01T00:00:00Z');
  });

  it('multiple subscriptions can coexist', () => {
    saveSubscription(makeSubscription({ id: 'sub_001', payee: 'payee1' }));
    saveSubscription(makeSubscription({ id: 'sub_002', payee: 'payee2' }));

    const loaded = loadSubscriptions();
    expect(loaded).toHaveLength(2);
  });

  it('cancel only removes the matching subscription', () => {
    saveSubscription(makeSubscription({ id: 'sub_001' }));
    saveSubscription(makeSubscription({ id: 'sub_002' }));

    cancelSubscription('sub_001');

    const loaded = loadSubscriptions();
    expect(loaded).toHaveLength(1);
    expect(loaded[0].id).toBe('sub_002');
  });
});
