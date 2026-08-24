'use client';

import { useCallback, useEffect, useState, useSyncExternalStore } from 'react';
import {
  getReadinessSnapshot,
  refreshApiV2Readiness,
  subscribeReadiness,
  type ReadinessSnapshot,
} from '@/lib/cctp/readiness';

const SSR_SNAPSHOT: ReadinessSnapshot = {
  loaded: false,
  corridors: [],
  cctpGloballyReady: false,
  providerKilled: false,
  error: null,
  fetchedAt: null,
};

export function useApiV2Readiness(options?: { refreshMs?: number }) {
  const snapshot = useSyncExternalStore(
    subscribeReadiness,
    getReadinessSnapshot,
    () => SSR_SNAPSHOT,
  );

  const [loading, setLoading] = useState(!snapshot.loaded);

  const refresh = useCallback(async (signal?: AbortSignal) => {
    setLoading(true);
    await refreshApiV2Readiness(signal);
    setLoading(false);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    void refresh(controller.signal);
    return () => controller.abort();
  }, [refresh]);

  useEffect(() => {
    if (!options?.refreshMs) return;
    const id = setInterval(() => {
      void refresh();
    }, options.refreshMs);
    return () => clearInterval(id);
  }, [options?.refreshMs, refresh]);

  return {
    ...snapshot,
    loading,
    refresh,
  };
}
