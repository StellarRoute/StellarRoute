import { describe, expect, it, vi, afterEach } from 'vitest';

import { StellarRouteApiError, StellarRouteClient } from './client';

describe('StellarRouteClient.getPriceHistory', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('serializes pagination/window params and returns typed payload', async () => {
    const payload = {
      base_asset: { asset_type: 'native' },
      quote_asset: {
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'G...USDC',
      },
      points: [{ timestamp: 1710000000000, price: '0.101' }],
      interval: '1m',
      next_cursor: 'cursor-next',
    };

    const fetchMock = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(
        new Response(JSON.stringify(payload), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }),
      );

    const client = new StellarRouteClient('http://localhost:8080');
    const result = await client.getPriceHistory(
      'native',
      'USDC:G...USDC',
      {
        interval: '1m',
        from: 1710000000000,
        to: 1710003600000,
        limit: 200,
        cursor: 'cursor-prev',
        window: '24h',
      },
    );

    expect(result.points).toHaveLength(1);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    const url = String(fetchMock.mock.calls[0][0]);
    expect(url).toContain('/api/v1/price-history/native/USDC%3AG...USDC');
    expect(url).toContain('interval=1m');
    expect(url).toContain('from=1710000000000');
    expect(url).toContain('to=1710003600000');
    expect(url).toContain('limit=200');
    expect(url).toContain('cursor=cursor-prev');
    expect(url).toContain('window=24h');
  });

  it('surfaces API failures as StellarRouteApiError', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          error: 'bad_request',
          message: 'Invalid interval',
          details: { interval: 'unsupported' },
        }),
        {
          status: 400,
          headers: { 'Content-Type': 'application/json' },
        },
      ),
    );

    const client = new StellarRouteClient('http://localhost:8080');

    await expect(
      client.getPriceHistory('native', 'USDC:G...USDC', {
        interval: 'bad',
      }),
    ).rejects.toBeInstanceOf(StellarRouteApiError);
  });
});
