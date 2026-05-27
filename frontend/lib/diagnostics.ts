/**
 * Diagnostics utilities for quote and route metadata collection
 * Provides functionality to track, format, and export quote diagnostics
 */

import type { PathStep, PriceQuote } from '@/types';

/**
 * Represents metadata about a quote request for diagnostics
 */
export interface QuoteDiagnostics {
  requestId: string;
  timestamp: number;
  quoteAge: number;
  base: string;
  quote: string;
  amount: string;
  type: 'sell' | 'buy';
  price: string;
  total: string;
  pathLength: number;
  sources: string[];
}

/**
 * Redacts sensitive information from diagnostics data
 * Redacts wallet addresses and issuer information
 */
export function redactSensitiveFields(data: string): string {
  // Redact full issuer addresses
  data = data.replace(/([A-Z0-9]{1,12}):[G][A-Z0-9]{54}/g, '$1:[REDACTED]');
  // Redact full wallet addresses
  data = data.replace(/\bG[A-Z0-9]{54}\b/g, '[REDACTED_ADDRESS]');
  // Redact any hex strings that look like hashes
  data = data.replace(/\b[a-f0-9]{64}\b/i, '[REDACTED_HASH]');
  return data;
}

/**
 * Generates a unique request ID for diagnostics tracking
 */
export function generateRequestId(): string {
  return `req_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

/**
 * Collects diagnostics metadata from a quote response
 */
export function collectQuoteDiagnostics(
  quote: PriceQuote,
  requestId: string,
): QuoteDiagnostics {
  const timestamp = Date.now();
  const quoteTimestamp = quote.timestamp * 1000;
  const quoteAge = timestamp - quoteTimestamp;

  const baseSymbol = quote.base_asset.asset_code || 'XLM';
  const quoteSymbol = quote.quote_asset.asset_code || 'native';

  const sources = [...new Set(quote.path.map((step: PathStep) => {
    if (step.source.startsWith('amm:')) {
      return 'AMM';
    }
    return step.source.toUpperCase();
  }))];

  return {
    requestId,
    timestamp,
    quoteAge,
    base: baseSymbol,
    quote: quoteSymbol,
    amount: quote.amount,
    type: quote.quote_type,
    price: quote.price,
    total: quote.total,
    pathLength: quote.path.length,
    sources,
  };
}

/**
 * Formats diagnostics data for display
 */
export function formatDiagnosticsForDisplay(diag: QuoteDiagnostics): string {
  const quoteAgeSeconds = (diag.quoteAge / 1000).toFixed(2);
  const routeInfo = `${diag.pathLength} step${diag.pathLength !== 1 ? 's' : ''}`;

  return `Request ID: ${diag.requestId}
Quote Age: ${quoteAgeSeconds}s
Route: ${routeInfo} (${diag.sources.join(', ')})
Type: ${diag.type.toUpperCase()} ${diag.amount} ${diag.base}
Price: ${diag.price} ${diag.quote}/${diag.base}
Total: ${diag.total} ${diag.quote}`;
}

/**
 * Exports diagnostics data as JSON
 */
export function exportDiagnosticsAsJson(diag: QuoteDiagnostics): string {
  return JSON.stringify(diag, null, 2);
}

/**
 * Exports diagnostics data as CSV
 */
export function exportDiagnosticsAsCsv(diag: QuoteDiagnostics): string {
  const headers = Object.keys(diag).join(',');
  const values = Object.values(diag)
    .map((v) => {
      if (typeof v === 'string') {
        return `"${v.replace(/"/g, '""')}"`;
      }
      return v;
    })
    .join(',');
  return `${headers}\n${values}`;
}
