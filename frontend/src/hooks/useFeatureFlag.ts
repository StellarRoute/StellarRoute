/**
 * useFeatureFlag
 *
 * A lightweight feature-flag hook for toggling experimental UI features
 * (routes beta, swap UI experiments, etc.) without redeploy churn.
 *
 * Supports two resolution layers (in priority order):
 *  1. Remote config  – fetched from NEXT_PUBLIC_FLAGS_URL (optional)
 *  2. Environment    – NEXT_PUBLIC_FLAG_<NAME>=true/false
 *
 * All flags default to OFF unless explicitly enabled.
 */

export type FlagName =
  | "routes_beta"
  | "swap_ui_v2"
  | "transaction_history"
  | "advanced_slippage";

export type FlagMap = Partial<Record<FlagName, boolean>>;

// ─── Environment layer ────────────────────────────────────────────────────────

/**
 * Reads a single flag from Next.js public env vars.
 * Env key format: NEXT_PUBLIC_FLAG_ROUTES_BETA → flag "routes_beta"
 */
function readEnvFlag(flag: FlagName): boolean | undefined {
  const key = `NEXT_PUBLIC_FLAG_${flag.toUpperCase()}`;
  const val = process.env[key];
  if (val === undefined) return undefined;
  return val === "true" || val === "1";
}

// ─── Remote config layer ──────────────────────────────────────────────────────

let remoteFlags: FlagMap | null = null;
let remoteFetchPromise: Promise<FlagMap> | null = null;

/**
 * Fetches remote flag config once and caches it for the session.
 * Remote URL is set via NEXT_PUBLIC_FLAGS_URL.
 * Remote payload shape: { "routes_beta": true, "swap_ui_v2": false, ... }
 */
async function fetchRemoteFlags(): Promise<FlagMap> {
  if (remoteFlags !== null) return remoteFlags;
  if (remoteFetchPromise) return remoteFetchPromise;

  const url = process.env.NEXT_PUBLIC_FLAGS_URL;
  if (!url) return {};

  remoteFetchPromise = fetch(url)
    .then((res) => {
      if (!res.ok) throw new Error(`Flags fetch failed: ${res.status}`);
      return res.json() as Promise<FlagMap>;
    })
    .then((data) => {
      remoteFlags = data;
      return data;
    })
    .catch(() => {
      // Silently fall back — never break the app over flag config
      remoteFlags = {};
      return {};
    });

  return remoteFetchPromise;
}

/** Invalidate the remote cache (useful for testing or manual refresh). */
export function invalidateFlagCache(): void {
  remoteFlags = null;
  remoteFetchPromise = null;
}

// ─── Resolution ───────────────────────────────────────────────────────────────

/**
 * Resolve a flag synchronously using available sources.
 * Priority: remote (cached) > env > default (false).
 */
function resolveFlag(flag: FlagName, remote: FlagMap): boolean {
  if (remote[flag] !== undefined) return remote[flag]!;
  const env = readEnvFlag(flag);
  if (env !== undefined) return env;
  return false; // default-off for all risky/experimental features
}

// ─── Hook ─────────────────────────────────────────────────────────────────────

import { useEffect, useState } from "react";

/**
 * useFeatureFlag(flag)
 *
 * Returns { enabled, loading } for the given flag.
 * Resolves remote config on first call, then reads from cache.
 *
 * @example
 * const { enabled } = useFeatureFlag("routes_beta");
 * if (enabled) return <RoutesBeta />;
 */
export function useFeatureFlag(flag: FlagName): {
  enabled: boolean;
  loading: boolean;
} {
  const [enabled, setEnabled] = useState<boolean>(false);
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    let cancelled = false;

    fetchRemoteFlags().then((remote) => {
      if (!cancelled) {
        setEnabled(resolveFlag(flag, remote));
        setLoading(false);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [flag]);

  return { enabled, loading };
}

/**
 * useFeatureFlags(flags[])
 *
 * Batch variant — resolves multiple flags in one remote fetch.
 *
 * @example
 * const flags = useFeatureFlags(["routes_beta", "swap_ui_v2"]);
 * if (flags.routes_beta) return <RoutesBeta />;
 */
export function useFeatureFlags(flags: FlagName[]): Record<FlagName, boolean> {
  const [resolved, setResolved] = useState<Record<FlagName, boolean>>(
    () => Object.fromEntries(flags.map((f) => [f, false])) as Record<FlagName, boolean>
  );

  useEffect(() => {
    let cancelled = false;

    fetchRemoteFlags().then((remote) => {
      if (!cancelled) {
        setResolved(
          Object.fromEntries(
            flags.map((f) => [f, resolveFlag(f, remote)])
          ) as Record<FlagName, boolean>
        );
      }
    });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flags.join(",")]);

  return resolved;
}
