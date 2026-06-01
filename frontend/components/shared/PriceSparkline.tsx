"use client";

/**
 * PriceSparkline — sparkline chart with a time-range selector (issue #518).
 *
 * Time ranges: 1h · 24h · 7d
 *  - Each button shows a loading spinner while data is being fetched.
 *  - The selected range for each trading pair is remembered for the session
 *    via sessionStorage (key: `sparkline_range_<pairKey>`).
 *  - If no data is available for the selected range the component shows a
 *    friendly "No price data" message instead of breaking.
 */

import { useMemo, useState, useEffect, useCallback } from "react";

// ─── Types ────────────────────────────────────────────────────────────────────

export type SparklineRange = "1h" | "24h" | "7d";

export type PricePoint = {
  timestamp: number;
  price: number;
};

export type RangeDataMap = Partial<Record<SparklineRange, PricePoint[]>>;

interface Props {
  /**
   * Pre-fetched price data keyed by range.
   * Pass `undefined` (or omit a key) to indicate data is still loading for
   * that range. Pass an empty array `[]` to indicate no data available.
   */
  rangeData?: RangeDataMap;
  /**
   * Which ranges are currently being loaded. When a range is in this set the
   * corresponding button shows a loading indicator.
   */
  loadingRanges?: Set<SparklineRange>;
  /**
   * Unique key for the trading pair (e.g. "XLM/USDC").
   * Used to persist / restore the selected range in sessionStorage.
   */
  pairKey?: string;
  /** Callback fired when the user picks a new range. */
  onRangeChange?: (range: SparklineRange) => void;
}

const RANGES: SparklineRange[] = ["1h", "24h", "7d"];
const SESSION_KEY = (pair: string) => `sparkline_range_${pair}`;

// ─── Component ────────────────────────────────────────────────────────────────

export default function PriceSparkline({
  rangeData = {},
  loadingRanges = new Set(),
  pairKey = "default",
  onRangeChange,
}: Props) {
  // Restore the user's last-chosen range from sessionStorage (falls back to "24h")
  const [range, setRange] = useState<SparklineRange>(() => {
    if (typeof window === "undefined") return "24h";
    try {
      const stored = sessionStorage.getItem(SESSION_KEY(pairKey));
      if (stored && RANGES.includes(stored as SparklineRange)) {
        return stored as SparklineRange;
      }
    } catch {
      // sessionStorage unavailable — proceed with default
    }
    return "24h";
  });

  // Persist selection whenever pairKey or range changes
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      sessionStorage.setItem(SESSION_KEY(pairKey), range);
    } catch {
      // ignore
    }
  }, [pairKey, range]);

  // When the pairKey changes, restore the saved preference for the new pair
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      const stored = sessionStorage.getItem(SESSION_KEY(pairKey));
      if (stored && RANGES.includes(stored as SparklineRange)) {
        setRange(stored as SparklineRange);
      } else {
        setRange("24h");
      }
    } catch {
      setRange("24h");
    }
  }, [pairKey]);

  const handleRangeSelect = useCallback(
    (r: SparklineRange) => {
      setRange(r);
      onRangeChange?.(r);
    },
    [onRangeChange]
  );

  const data = rangeData[range];
  const isLoading = loadingRanges.has(range);

  return (
    <div className="w-full space-y-2">
      {/* ── Range selector buttons ── */}
      <div
        className="flex gap-1"
        role="group"
        aria-label="Sparkline time range"
      >
        {RANGES.map((r) => (
          <RangeButton
            key={r}
            label={r}
            active={range === r}
            loading={loadingRanges.has(r)}
            onClick={() => handleRangeSelect(r)}
          />
        ))}
      </div>

      {/* ── Chart area ── */}
      {isLoading ? (
        <LoadingState />
      ) : !data || data.length === 0 ? (
        <EmptyState range={range} />
      ) : (
        <SparklineChart data={data} />
      )}
    </div>
  );
}

// ─── Sub-components ───────────────────────────────────────────────────────────

interface RangeButtonProps {
  label: SparklineRange;
  active: boolean;
  loading: boolean;
  onClick: () => void;
}

function RangeButton({ label, active, loading, onClick }: RangeButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      aria-label={`${label} range${loading ? ", loading" : ""}`}
      className={[
        "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        active
          ? "bg-primary text-primary-foreground"
          : "bg-muted text-muted-foreground hover:bg-muted/80",
      ].join(" ")}
    >
      {loading && (
        <span
          className="inline-block h-2.5 w-2.5 rounded-full border border-current border-t-transparent animate-spin"
          aria-hidden="true"
        />
      )}
      {label}
    </button>
  );
}

function LoadingState() {
  return (
    <div
      role="status"
      aria-live="polite"
      className="h-16 flex items-center justify-center"
    >
      <span className="inline-block h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin mr-2" />
      <span className="text-xs text-muted-foreground">Loading price data…</span>
    </div>
  );
}

function EmptyState({ range }: { range: SparklineRange }) {
  return (
    <div
      role="status"
      className="h-16 flex items-center"
    >
      <p className="text-xs text-muted-foreground">
        No price data available for {range}
      </p>
    </div>
  );
}

// ─── Chart ────────────────────────────────────────────────────────────────────

interface SparklineChartProps {
  data: PricePoint[];
}

function SparklineChart({ data }: SparklineChartProps) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  // Keep at most 50 points for performance
  const sliced = useMemo(() => data.slice(-50), [data]);

  const { points, normalized } = useMemo(() => {
    const max = Math.max(...sliced.map((d) => d.price));
    const min = Math.min(...sliced.map((d) => d.price));

    const normalized = sliced.map((d, i) => {
      const x = (i / Math.max(sliced.length - 1, 1)) * 100;
      const y =
        max === min ? 50 : 100 - ((d.price - min) / (max - min)) * 100;
      return { x, y, ...d };
    });

    const points = normalized.map((p) => `${p.x},${p.y}`).join(" ");
    return { points, normalized };
  }, [sliced]);

  return (
    <div className="w-full">
      <div className="w-full h-16 relative">
        <svg
          viewBox="0 0 100 100"
          className="w-full h-full"
          aria-label="Price sparkline chart"
          role="img"
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
          <polyline
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            points={points}
          />
          {hoveredIndex !== null && normalized[hoveredIndex] && (
            <circle
              cx={normalized[hoveredIndex].x}
              cy={normalized[hoveredIndex].y}
              r="2"
              fill="currentColor"
            />
          )}
        </svg>
      </div>

      {/* Hover tooltip */}
      {hoveredIndex !== null && normalized[hoveredIndex] && (
        <div className="text-xs mt-1 text-muted-foreground tabular-nums">
          {new Date(normalized[hoveredIndex].timestamp).toLocaleTimeString()} —{" "}
          {normalized[hoveredIndex].price.toFixed(2)}
        </div>
      )}
    </div>
  );
}
