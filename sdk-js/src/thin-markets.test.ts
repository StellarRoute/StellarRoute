import { describe, expect, it, vi, afterEach } from 'vitest';
import { StellarRouteClient, StellarRouteApiError } from './client.js';

function ok(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function apiError(code: string, message: string, status: number): Response {
    return new Response(JSON.stringify({ error: code, message }), {
      status,
      headers: { 'Content-Type': 'application/json' },
    });
}

const NATIVE = { asset_type: 'native' } as import('./types.js').Asset;
const USDC = { asset_type: 'credit_alphanum4', asset_code: 'USDC', asset_issuer: 'GDUKMGUGDZQK6YH...' } as import('./types.js').Asset;

afterEach(() => {
  vi.restoreAllMocks();
});

describe('Thin Orderbook Routing Scenarios', () => {
  it('splits across routes correctly when a single path has insufficient depth (thin markets)', async () => {
    const splitQuoteFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '1000',
      price: '0.94',
      total: '940',
      quote_type: 'sell',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.96', source: 'sdex:offer1' },
        { from_asset: NATIVE, to_asset: USDC, price: '0.92', source: 'amm:poolXYZ' }
      ],
      timestamp: 1_700_000_000,
    };

    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(splitQuoteFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 1000);

    expect(result.path.length).toBeGreaterThan(1);
    expect(result.path[0]?.source).toContain('sdex');
    expect(result.path[1]?.source).toContain('amm');
    expect(result.total).toBe('940'); 
  });

  it('rejects trade if slippage exceeds limits instead of executing poorly in a thin market', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(
      apiError('slippage_exceeded', 'Slippage limit violated due to thin orderbook depth', 400)
    );

    const client = new StellarRouteClient({ retries: 0 });
    let error: StellarRouteApiError | undefined;
    
    try {
      await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 500_000);
    } catch (e: any) {
      error = e;
    }

    expect(error).toBeDefined();
    expect(error?.status).toBe(400);
    expect(error?.code).toBe('slippage_exceeded');
  });

  it('selects the safest best route avoiding misleading deep paths that degrade execution quality', async () => {
    const robustQuoteFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '500',
      price: '0.98',
      total: '490',
      quote_type: 'sell',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.98', source: 'sdex:reliable-offer' },
      ],
      timestamp: 1_700_000_000,
    };

    vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(robustQuoteFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 500);

    expect(result.path).toHaveLength(1);
    expect(result.path[0]?.source).toBe('sdex:reliable-offer'); 
    expect(result.price).toBe('0.98');
  });

  it('splits across three venues when liquidity is distributed across AMMs', async () => {
    const threeVenueFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '5000',
      price: '0.93',
      total: '4650',
      quote_type: 'sell',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.95', source: 'sdex:offerA' },
        { from_asset: NATIVE, to_asset: USDC, price: '0.93', source: 'amm:poolABC' },
        { from_asset: NATIVE, to_asset: USDC, price: '0.91', source: 'amm:poolDEF' },
      ],
      timestamp: 1_700_000_001,
    };

    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(threeVenueFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 5000);

    expect(spy).toHaveBeenCalledTimes(1);
    expect(result.path).toHaveLength(3);
    expect(result.total).toBe('4650');
    expect(result.path[0]?.source).toBe('sdex:offerA');
    expect(result.path[1]?.source).toBe('amm:poolABC');
    expect(result.path[2]?.source).toBe('amm:poolDEF');
  });

  it('returns single path when thin market still has enough depth for the trade', async () => {
    const singleVenueFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '100',
      price: '0.97',
      total: '97',
      quote_type: 'sell',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.97', source: 'sdex:deep-offer' },
      ],
      timestamp: 1_700_000_002,
    };

    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(singleVenueFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 100);

    expect(spy).toHaveBeenCalledTimes(1);
    expect(result.path).toHaveLength(1);
    expect(result.price).toBe('0.97');
  });

  it('handles buy quote type for thin market scenarios', async () => {
    const buyQuoteFixture = {
      base_asset: NATIVE,
      quote_asset: USDC,
      amount: '200',
      price: '0.95',
      total: '190',
      quote_type: 'buy',
      path: [
        { from_asset: NATIVE, to_asset: USDC, price: '0.95', source: 'sdex:buy-offer' },
        { from_asset: NATIVE, to_asset: USDC, price: '0.94', source: 'amm:poolBuy' },
      ],
      timestamp: 1_700_000_003,
    };

    const spy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce(ok(buyQuoteFixture));

    const client = new StellarRouteClient();
    const result = await client.getQuote('native', 'USDC:GDUKMGUGDZQK6YH...', 200, 'buy');

    expect(spy).toHaveBeenCalledTimes(1);
    expect(result.quote_type).toBe('buy');
    expect(result.path).toHaveLength(2);
  });
});