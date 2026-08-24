'use client';

import { useEffect, useState } from 'react';

export type FlagName =
  | "routes_beta"
  | "batch_swaps"
  | "swap_ui_v2"
  | "transaction_history"
  | "advanced_slippage"
  | "real_xdr"
  | "analytics";

export type FlagMap = Partial<Record<FlagName, boolean>>;

/** Security-critical: secure API swap path — not remotely killable. */
export const SECURITY_PINNED_FLAGS: ReadonlySet<FlagName> = new Set(['real_xdr']);

function defaultFlagValue(flag: FlagName): boolean {
  return flag === 'swap_ui_v2';
}

// Cache layer
let remoteFlags: FlagMap | null = null;
let remoteFetchPromise: Promise<FlagMap> | null = null;

export function invalidateFlagCache(): void {
  remoteFlags = null;
  remoteFetchPromise = null;
}

function readEnvFlag(flag: FlagName): boolean | undefined {
  // Static property access is required for Next.js to expose public env values
  // in the browser bundle.
  const val =
    flag === 'routes_beta'
      ? process.env.NEXT_PUBLIC_FLAG_ROUTES_BETA
      : flag === 'batch_swaps'
        ? process.env.NEXT_PUBLIC_FLAG_BATCH_SWAPS
        : flag === 'swap_ui_v2'
          ? process.env.NEXT_PUBLIC_FLAG_SWAP_UI_V2
          : flag === 'transaction_history'
            ? process.env.NEXT_PUBLIC_FLAG_TRANSACTION_HISTORY
            : flag === 'real_xdr'
              ? process.env.NEXT_PUBLIC_FLAG_REAL_XDR
              : flag === 'analytics'
                ? process.env.NEXT_PUBLIC_FEATURE_ANALYTICS
                : process.env.NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE;
  if (val === undefined) return undefined;
  return val === 'true' || val === '1';
}

function readWindowFlag(flag: FlagName): boolean | undefined {
  if (typeof window === 'undefined') return undefined;
  const flags = (window as { __STELLAR_ROUTE_FLAGS__?: FlagMap })
    .__STELLAR_ROUTE_FLAGS__;
  if (flags?.[flag] !== undefined) return flags[flag]!;
  return undefined;
}

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
      remoteFlags = {};
      return {};
    });

  return remoteFetchPromise;
}

/**
 * SSR-safe initial snapshot: pinned/env (and warmed remote cache only).
 * Never reads `window.__STELLAR_ROUTE_FLAGS__` so server HTML matches the first
 * client render.
 */
export function resolveFlagForInitialRender(flag: FlagName): boolean {
  if (SECURITY_PINNED_FLAGS.has(flag)) {
    const env = readEnvFlag(flag);
    if (env !== undefined) return env;
    return true;
  }
  if (remoteFlags !== null && remoteFlags[flag] !== undefined) {
    return remoteFlags[flag]!;
  }
  const env = readEnvFlag(flag);
  if (env !== undefined) return env;
  return defaultFlagValue(flag);
}

function initialFlagLoading(flag: FlagName): boolean {
  if (SECURITY_PINNED_FLAGS.has(flag)) return false;
  if (readEnvFlag(flag) !== undefined) return false;
  if (remoteFlags !== null && remoteFlags[flag] !== undefined) return false;
  return Boolean(process.env.NEXT_PUBLIC_FLAGS_URL);
}

/**
 * Full post-hydration resolution for ordinary flags: remote > window > env > default.
 * `real_xdr` is security-pinned: env/default only (default on). Remote and window
 * cannot disable the secure API prepare/sign/submit path.
 */
export function resolveFlag(flag: FlagName, remote: FlagMap = {}): boolean {
  if (SECURITY_PINNED_FLAGS.has(flag)) {
    const env = readEnvFlag(flag);
    if (env !== undefined) return env;
    return true;
  }
  if (remote[flag] !== undefined) return remote[flag]!;
  const windowFlag = readWindowFlag(flag);
  if (windowFlag !== undefined) return windowFlag;
  const env = readEnvFlag(flag);
  if (env !== undefined) return env;
  return defaultFlagValue(flag);
}

export function useFeatureFlag(flag: FlagName): {
  enabled: boolean;
  loading: boolean;
} {
  const [enabled, setEnabled] = useState<boolean>(() =>
    resolveFlagForInitialRender(flag),
  );
  const [loading, setLoading] = useState<boolean>(() => initialFlagLoading(flag));

  useEffect(() => {
    let cancelled = false;
    const flagsUrl = process.env.NEXT_PUBLIC_FLAGS_URL;

    if (SECURITY_PINNED_FLAGS.has(flag)) {
      setEnabled(resolveFlag(flag));
      setLoading(false);
      return;
    }

    // Dev/e2e window overrides apply after mount; remote fetch may supersede.
    if (readWindowFlag(flag) !== undefined) {
      setEnabled(resolveFlag(flag));
    }

    if (!flagsUrl) {
      setLoading(false);
      return;
    }

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

export function useFeatureFlags(flags: FlagName[]): Record<FlagName, boolean> {
  const [resolved, setResolved] = useState<Record<FlagName, boolean>>(
    () =>
      Object.fromEntries(
        flags.map((f) => [f, resolveFlagForInitialRender(f)]),
      ) as Record<FlagName, boolean>,
  );

  useEffect(() => {
    let cancelled = false;
    const flagsUrl = process.env.NEXT_PUBLIC_FLAGS_URL;

    const hasWindowOverride = flags.some((f) => readWindowFlag(f) !== undefined);
    if (hasWindowOverride) {
      setResolved(
        Object.fromEntries(flags.map((f) => [f, resolveFlag(f)])) as Record<
          FlagName,
          boolean
        >,
      );
    }

    if (!flagsUrl) return;

    fetchRemoteFlags().then((remote) => {
      if (!cancelled) {
        setResolved(
          Object.fromEntries(
            flags.map((f) => [f, resolveFlag(f, remote)]),
          ) as Record<FlagName, boolean>,
        );
      }
    });

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flags.join(',')]);

  return resolved;
}
