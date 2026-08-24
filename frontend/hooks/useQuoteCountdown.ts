'use client';

import { useEffect, useState, useCallback } from 'react';

export interface QuoteCountdown {
  remainingSeconds: number;
  isExpired: boolean;
  progress: number; // 0 to 1
}

/**
 * Hook for a drift-free quote TTL countdown timer.
 * Syncs with the server-provided expiration timestamp.
 *
 * @param expiresAtMs The Unix timestamp (ms) when the quote expires.
 * @param totalTtlMs The total duration of the quote TTL in ms (for progress calculation).
 */
export function useQuoteCountdown(
  expiresAtMs: number | undefined,
  totalTtlMs: number = 5500
): QuoteCountdown {
  const [now, setNow] = useState(() => Date.now());

  const updateNow = useCallback(() => {
    setNow(Date.now());
  }, []);

  useEffect(() => {
    if (!expiresAtMs) return;

    // Use a high-frequency interval to minimize visual "jumpiness" 
    // while the actual value is derived from the absolute system clock
    // to prevent drift accumulation.
    const id = setInterval(updateNow, 100);
    return () => clearInterval(id);
  }, [expiresAtMs, updateNow]);

  if (!expiresAtMs) {
    return { remainingSeconds: 0, isExpired: false, progress: 1 };
  }

  const diffMs = expiresAtMs - now;
  const remainingSeconds = Math.max(0, Math.ceil(diffMs / 1000));
  const isExpired = diffMs <= 0;
  
  // Progress from 1 (new) to 0 (expired)
  const progress = Math.min(1, Math.max(0, diffMs / totalTtlMs));

  return {
    remainingSeconds,
    isExpired,
    progress,
  };
}
