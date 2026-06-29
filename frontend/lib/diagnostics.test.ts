import { describe, expect, it } from 'vitest';

import {
  collectQuoteDiagnostics,
  computeQuoteAgeMs,
  exportDiagnosticsAsCsv,
  exportDiagnosticsAsJson,
  formatDiagnosticsForDisplay,
  generateRequestId,
  redactDiagnosticsObject,
  redactSensitiveFields,
} from '@/lib/diagnostics';
import type { PriceQuote } from '@/types';

const mockQuote: PriceQuote = {
  base_asset: {
    asset_type: 'native',
  },
  quote_asset: {
    asset_type: 'credit_alphanum4',
    asset_code: 'USDC',
    asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
  },
  amount: '100.00',
  price: '0.105',
  total: '10.50',
  quote_type: 'sell',
  path: [
    {
      from_asset: { asset_type: 'native' },
      to_asset: {
        asset_type: 'credit_alphanum4',
        asset_code: 'USDC',
        asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
      },
      price: '0.105',
      source: 'sdex',
    },
  ],
  timestamp: Date.now(),
};

describe('diagnostics utilities', () => {
  describe('generateRequestId', () => {
    it('generates unique request IDs', () => {
      const id1 = generateRequestId();
      const id2 = generateRequestId();
      expect(id1).not.toEqual(id2);
      expect(id1).toMatch(/^req_\d+_[a-z0-9]+$/);
    });
  });

  describe('redactSensitiveFields', () => {
    it('redacts Stellar addresses', () => {
      const input =
        'Address: GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU';
      const output = redactSensitiveFields(input);
      expect(output).toContain('[REDACTED_ADDRESS]');
      expect(output).not.toContain('GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU');
    });

    it('redacts issuer in asset identifiers', () => {
      const input = 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU';
      const output = redactSensitiveFields(input);
      expect(output).toContain('USDC:[REDACTED]');
    });

    it('leaves non-sensitive data unchanged', () => {
      const input = 'Quote for 100 XLM at price 0.105';
      const output = redactSensitiveFields(input);
      expect(output).toEqual(input);
    });
  });

  describe('computeQuoteAgeMs', () => {
    it('uses lastQuotedAtMs when provided', () => {
      const lastQuotedAtMs = Date.now() - 5_000;
      const age = computeQuoteAgeMs(mockQuote, lastQuotedAtMs);
      expect(age).toBeGreaterThanOrEqual(4_900);
      expect(age).toBeLessThan(6_000);
    });
  });

  describe('collectQuoteDiagnostics', () => {
    it('collects diagnostics from quote response', () => {
      const requestId = 'req_test_123';
      const diag = collectQuoteDiagnostics(mockQuote, requestId);

      expect(diag.requestId).toEqual(requestId);
      expect(diag.base).toEqual('XLM');
      expect(diag.quote).toEqual('USDC');
      expect(diag.amount).toEqual('100.00');
      expect(diag.price).toEqual('0.105');
      expect(diag.total).toEqual('10.50');
      expect(diag.pathLength).toEqual(1);
      expect(diag.sources).toContain('SDEX');
    });

    it('calculates quote age correctly', () => {
      const requestId = 'req_test_456';
      const lastQuotedAtMs = Date.now() - 2_000;
      const diag = collectQuoteDiagnostics(mockQuote, requestId, {
        lastQuotedAtMs,
      });

      expect(diag.quoteAge).toBeGreaterThanOrEqual(1_900);
      expect(typeof diag.quoteAge).toBe('number');
    });

    it('extracts AMM sources correctly', () => {
      const quoteWithAmm = {
        ...mockQuote,
        path: [
          {
            from_asset: { asset_type: 'native' },
            to_asset: {
              asset_type: 'credit_alphanum4',
              asset_code: 'USDC',
              asset_issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
            },
            price: '0.105',
            source: 'amm:abc123def456',
          },
        ],
      };

      const diag = collectQuoteDiagnostics(quoteWithAmm, 'req_amm_test');
      expect(diag.sources).toContain('AMM');
    });

    it('extracts route diagnostics from quote metadata', () => {
      const quoteWithDiagnostics: PriceQuote = {
        ...mockQuote,
        degraded: true,
        rationale: {
          strategy: 'single_hop_direct_venue_comparison',
          selected_source: 'sdex:offer-1',
          compared_venues: [
            {
              source: 'sdex:offer-1',
              price: '0.105',
              available_amount: '1000',
              executable: true,
            },
          ],
        },
        exclusion_diagnostics: {
          excluded_venues: [
            {
              venue_ref: 'amm:pool-1',
              reason: { type: 'stale_data' },
            },
          ],
        },
        data_freshness: {
          fresh_count: 2,
          stale_count: 1,
          max_staleness_secs: 45,
        },
      };

      const diag = collectQuoteDiagnostics(quoteWithDiagnostics, 'req_route_test');
      expect(diag.degraded).toBe(true);
      expect(diag.routeDiagnostics?.strategy).toBe(
        'single_hop_direct_venue_comparison',
      );
      expect(diag.routeDiagnostics?.selectedSource).toBe('sdex:offer-1');
      expect(diag.routeDiagnostics?.comparedVenuesCount).toBe(1);
      expect(diag.routeDiagnostics?.excludedVenues).toHaveLength(1);
      expect(diag.routeDiagnostics?.dataFreshness?.freshCount).toBe(2);
    });
  });

  describe('formatDiagnosticsForDisplay', () => {
    it('formats diagnostics as readable text', () => {
      const requestId = 'req_display_test';
      const diag = collectQuoteDiagnostics(mockQuote, requestId);
      const formatted = formatDiagnosticsForDisplay(diag);

      expect(formatted).toContain('Request ID:');
      expect(formatted).toContain('Quote Age:');
      expect(formatted).toContain('Route:');
      expect(formatted).toContain('Type: SELL');
      expect(formatted).toContain('Price:');
      expect(formatted).toContain('Total:');
    });

    it('formats quote age in seconds', () => {
      const requestId = 'req_age_test';
      const diag = collectQuoteDiagnostics(mockQuote, requestId, {
        lastQuotedAtMs: Date.now() - 1_500,
      });
      const formatted = formatDiagnosticsForDisplay(diag);

      expect(formatted).toMatch(/Quote Age: \d+\.\d{2}s/);
    });

    it('includes route diagnostics section when present', () => {
      const quoteWithDiagnostics: PriceQuote = {
        ...mockQuote,
        rationale: {
          strategy: 'single_hop_direct_venue_comparison',
          selected_source: 'sdex:offer-1',
          compared_venues: [],
        },
      };
      const diag = collectQuoteDiagnostics(quoteWithDiagnostics, 'req_route_fmt');
      const formatted = formatDiagnosticsForDisplay(diag);

      expect(formatted).toContain('Route Diagnostics:');
      expect(formatted).toContain('Strategy:');
    });
  });

  describe('exportDiagnosticsAsJson', () => {
    it('exports diagnostics as valid JSON', () => {
      const requestId = 'req_json_test';
      const diag = collectQuoteDiagnostics(mockQuote, requestId);
      const json = exportDiagnosticsAsJson(diag);

      expect(() => JSON.parse(json)).not.toThrow();
      const parsed = JSON.parse(json);
      expect(parsed.requestId).toEqual(requestId);
      expect(parsed.amount).toEqual('100.00');
    });

    it('redacts sensitive fields in JSON export', () => {
      const quoteWithIssuer: PriceQuote = {
        ...mockQuote,
        rationale: {
          strategy: 'test',
          selected_source:
            'sdex:GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
          compared_venues: [],
        },
      };
      const diag = collectQuoteDiagnostics(quoteWithIssuer, 'req_redact_json');
      const json = exportDiagnosticsAsJson(diag);

      expect(json).not.toContain('GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU');
      expect(json).toContain('[REDACTED');
    });
  });

  describe('exportDiagnosticsAsCsv', () => {
    it('exports diagnostics as valid CSV', () => {
      const requestId = 'req_csv_test';
      const diag = collectQuoteDiagnostics(mockQuote, requestId);
      const csv = exportDiagnosticsAsCsv(diag);

      const lines = csv.split('\n');
      expect(lines.length).toBe(2);
      expect(lines[0]).toContain('requestId');
      expect(lines[1]).toContain(requestId);
    });
  });

  describe('redactDiagnosticsObject', () => {
    it('redacts nested route diagnostics fields', () => {
      const diag = collectQuoteDiagnostics(
        {
          ...mockQuote,
          rationale: {
            strategy: 'test',
            selected_source:
              'sdex:GA5ZSEJYB37JRC5AVCIA5MOP4IHTZMAB5KYXOM5KBVG7GBJINW7JCXU',
            compared_venues: [],
          },
        },
        'req_obj_redact',
      );

      const redacted = redactDiagnosticsObject(diag);
      expect(redacted.routeDiagnostics?.selectedSource).toContain('[REDACTED');
    });
  });
});
