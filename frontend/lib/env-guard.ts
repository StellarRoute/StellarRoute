/**
 * Production env guard for the Next.js frontend (issue #1036).
 *
 * In production (Vercel production or STELLARROUTE_ENV=production), the public
 * API URL must be set and must not point at localhost.
 * Development may keep localhost defaults.
 */

export type EnvGuardResult =
  | { ok: true }
  | { ok: false; message: string };

function isTruthy(value: string | undefined): boolean {
  if (!value) return false;
  const v = value.trim().toLowerCase();
  return v === '1' || v === 'true' || v === 'yes' || v === 'on' || v === 'production';
}

/** True when this build/runtime must reject localhost API URLs. */
export function isProductionFrontendEnv(
  env: NodeJS.ProcessEnv = process.env
): boolean {
  if (isTruthy(env.STELLARROUTE_ENV) && env.STELLARROUTE_ENV?.toLowerCase() === 'production') {
    return true;
  }
  if (env.VERCEL_ENV === 'production') {
    return true;
  }
  if (env.NODE_ENV === 'production' && env.VERCEL_ENV !== 'preview' && env.VERCEL_ENV !== 'development') {
    // Plain `next build` / `next start` without Vercel: treat as production
    // only when STELLARROUTE_ENV=production (avoid breaking local prod builds
    // used for smoke). Vercel production is already covered above.
    return env.STELLARROUTE_ENV?.toLowerCase() === 'production';
  }
  return false;
}

function isLocalhostUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.toLowerCase();
    return (
      host === 'localhost' ||
      host === '127.0.0.1' ||
      host === '0.0.0.0' ||
      host === '::1' ||
      host.endsWith('.localhost')
    );
  } catch {
    return /localhost|127\.0\.0\.1/i.test(url);
  }
}

/**
 * Resolve the critical public API URL used for production builds.
 * Prefers per-network testnet URL when STELLAR network is testnet.
 */
export function resolveCriticalApiUrl(
  env: NodeJS.ProcessEnv = process.env
): string | undefined {
  const network = (env.NEXT_PUBLIC_STELLAR_NETWORK || 'testnet').toLowerCase();
  const candidates =
    network === 'mainnet'
      ? [env.NEXT_PUBLIC_API_URL_MAINNET, env.NEXT_PUBLIC_API_URL]
      : [env.NEXT_PUBLIC_API_URL_TESTNET, env.NEXT_PUBLIC_API_URL];

  for (const c of candidates) {
    const trimmed = c?.trim();
    if (trimmed) return trimmed;
  }
  return undefined;
}

/**
 * Validate frontend env for the current deployment profile.
 * Call from next.config at build time and from unit tests.
 */
export function assertFrontendProductionEnv(
  env: NodeJS.ProcessEnv = process.env
): EnvGuardResult {
  if (!isProductionFrontendEnv(env)) {
    return { ok: true };
  }

  const apiUrl = resolveCriticalApiUrl(env);
  if (!apiUrl) {
    return {
      ok: false,
      message:
        'Production frontend requires NEXT_PUBLIC_API_URL (or NEXT_PUBLIC_API_URL_TESTNET / _MAINNET). Localhost defaults are not allowed.',
    };
  }

  if (isLocalhostUrl(apiUrl)) {
    return {
      ok: false,
      message: `Production frontend API URL must not point at localhost (got "${apiUrl}"). Set NEXT_PUBLIC_API_URL to the public staging/production API.`,
    };
  }

  try {
    // eslint-disable-next-line no-new
    new URL(apiUrl);
  } catch {
    return {
      ok: false,
      message: `Production frontend API URL is not a valid URL: "${apiUrl}"`,
    };
  }

  return { ok: true };
}

/** Throws if production env is invalid — used by next.config.ts. */
export function enforceFrontendProductionEnv(
  env: NodeJS.ProcessEnv = process.env
): void {
  const result = assertFrontendProductionEnv(env);
  if (!result.ok) {
    throw new Error(`[env-guard] ${result.message}`);
  }
}
