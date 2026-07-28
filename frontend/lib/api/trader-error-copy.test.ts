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

  describe('Horizon and Soroban failure code mapping', () => {
    const cases = [
      {
        code: 'tx_bad_seq',
        headline: 'Account sequence is out of date',
        ctaLabel: 'Refresh and retry',
      },
      {
        code: 'op_no_trust',
        headline: 'Missing trustline for this asset',
        ctaLabel: 'Add trustline and retry',
      },
      {
        code: 'op_underfunded',
        headline: 'Insufficient funds for this swap',
        ctaLabel: 'Adjust amount',
      },
      {
        code: 'op_line_full',
        headline: 'Trustline limit reached for this asset',
        ctaLabel: 'Adjust trustline or amount',
      },
      {
        code: 'op_low_reserve',
        headline: 'Minimum account reserve required',
        ctaLabel: 'Add funds and retry',
      },
      {
        code: 'op_no_issuer',
        headline: 'Asset issuer could not be found',
        ctaLabel: 'Select another pair',
      },
      {
        code: 'op_no_destination',
        headline: 'Destination account does not exist',
        ctaLabel: 'Check destination and retry',
      },
      {
        code: 'tx_insufficient_balance',
        headline: 'Not enough balance to cover this trade',
        ctaLabel: 'Adjust amount',
      },
      {
        code: 'tx_insufficient_fee',
        headline: 'Network fee was too low',
        ctaLabel: 'Refresh and resubmit',
      },
      {
        code: 'tx_too_late',
        headline: 'This quote expired before it was submitted',
        ctaLabel: 'Refresh quote',
      },
      {
        code: 'invoke_host_function_trapped',
        headline: 'The swap contract could not complete this trade',
        ctaLabel: 'Adjust trade and retry',
      },
      {
        code: 'invoke_host_function_resource_limit_exceeded',
        headline: 'This trade is too complex to execute right now',
        ctaLabel: 'Simplify trade',
      },
      {
        code: 'invoke_host_function_entry_archived',
        headline: 'Contract data needs to be restored first',
        ctaLabel: 'Refresh quote',
      },
    ] as const;

    for (const { code, headline, ctaLabel } of cases) {
      it(`maps raw Horizon/Soroban message containing "${code}" to trader-facing copy`, () => {
        const copy = getTraderErrorCopy(
          new Error(`transaction failed: result_codes: { transaction: ${code} }`),
        );

        expect(copy.headline).toBe(headline);
        expect(copy.ctaLabel).toBe(ctaLabel);
        expect(copy.explanation.length).toBeGreaterThan(0);
        expect(copy.recoveryAction.length).toBeGreaterThan(0);
      });
    }

    it('matches Horizon/Soroban codes case-insensitively', () => {
      const copy = getTraderErrorCopy(new Error('TX_BAD_SEQ'));
      expect(copy.headline).toBe('Account sequence is out of date');
    });

    it('maps a generic timeout message when no specific code is present', () => {
      const copy = getTraderErrorCopy(new Error('Transaction timed out'));
      expect(copy.headline).toBe('Transaction timed out');
    });

    it('prefers a specific Horizon/Soroban code over the generic timeout match', () => {
      const copy = getTraderErrorCopy(
        new Error('op_line_full: transaction timed out waiting for confirmation'),
      );
      expect(copy.headline).toBe('Trustline limit reached for this asset');
    });

    it('does not use blame or panic language in any mapped copy', () => {
      const blameOrPanicWords = ['you failed', 'invalid user', 'fatal', 'catastrophic', 'critical failure'];

      for (const { code } of cases) {
        const copy = getTraderErrorCopy(new Error(code));
        const combined = `${copy.headline} ${copy.explanation} ${copy.recoveryAction}`.toLowerCase();

        for (const word of blameOrPanicWords) {
          expect(combined).not.toContain(word);
        }
      }
    });
  });
});
