/**
 * Quote freshness / stale UI timing.
 *
 * API `expires_at` / `ttl_seconds` track the **server cache TTL** (often 2s via
 * `QUOTE_CACHE_TTL_SECONDS`), not a trading safety window. UI stale detection
 * therefore uses a client receive-time floor so a 2s cache expiry cannot brick
 * the swap CTA between auto-refresh ticks.
 */
export const QUOTE_STALE_AFTER_MS = 5500;

/** Debounce before firing a quote request while the user edits amount. */
export const QUOTE_AMOUNT_DEBOUNCE_MS = 450;

/**
 * Default interval when auto-refresh is enabled (15–30s range per product guidance).
 */
export const QUOTE_AUTO_REFRESH_INTERVAL_MS = 20_000;

/**
 * Minimum spacing between **manual** refresh clicks to avoid hammering the API.
 * Auto-refresh uses {@link QUOTE_AUTO_REFRESH_INTERVAL_MS} instead.
 */
export const QUOTE_MANUAL_REFRESH_COOLDOWN_MS = 2000;

/**
 * Returns true when a successful quote is older than the UI stale window.
 *
 * Uses the **later** of:
 * - client receive-time + `staleAfterMs`
 * - server `expires_at` (when still ahead of receive time)
 *
 * So a short cache TTL (2s) cannot mark a quote stale before the client floor,
 * while a longer server expiry can keep the quote fresh. Already-past
 * `expires_at` values are ignored (cached responses) and fall back to the
 * client window — otherwise the CTA stays bricked after session restore.
 */
export function isQuoteStale(
  lastSuccessTimeMs: number | null,
  nowMs: number,
  staleAfterMs: number = QUOTE_STALE_AFTER_MS,
  expiresAtMs?: number,
): boolean {
  if (lastSuccessTimeMs == null) return false;

  const clientStaleAtMs = lastSuccessTimeMs + staleAfterMs;

  if (
    expiresAtMs != null &&
    Number.isFinite(expiresAtMs) &&
    expiresAtMs > lastSuccessTimeMs
  ) {
    return nowMs >= Math.max(expiresAtMs, clientStaleAtMs);
  }

  return nowMs >= clientStaleAtMs;
}
