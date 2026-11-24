import type { QuoteExportPayload } from '@/lib/quote-export';

/**
 * Client-side encode/decode/validate helpers for passing an already-fetched
 * quote into the print route via a URL query param.
 *
 * No API calls are made here. The payload shape re-uses the existing
 * `QuoteExportPayload` type from `lib/quote-export.ts` (the same shape the
 * JSON/CSV export buttons already produce), so any caller that can build a
 * JSON/CSV export can also build a print link with the same object.
 */

const QUERY_PARAM = 'data';

function toBase64Url(input: string): string {
  const bytes = new TextEncoder().encode(input);
  let binary = '';
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
}

function fromBase64Url(input: string): string {
  const base64 = input.replace(/-/g, '+').replace(/_/g, '/');
  const padded = base64 + '='.repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

/**
 * Structural check for `QuoteExportPayload`. Intentionally hand-rolled
 * (no new validation dependency) since every field is a plain string.
 */
export function isQuoteExportPayload(
  value: unknown
): value is QuoteExportPayload {
  if (!value || typeof value !== 'object') return false;
  const v = value as Record<string, unknown>;
  if (!isNonEmptyString(v.exportedAt)) return false;

  const market = v.market as Record<string, unknown> | undefined;
  if (!market || typeof market !== 'object') return false;
  if (
    !isNonEmptyString(market.fromAsset) ||
    !isNonEmptyString(market.toAsset) ||
    !isNonEmptyString(market.fromAmount) ||
    !isNonEmptyString(market.expectedToAmount)
  ) {
    return false;
  }

  const pricing = v.pricing as Record<string, unknown> | undefined;
  if (!pricing || typeof pricing !== 'object') return false;
  if (
    !isNonEmptyString(pricing.rate) ||
    !isNonEmptyString(pricing.priceImpactPct) ||
    !isNonEmptyString(pricing.minimumReceived) ||
    !isNonEmptyString(pricing.networkFee)
  ) {
    return false;
  }

  const route = v.route as Record<string, unknown> | undefined;
  if (!route || typeof route !== 'object') return false;
  if (
    !isNonEmptyString(route.selectedVenue) ||
    !isNonEmptyString(route.routeSummary)
  ) {
    return false;
  }

  return true;
}

/** Builds a `/quote/print?data=...` URL (relative) for a given quote payload. */
export function buildQuotePrintUrl(payload: QuoteExportPayload): string {
  const encoded = toBase64Url(JSON.stringify(payload));
  const params = new URLSearchParams();
  params.set(QUERY_PARAM, encoded);
  return `/quote/print?${params.toString()}`;
}

/**
 * Decodes and validates the `data` query param. Returns `null` (never
 * throws) when the param is missing, malformed, or does not match the
 * expected shape — callers should render an empty/fallback state in that
 * case rather than crash the page.
 */
export function parseQuotePrintPayload(
  encoded: string | null
): QuoteExportPayload | null {
  if (!encoded) return null;
  try {
    const json = fromBase64Url(encoded);
    const parsed: unknown = JSON.parse(json);
    return isQuoteExportPayload(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export { QUERY_PARAM as QUOTE_PRINT_QUERY_PARAM };
