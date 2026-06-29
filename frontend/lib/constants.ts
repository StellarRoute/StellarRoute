export const API_PROXY_ENABLED =
  process.env.NEXT_PUBLIC_API_PROXY === 'true';

export const API_BASE_URL = API_PROXY_ENABLED
  ? '/api/v1'
  : process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080/api/v1';

/**
 * Explicit alias for API_BASE_URL that makes the /api/v1 suffix intent clear.
 * Prefer this name in new code; API_BASE_URL is kept for backwards-compat.
 */
export const API_VERSIONED_BASE = API_BASE_URL;

/**
 * Returns the bare API origin with no path suffix (no /api/v1).
 *
 * Use this for endpoints that live outside the versioned namespace, such as
 * GET /health and GET /health/deps.
 *
 * Works correctly in all three environments:
 *   - local:      http://localhost:8080
 *   - preview:    https://preview.stellarroute.xyz  (no trailing slash)
 *   - production: https://api.stellarroute.xyz      (no trailing slash)
 *   - proxy mode: "" (empty string — relative URLs, same origin)
 */
export function getApiRoot(): string {
  if (API_PROXY_ENABLED) {
    // In proxy mode the Next.js rewrite handles routing; the root is the same
    // origin, represented as an empty string so paths like "/health" work.
    return '';
  }
  // Strip any trailing /api/v1 or /api suffix, then remove trailing slashes.
  return (process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080')
    .replace(/\/api\/v\d+\/?$/, '')
    .replace(/\/+$/, '');
}

export const APP_NAME = 'StellarRoute';
export const APP_DESCRIPTION =
  'Best-price routing across Stellar DEX and Soroban AMM pools';

export const STELLAR_NETWORK =
  process.env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet';

export const ROUTES = {
  HOME: '/',
  SWAP: '/',
} as const;
