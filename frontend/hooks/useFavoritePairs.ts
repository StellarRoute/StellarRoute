"use client";

import { useState, useCallback } from "react";

const FAVORITES_KEY = "stellar-route-favorite-pairs";
const MAX_FAVORITES = 20;

export interface FavoritePair {
  base: string;
  quote: string;
  /** canonical asset ids for lookup */
  baseAsset: string;
  quoteAsset: string;
}

function pairKey(baseAsset: string, quoteAsset: string): string {
  return `${baseAsset}|${quoteAsset}`;
}

function loadFavorites(): FavoritePair[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(FAVORITES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (
      Array.isArray(parsed) &&
      parsed.every(
        (p) =>
          typeof p.base === "string" &&
          typeof p.quote === "string" &&
          typeof p.baseAsset === "string" &&
          typeof p.quoteAsset === "string"
      )
    ) {
      return parsed;
    }
  } catch {
    // ignore corrupt storage
  }
  return [];
}

function saveFavorites(favorites: FavoritePair[]): void {
  localStorage.setItem(FAVORITES_KEY, JSON.stringify(favorites));
}

export function useFavoritePairs() {
  const [favorites, setFavorites] = useState<FavoritePair[]>(loadFavorites);

  const isFavorite = useCallback(
    (baseAsset: string, quoteAsset: string) =>
      favorites.some(
        (f) => pairKey(f.baseAsset, f.quoteAsset) === pairKey(baseAsset, quoteAsset)
      ),
    [favorites]
  );

  const addFavorite = useCallback((pair: FavoritePair) => {
    setFavorites((prev) => {
      if (prev.some((f) => pairKey(f.baseAsset, f.quoteAsset) === pairKey(pair.baseAsset, pair.quoteAsset))) {
        return prev;
      }
      const next = [pair, ...prev].slice(0, MAX_FAVORITES);
      saveFavorites(next);
      return next;
    });
  }, []);

  const removeFavorite = useCallback((baseAsset: string, quoteAsset: string) => {
    setFavorites((prev) => {
      const next = prev.filter(
        (f) => pairKey(f.baseAsset, f.quoteAsset) !== pairKey(baseAsset, quoteAsset)
      );
      saveFavorites(next);
      return next;
    });
  }, []);

  const toggleFavorite = useCallback(
    (pair: FavoritePair) => {
      if (isFavorite(pair.baseAsset, pair.quoteAsset)) {
        removeFavorite(pair.baseAsset, pair.quoteAsset);
      } else {
        addFavorite(pair);
      }
    },
    [isFavorite, addFavorite, removeFavorite]
  );

  return { favorites, isFavorite, addFavorite, removeFavorite, toggleFavorite };
}
