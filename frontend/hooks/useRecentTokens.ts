"use client";

import { useState, useEffect, useCallback } from "react";

const RECENT_TOKENS_KEY = "stellar-route-recent-tokens";
const MAX_RECENT_TOKENS = 10;

export function useRecentTokens() {
  const [recentTokens, setRecentTokens] = useState<string[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    const stored = localStorage.getItem(RECENT_TOKENS_KEY);
    if (stored) {
      try {
        setRecentTokens(JSON.parse(stored));
      } catch (e) {
        console.error("Failed to parse recent tokens", e);
      }
    }
    setIsLoaded(true);
  }, []);

  const addRecentToken = useCallback((asset: string) => {
    setRecentTokens((prev) => {
      const next = [asset, ...prev.filter((t) => t !== asset)].slice(
        0,
        MAX_RECENT_TOKENS
      );
      localStorage.setItem(RECENT_TOKENS_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  return { recentTokens, addRecentToken, isLoaded };
}
