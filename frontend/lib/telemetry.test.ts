import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  SWAP_FUNNEL_EVENT_NAME,
  emitSwapFunnelEvent,
  getPriceImpactTier,
} from './telemetry';

describe('swap funnel telemetry (#1016)', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('maps price impact into safe tiers', () => {
    expect(getPriceImpactTier(0.1)).toBe('low');
    expect(getPriceImpactTier(0.5)).toBe('medium');
    expect(getPriceImpactTier(2)).toBe('high');
    expect(getPriceImpactTier(5)).toBe('severe');
  });

  it('emits quote_requested / confirm / submit / finalize / fail events', () => {
    const listener = vi.fn();
    window.addEventListener(SWAP_FUNNEL_EVENT_NAME, listener as EventListener);

    try {
      for (const name of [
        'quote_requested',
        'confirm_clicked',
        'swap_submitted',
        'swap_finalized',
        'swap_failed',
      ] as const) {
        emitSwapFunnelEvent(name, {
          quoteId: 'q1',
          routeId: 'r1',
          fromAssetCode: 'XLM',
          toAssetCode: 'USDC',
          hopCount: 1,
          priceImpactTier: 'low',
          ...(name === 'swap_failed' ? { failureStage: 'submit' } : {}),
        });
      }

      expect(listener).toHaveBeenCalledTimes(5);
      const names = listener.mock.calls.map(
        ([event]) => (event as CustomEvent).detail.eventName,
      );
      expect(names).toEqual([
        'quote_requested',
        'confirm_clicked',
        'swap_submitted',
        'swap_finalized',
        'swap_failed',
      ]);

      const failed = (listener.mock.calls[4][0] as CustomEvent).detail;
      expect(failed.payload).not.toHaveProperty('walletAddress');
      expect(failed.payload).not.toHaveProperty('amountIn');
      expect(failed.payload.failureStage).toBe('submit');
    } finally {
      window.removeEventListener(SWAP_FUNNEL_EVENT_NAME, listener as EventListener);
    }
  });

  it('does not emit when NEXT_PUBLIC_TELEMETRY_ENABLED is false', () => {
    vi.stubEnv('NEXT_PUBLIC_TELEMETRY_ENABLED', 'false');
    const listener = vi.fn();
    window.addEventListener(SWAP_FUNNEL_EVENT_NAME, listener as EventListener);

    try {
      emitSwapFunnelEvent('quote_requested', { fromAssetCode: 'XLM' });
      expect(listener).not.toHaveBeenCalled();
    } finally {
      window.removeEventListener(SWAP_FUNNEL_EVENT_NAME, listener as EventListener);
    }
  });
});
