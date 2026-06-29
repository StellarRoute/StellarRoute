'use client';

import { useEffect, useState } from 'react';

export type FlagName =
  | "routes_beta"
  | "batch_swaps"
  | "swap_ui_v2"
  | "transaction_history"
  | "advanced_slippage"
  | "real_xdr";

export type FlagMap = Partial<Record<FlagName, boolean>>;

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
            : process.env.NEXT_PUBLIC_FLAG_ADVANCED_SLIPPAGE;
  if (val === undefined) return undefined;
  return val === 'true' || val === '1';
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

function resolveFlag(flag: FlagName, remote: FlagMap): boolean {
  if (remote[flag] !== undefined) return remote[flag]!;
  const env = readEnvFlag(flag);
  if (env !== undefined) return env;
  return false;
}

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

export function useFeatureFlags(flags: FlagName[]): Record<FlagName, boolean> {
  const [resolved, setResolved] = useState<Record<FlagName, boolean>>(
    () =>
      Object.fromEntries(flags.map((f) => [f, false])) as Record<
        FlagName,
        boolean
      >
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
  }, [flags.join(',')]);

  return resolved;
}
