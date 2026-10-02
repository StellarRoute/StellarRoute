/**
 * AI-50 capstone frontend test (closes #1460).
 *
 * A parsed send matches the JSON the server accepts when the flag is on,
 * and resolves to null (flag-off 404 path) without touching quote ranking.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';

import { validateIntent } from './client';

const SEND_INTENT = {
  kind: 'send',
  amount: '50',
  fromAsset: 'XLM',
  recipient: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
} as const;

describe('validateIntent send capstone', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
    delete (window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> })
      .__STELLAR_ROUTE_FLAGS__;
  });

  it('parsed send matches server-accepted JSON when flag is on', async () => {
    process.env.NEXT_PUBLIC_AI_AGENT = 'true';
    const fetchMock = vi.fn().mockResolvedValue({
      status: 200,
      ok: true,
      json: async () => ({ data: { amount: '50', type: 'send' } }),
    });
    vi.stubGlobal('fetch', fetchMock);

    const result = await validateIntent({ ...SEND_INTENT });

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/v1/agent/intents/validate',
      expect.objectContaining({
        method: 'POST',
        body: expect.stringContaining('"amount":"50"'),
      }),
    );
    expect(result).toEqual({ amount: '50', type: 'send' });
    const quoteCalls = fetchMock.mock.calls.filter(([url]) =>
      String(url).includes('/api/v1/quote'),
    );
    expect(quoteCalls).toHaveLength(0);
  });

  it('same request resolves null on flag-off 404', async () => {
    process.env.NEXT_PUBLIC_AI_AGENT = 'true';
    const fetchMock = vi.fn().mockResolvedValue({
      status: 404,
      ok: false,
      json: async () => ({ error: 'Not Found' }),
    });
    vi.stubGlobal('fetch', fetchMock);

    const result = await validateIntent({ ...SEND_INTENT });

    expect(result).toBeNull();
  });

  it('returns null without fetching when flag is unset', async () => {
    delete process.env.NEXT_PUBLIC_AI_AGENT;
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);

    const result = await validateIntent({ ...SEND_INTENT });

    expect(result).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
