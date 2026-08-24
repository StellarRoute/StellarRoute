"use client";

import { useRef, KeyboardEvent } from "react";
import { Star, X } from "lucide-react";
import { cn } from "@/lib/utils";
import type { FavoritePair } from "@/hooks/useFavoritePairs";

interface FavoritesRailProps {
  favorites: FavoritePair[];
  selectedBase?: string;
  selectedQuote?: string;
  onSelect: (pair: FavoritePair) => void;
  onRemove: (baseAsset: string, quoteAsset: string) => void;
  className?: string;
}

/**
 * Horizontal scrollable rail of favorite trading pairs.
 * Keyboard: Arrow keys navigate between chips, Enter/Space selects, Delete/Backspace removes.
 */
export function FavoritesRail({
  favorites,
  selectedBase,
  selectedQuote,
  onSelect,
  onRemove,
  className,
}: FavoritesRailProps) {
  const listRef = useRef<HTMLUListElement>(null);

  if (favorites.length === 0) return null;

  const focusItem = (index: number) => {
    const items = listRef.current?.querySelectorAll<HTMLElement>("[data-chip]");
    items?.[index]?.focus();
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLLIElement>, index: number, pair: FavoritePair) => {
    const items = listRef.current?.querySelectorAll<HTMLElement>("[data-chip]");
    const count = items?.length ?? 0;

    switch (e.key) {
      case "ArrowRight":
        e.preventDefault();
        focusItem((index + 1) % count);
        break;
      case "ArrowLeft":
        e.preventDefault();
        focusItem((index - 1 + count) % count);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        onSelect(pair);
        break;
      case "Delete":
      case "Backspace":
        e.preventDefault();
        onRemove(pair.baseAsset, pair.quoteAsset);
        // Focus next or previous after removal
        setTimeout(() => focusItem(Math.min(index, count - 2)), 0);
        break;
    }
  };

  return (
    <nav aria-label="Favorite trading pairs" className={cn("w-full", className)}>
      <div className="flex items-center gap-1.5 mb-1.5">
        <Star className="h-3 w-3 text-amber-500 fill-amber-500" aria-hidden="true" />
        <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          Favorites
        </span>
      </div>
      <ul
        ref={listRef}
        role="list"
        className="flex gap-2 overflow-x-auto pb-1 scrollbar-none"
        aria-label="Favorite pairs list"
      >
        {favorites.map((pair, index) => {
          const isActive =
            pair.baseAsset === selectedBase && pair.quoteAsset === selectedQuote;
          return (
            <li key={`${pair.baseAsset}|${pair.quoteAsset}`} onKeyDown={(e) => handleKeyDown(e, index, pair)}>
              <div
                data-chip
                role="button"
                tabIndex={0}
                aria-pressed={isActive}
                aria-label={`${pair.base}/${pair.quote} favorite pair${isActive ? ", selected" : ""}`}
                onClick={() => onSelect(pair)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onSelect(pair);
                  }
                }}
                className={cn(
                  "flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium cursor-pointer select-none",
                  "transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
                  "whitespace-nowrap",
                  isActive
                    ? "bg-primary/10 border-primary/40 text-primary"
                    : "bg-muted/50 border-border/50 text-muted-foreground hover:bg-muted hover:text-foreground"
                )}
              >
                <span>{pair.base}/{pair.quote}</span>
                <button
                  type="button"
                  aria-label={`Remove ${pair.base}/${pair.quote} from favorites`}
                  tabIndex={-1}
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(pair.baseAsset, pair.quoteAsset);
                  }}
                  className="rounded-full p-0.5 hover:bg-destructive/20 hover:text-destructive transition-colors"
                >
                  <X className="h-2.5 w-2.5" aria-hidden="true" />
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
