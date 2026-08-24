"use client";

import { useMemo, useState, useEffect } from "react";
import { cn } from "@/lib/utils";

export type PricePoint = {
  timestamp: number;
  price: number;
};

export type SparklineRange = "1h" | "24h" | "7d";
export type RangeDataMap = Partial<Record<SparklineRange, PricePoint[]>>;

export type PriceSparklineProps = {
  rangeData?: RangeDataMap;
  loadingRanges?: Set<SparklineRange>;
  onRangeChange?: (range: SparklineRange) => void;
  pairKey?: string;
};

const RANGES: SparklineRange[] = ["1h", "24h", "7d"];

export default function PriceSparkline({
  rangeData = {},
  loadingRanges = new Set(),
  onRangeChange,
  pairKey,
}: PriceSparklineProps) {
  const [activeRange, setActiveRange] = useState<SparklineRange>("24h");
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  // Restore selection from sessionStorage
  useEffect(() => {
    if (pairKey) {
      const saved = sessionStorage.getItem(`sparkline_range_${pairKey}`);
      if (saved && RANGES.includes(saved as SparklineRange)) {
        setActiveRange(saved as SparklineRange);
      }
    }
  }, [pairKey]);

  const handleRangeChange = (range: SparklineRange) => {
    setActiveRange(range);
    if (pairKey) {
      sessionStorage.setItem(`sparkline_range_${pairKey}`, range);
    }
    onRangeChange?.(range);
  };

  const currentData = rangeData[activeRange] ?? [];
  const isLoading = loadingRanges.has(activeRange);

  // Limit points for performance
  const sliced = useMemo(() => currentData.slice(-50), [currentData]);

  const { points, normalized } = useMemo(() => {
    if (sliced.length === 0) {
      return { points: "", normalized: [] as Array<PricePoint & { x: number; y: number }> };
    }

    const max = Math.max(...sliced.map((d) => d.price));
    const min = Math.min(...sliced.map((d) => d.price));

    const normalized = sliced.map((d, i) => {
      const x = (i / Math.max(sliced.length - 1, 1)) * 100;
      const y = max === min ? 50 : 100 - ((d.price - min) / (max - min)) * 100;

      return { x, y, ...d };
    });

    const points = normalized.map((p) => `${p.x},${p.y}`).join(" ");

    return { points, normalized };
  }, [sliced]);

  return (
    <div className="w-full flex flex-col gap-2">
      {/* Range Selector */}
      <div className="flex items-center gap-1">
        {RANGES.map((r) => {
          const isL = loadingRanges.has(r);
          return (
            <button
              key={r}
              type="button"
              onClick={() => handleRangeChange(r)}
              aria-pressed={activeRange === r}
              aria-label={`${r}${isL ? " loading" : ""}`}
              className={cn(
                "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                activeRange === r
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:bg-muted/80"
              )}
            >
              {isL && (
                <span
                  className="inline-block h-2 w-2 rounded-full border border-current border-t-transparent animate-spin"
                  aria-hidden="true"
                />
              )}
              {r}
            </button>
          );
        })}
      </div>

      {/* Chart Area */}
      {isLoading ? (
        <div role="status" aria-live="polite" className="h-48 flex items-center justify-center">
          <span className="inline-block h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin mr-2" />
          <span className="text-xs text-muted-foreground">Loading price data...</span>
        </div>
      ) : !rangeData[activeRange] || rangeData[activeRange]!.length === 0 ? (
        <div className="flex items-center justify-center h-48">
          <p className="text-xs text-muted-foreground">
            No price data available for {activeRange}
          </p>
        </div>
      ) : (
        <div className="w-full h-48 relative">
          <svg
            viewBox="0 0 100 100"
            className="w-full h-full"
            preserveAspectRatio="none"
            onMouseMove={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              const x = e.clientX - rect.left;
              const percent = x / rect.width;

              const index = Math.min(
                normalized.length - 1,
                Math.max(0, Math.round(percent * (normalized.length - 1)))
              );

              setHoveredIndex(index);
            }}
            onMouseLeave={() => setHoveredIndex(null)}
          >
            <polyline fill="none" stroke="currentColor" strokeWidth="2" points={points} />

            {/* Optional hover dot */}
            {hoveredIndex !== null && normalized[hoveredIndex] && (
              <circle
                cx={normalized[hoveredIndex].x}
                cy={normalized[hoveredIndex].y}
                r="2"
                fill="currentColor"
              />
            )}
          </svg>

          {/* Tooltip */}
          {hoveredIndex !== null && normalized[hoveredIndex] && (
            <div className="absolute top-0 left-0 bg-popover text-popover-foreground text-xs p-1 rounded shadow tabular-nums">
              {new Date(normalized[hoveredIndex].timestamp).toLocaleTimeString()} —{" "}
              {normalized[hoveredIndex].price.toFixed(2)}
            </div>
          )}
        </div>
      )}
    </div>
  );
}