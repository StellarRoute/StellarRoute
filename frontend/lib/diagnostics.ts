/**
 * Diagnostics utilities for quote and route metadata collection
 * Provides functionality to track, format, and export quote diagnostics
 */

import type {
  ExcludedVenueInfo,
  ExclusionReason,
  PathStep,
  PriceQuote,
} from '@/types';

/**
 * Route-level diagnostics extracted from quote metadata
 */
export interface RouteDiagnostics {
  strategy?: string;
  selectedSource?: string;
  comparedVenuesCount?: number;
  excludedVenues?: { venueRef: string; reason: string }[];
  dataFreshness?: {
    freshCount: number;
    staleCount: number;
    maxStalenessSecs: number;
  };
}

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
  degraded?: boolean;
  routeDiagnostics?: RouteDiagnostics;
}

/**
 * Redacts sensitive information from diagnostics data
 * Redacts wallet addresses and issuer information
 */
export function redactSensitiveFields(data: string): string {
  data = data.replace(/([A-Z0-9]{1,12}):[G][A-Z0-9]{54}/g, '$1:[REDACTED]');
  data = data.replace(/\bG[A-Z0-9]{54}\b/g, '[REDACTED_ADDRESS]');
  data = data.replace(/\b[a-f0-9]{64}\b/i, '[REDACTED_HASH]');
  return data;
}

/**
 * Redacts sensitive fields from a diagnostics object for export
 */
export function redactDiagnosticsObject(diag: QuoteDiagnostics): QuoteDiagnostics {
  const redact = (value: string) => redactSensitiveFields(value);

  return {
    ...diag,
    base: redact(diag.base),
    quote: redact(diag.quote),
    amount: redact(diag.amount),
    price: redact(diag.price),
    total: redact(diag.total),
    sources: diag.sources.map(redact),
    routeDiagnostics: diag.routeDiagnostics
      ? {
          ...diag.routeDiagnostics,
          strategy: diag.routeDiagnostics.strategy
            ? redact(diag.routeDiagnostics.strategy)
            : undefined,
          selectedSource: diag.routeDiagnostics.selectedSource
            ? redact(diag.routeDiagnostics.selectedSource)
            : undefined,
          excludedVenues: diag.routeDiagnostics.excludedVenues?.map((v) => ({
            venueRef: redact(v.venueRef),
            reason: redact(v.reason),
          })),
        }
      : undefined,
  };
}

/**
 * Generates a unique request ID for diagnostics tracking
 */
export function generateRequestId(): string {
  return `req_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;
}

function formatExclusionReason(reason: ExclusionReason): string {
  switch (reason.type) {
    case 'policy_threshold':
      return `policy_threshold(${reason.threshold})`;
    case 'override':
      return 'override';
    case 'stale_data':
      return 'stale_data';
    case 'circuit_breaker_open':
      return 'circuit_breaker_open';
    case 'liquidity_anomaly':
      return 'liquidity_anomaly';
    default:
      return 'unknown';
  }
}

function extractRouteDiagnostics(quote: PriceQuote): RouteDiagnostics | undefined {
  const hasRationale = Boolean(quote.rationale);
  const hasExclusions = Boolean(quote.exclusion_diagnostics?.excluded_venues?.length);
  const hasFreshness = Boolean(quote.data_freshness);

  if (!hasRationale && !hasExclusions && !hasFreshness) {
    return undefined;
  }

  const routeDiagnostics: RouteDiagnostics = {};

  if (quote.rationale) {
    routeDiagnostics.strategy = quote.rationale.strategy;
    routeDiagnostics.selectedSource = quote.rationale.selected_source;
    routeDiagnostics.comparedVenuesCount = quote.rationale.compared_venues.length;
  }

  if (quote.exclusion_diagnostics?.excluded_venues?.length) {
    routeDiagnostics.excludedVenues = quote.exclusion_diagnostics.excluded_venues.map(
      (venue: ExcludedVenueInfo) => ({
        venueRef: venue.venue_ref,
        reason: formatExclusionReason(venue.reason),
      }),
    );
  }

  if (quote.data_freshness) {
    routeDiagnostics.dataFreshness = {
      freshCount: quote.data_freshness.fresh_count,
      staleCount: quote.data_freshness.stale_count,
      maxStalenessSecs: quote.data_freshness.max_staleness_secs,
    };
  }

  return routeDiagnostics;
}

/**
 * Computes quote age in milliseconds from client fetch time or quote timestamp
 */
export function computeQuoteAgeMs(
  quote: PriceQuote,
  lastQuotedAtMs?: number | null,
): number {
  if (lastQuotedAtMs != null) {
    return Math.max(0, Date.now() - lastQuotedAtMs);
  }

  const quoteTimestampMs =
    quote.timestamp > 1_000_000_000_000 ? quote.timestamp : quote.timestamp * 1000;

  return Math.max(0, Date.now() - quoteTimestampMs);
}

/**
 * Collects diagnostics metadata from a quote response
 */
export function collectQuoteDiagnostics(
  quote: PriceQuote,
  requestId: string,
  options?: { lastQuotedAtMs?: number | null },
): QuoteDiagnostics {
  const timestamp = Date.now();
  const quoteAge = computeQuoteAgeMs(quote, options?.lastQuotedAtMs);

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
    degraded: quote.degraded,
    routeDiagnostics: extractRouteDiagnostics(quote),
  };
}

/**
 * Formats diagnostics data for display
 */
export function formatDiagnosticsForDisplay(diag: QuoteDiagnostics): string {
  const quoteAgeSeconds = (diag.quoteAge / 1000).toFixed(2);
  const routeInfo = `${diag.pathLength} step${diag.pathLength !== 1 ? 's' : ''}`;

  const lines = [
    `Request ID: ${diag.requestId}`,
    `Quote Age: ${quoteAgeSeconds}s`,
    `Route: ${routeInfo} (${diag.sources.join(', ')})`,
    `Type: ${diag.type.toUpperCase()} ${diag.amount} ${diag.base}`,
    `Price: ${diag.price} ${diag.quote}/${diag.base}`,
    `Total: ${diag.total} ${diag.quote}`,
  ];

  if (diag.degraded) {
    lines.push('Status: DEGRADED');
  }

  if (diag.routeDiagnostics) {
    lines.push('', 'Route Diagnostics:');
    if (diag.routeDiagnostics.strategy) {
      lines.push(`  Strategy: ${diag.routeDiagnostics.strategy}`);
    }
    if (diag.routeDiagnostics.selectedSource) {
      lines.push(`  Selected Source: ${diag.routeDiagnostics.selectedSource}`);
    }
    if (diag.routeDiagnostics.comparedVenuesCount !== undefined) {
      lines.push(`  Compared Venues: ${diag.routeDiagnostics.comparedVenuesCount}`);
    }
    if (diag.routeDiagnostics.dataFreshness) {
      const { freshCount, staleCount, maxStalenessSecs } =
        diag.routeDiagnostics.dataFreshness;
      lines.push(
        `  Data Freshness: ${freshCount} fresh, ${staleCount} stale (max ${maxStalenessSecs}s)`,
      );
    }
    if (diag.routeDiagnostics.excludedVenues?.length) {
      lines.push('  Excluded Venues:');
      for (const venue of diag.routeDiagnostics.excludedVenues) {
        lines.push(`    - ${venue.venueRef}: ${venue.reason}`);
      }
    }
  }

  return lines.join('\n');
}

/**
 * Exports diagnostics data as JSON with sensitive fields redacted
 */
export function exportDiagnosticsAsJson(diag: QuoteDiagnostics): string {
  return JSON.stringify(redactDiagnosticsObject(diag), null, 2);
}

/**
 * Exports diagnostics data as CSV with sensitive fields redacted
 */
export function exportDiagnosticsAsCsv(diag: QuoteDiagnostics): string {
  const redacted = redactDiagnosticsObject(diag);
  const flat: Record<string, string | number | boolean> = {
    requestId: redacted.requestId,
    timestamp: redacted.timestamp,
    quoteAge: redacted.quoteAge,
    base: redacted.base,
    quote: redacted.quote,
    amount: redacted.amount,
    type: redacted.type,
    price: redacted.price,
    total: redacted.total,
    pathLength: redacted.pathLength,
    sources: redacted.sources.join(';'),
    degraded: redacted.degraded ?? false,
    routeStrategy: redacted.routeDiagnostics?.strategy ?? '',
    selectedSource: redacted.routeDiagnostics?.selectedSource ?? '',
    comparedVenuesCount: redacted.routeDiagnostics?.comparedVenuesCount ?? '',
    freshCount: redacted.routeDiagnostics?.dataFreshness?.freshCount ?? '',
    staleCount: redacted.routeDiagnostics?.dataFreshness?.staleCount ?? '',
    maxStalenessSecs:
      redacted.routeDiagnostics?.dataFreshness?.maxStalenessSecs ?? '',
    excludedVenues: redacted.routeDiagnostics?.excludedVenues
      ?.map((v) => `${v.venueRef}:${v.reason}`)
      .join(';') ?? '',
  };

  const headers = Object.keys(flat).join(',');
  const values = Object.values(flat)
    .map((v) => {
      if (typeof v === 'string') {
        return `"${v.replace(/"/g, '""')}"`;
      }
      return v;
    })
    .join(',');

  return `${headers}\n${values}`;
}
