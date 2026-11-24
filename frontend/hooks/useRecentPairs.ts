"use client";

import { useState, useCallback } from "react";

const RECENT_PAIRS_KEY = "stellar-route-recent-pairs";
const MAX_RECENT = 5;

export interface RecentPair {
  base: string;
  quote: string;
  /** canonical asset ids */
  baseAsset: string;
  quoteAsset: string;
}

function pairKey(baseAsset: string, quoteAsset: string): string {
  return `${baseAsset}|${quoteAsset}`;
}

function loadRecent(): RecentPair[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(RECENT_PAIRS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (
      Array.isArray(parsed) &&
      parsed.every(
        (p) =>
          typeof p.base === "string" &&
          typeof p.quote === "string" &&
          typeof p.baseAsset === "string" &&
          typeof p.quoteAsset === "string",
      )
    ) {
      return parsed as RecentPair[];
    }
  } catch {
    // ignore corrupt storage
  }
  return [];
}

function saveRecent(pairs: RecentPair[]): void {
  try {
    localStorage.setItem(RECENT_PAIRS_KEY, JSON.stringify(pairs));
  } catch {
    // ignore storage errors (e.g. quota exceeded)
  }
}

/**
 * LRU cache of recently-selected trading pairs backed by localStorage.
 *
 * Adding a pair that is already in the list moves it to position 0 (LRU
 * promotion). The list is capped at MAX_RECENT (5) entries; the oldest entry
 * falls off when the cap is exceeded.
 */
export function useRecentPairs() {
  const [recentPairs, setRecentPairs] = useState<RecentPair[]>(loadRecent);

  const addRecentPair = useCallback((pair: RecentPair) => {
    setRecentPairs((prev) => {
      // Remove any existing entry for this pair (LRU: promote to front)
      const filtered = prev.filter(
        (p) => pairKey(p.baseAsset, p.quoteAsset) !== pairKey(pair.baseAsset, pair.quoteAsset),
      );
      const next = [pair, ...filtered].slice(0, MAX_RECENT);
      saveRecent(next);
      return next;
    });
  }, []);

  const clearRecentPairs = useCallback(() => {
    setRecentPairs([]);
    try {
      localStorage.removeItem(RECENT_PAIRS_KEY);
    } catch {
      // ignore
    }
  }, []);

  return { recentPairs, addRecentPair, clearRecentPairs };
}
