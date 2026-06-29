import { describe, expect, it } from 'vitest';

import { StellarRouteApiError } from '@/lib/api/client';
import {
  getTraderErrorCopy,
  toTraderErrorLine,
} from '@/lib/api/trader-error-copy';

describe('getTraderErrorCopy', () => {
  it('maps all top 10 API error codes to trader-facing copy', () => {
    const cases = [
      { code: 'validation_error', headline: 'Check your trade details' },
      { code: 'invalid_asset', headline: 'This asset pair is not available right now' },
      { code: 'no_route', headline: 'No executable route found' },
      { code: 'stale_market_data', headline: 'Market data is still updating' },
      { code: 'rate_limit_exceeded', headline: 'Quote refresh is temporarily limited' },
      { code: 'overloaded', headline: 'Quote service is handling high traffic' },
      { code: 'bad_request', headline: 'We could not process this request' },
      { code: 'unauthorized', headline: 'Session check required' },
      { code: 'not_found', headline: 'Requested market data was not found' },
      { code: 'internal_error', headline: 'Quote service hit an internal issue' },
    ] as const;

    for (const { code, headline } of cases) {
      const error = new StellarRouteApiError(400, code as any, 'Test');
      const copy = getTraderErrorCopy(error);
      expect(copy.headline).toBe(headline);
    }
  });

  it('maps raw HTTP status codes correctly when error code is unknown_error', () => {
    const cases = [
      { status: 400, headline: 'We could not process this request' }, // bad_request
      { status: 401, headline: 'Session check required' }, // unauthorized
      { status: 404, headline: 'Requested market data was not found' }, // not_found
      { status: 429, headline: 'Quote refresh is temporarily limited' }, // rate_limit_exceeded
      { status: 500, headline: 'Quote service hit an internal issue' }, // internal_error
      { status: 503, headline: 'Quote service hit an internal issue' }, // internal_error
    ];

    for (const { status, headline } of cases) {
      const error = new StellarRouteApiError(status, 'unknown_error', 'Test');
      const copy = getTraderErrorCopy(error);
      expect(copy.headline).toBe(headline);
    }
  });

  it('maps generic object with status property', () => {
    const cases = [
      { status: 400, headline: 'We could not process this request' },
      { status: 429, headline: 'Quote refresh is temporarily limited' },
      { status: 500, headline: 'Quote service hit an internal issue' },
    ];

    for (const { status, headline } of cases) {
      const error = { status };
      const copy = getTraderErrorCopy(error);
      expect(copy.headline).toBe(headline);
    }
  });

  it('falls back to safe default for truly unknown situations', () => {
    const error = new StellarRouteApiError(
      418,
      'unknown_error',
      'Unexpected upstream error',
    );

    const copy = getTraderErrorCopy(error);

    expect(copy.headline).toBe('We could not refresh this quote');
  });

  it('infers wallet copy from generic wallet rejection errors', () => {
    const copy = getTraderErrorCopy(new Error('Freighter rejected signature request'));

    expect(copy.headline).toBe('Wallet action was not completed');
    expect(copy.ctaLabel).toBe('Open wallet and retry');
  });

  it('infers network copy from transport failures', () => {
    const copy = getTraderErrorCopy(new Error('Failed to fetch'));

    expect(copy.headline).toBe('Network connection interrupted');
  });

  it('formats copy into a single display line', () => {
    const copy = getTraderErrorCopy(
      new StellarRouteApiError(400, 'validation_error', 'Invalid request'),
    );

    expect(toTraderErrorLine(copy)).toContain('Check your trade details.');
    expect(toTraderErrorLine(copy)).not.toContain('—');
  });
});
