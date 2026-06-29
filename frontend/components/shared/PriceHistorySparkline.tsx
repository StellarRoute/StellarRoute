"use client";

import { useId, useMemo, useRef, useState, type PointerEvent } from "react";

import { cn } from "@/lib/utils";
import type { PriceHistoryPoint } from "@/types";

type SparklinePoint = PriceHistoryPoint & {
  x: number;
  y: number;
  numericPrice: number;
};

interface PriceHistorySparklineProps {
  points?: PriceHistoryPoint[];
  loading?: boolean;
  className?: string;
  title?: string;
  emptyLabel?: string;
}

function formatPrice(value: number): string {
  if (!Number.isFinite(value)) {
    return "-";
  }

  if (value >= 1_000) {
    return value.toFixed(0);
  }

  if (value >= 1) {
    return value.toFixed(4);
  }

  if (value >= 0.1) {
    return value.toFixed(5);
  }

  return value.toPrecision(6);
}

function formatTime(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(timestamp));
}

export function PriceHistorySparkline({
  points = [],
  loading = false,
  className,
  title = "24h price trend",
  emptyLabel = "No 24h price data available yet.",
}: PriceHistorySparklineProps) {
  const chartRef = useRef<HTMLDivElement | null>(null);
  const gradientId = useId();
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const chartPoints = useMemo<SparklinePoint[]>(() => {
    const recent = points.slice(-24);
    if (!recent.length) {
      return [];
    }

    const numericPrices = recent.map((point) => Number(point.price));
    const max = Math.max(...numericPrices);
    const min = Math.min(...numericPrices);
    const range = max - min;

    return recent.map((point, index) => {
      const x = (index / Math.max(recent.length - 1, 1)) * 100;
      const price = Number(point.price);
      const y = range === 0 ? 50 : 100 - ((price - min) / range) * 100;

      return {
        ...point,
        x,
        y,
        numericPrice: price,
      };
    });
  }, [points]);

  const activePoint = hoveredIndex !== null ? chartPoints[hoveredIndex] : null;
  const lastPrice = chartPoints[chartPoints.length - 1]?.numericPrice;

  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!chartPoints.length || !chartRef.current) {
      return;
    }

    const rect = chartRef.current.getBoundingClientRect();
    const x = (event.clientX - rect.left) / rect.width;
    const index = Math.min(
      chartPoints.length - 1,
      Math.max(0, Math.round(x * (chartPoints.length - 1)))
    );
    setHoveredIndex(index);
  };

  if (loading) {
    return (
      <div className={cn("space-y-2", className)}>
        <div className="h-4 w-32 animate-pulse rounded-full bg-muted" />
        <div className="h-20 animate-pulse rounded-2xl bg-muted/60" />
      </div>
    );
  }

  if (!chartPoints.length) {
    return (
      <div
        className={cn(
          "rounded-2xl border border-dashed border-border/50 bg-muted/20 px-4 py-5",
          className
        )}
        role="status"
      >
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              {title}
            </p>
            <p className="mt-1 text-xs text-muted-foreground">{emptyLabel}</p>
          </div>
          <div className="rounded-full border border-border/40 bg-background px-3 py-1 text-xs font-medium text-muted-foreground">
            24h
          </div>
        </div>
      </div>
    );
  }

  const linePath = chartPoints
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ");
  const firstPoint = chartPoints[0];
  const lastPoint = chartPoints[chartPoints.length - 1];
  const areaPath = `${linePath} L ${lastPoint.x} 100 L ${firstPoint.x} 100 Z`;
  const linePoints = chartPoints
    .map((point) => `${point.x},${point.y}`)
    .join(" ");

  return (
    <div className={cn("space-y-2", className)}>
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {title}
          </p>
          <p className="text-[11px] text-muted-foreground">
            {chartPoints.length} sample{chartPoints.length === 1 ? "" : "s"} in the last 24h
          </p>
        </div>
        <div className="text-right">
          <p className="text-xs text-muted-foreground">Approx. price</p>
          <p className="font-mono text-sm font-semibold tabular-nums text-foreground">
            {lastPrice !== undefined ? formatPrice(lastPrice) : "-"}
          </p>
        </div>
      </div>

      <div
        className="relative h-24 overflow-hidden rounded-2xl border border-border/40 bg-gradient-to-b from-background to-muted/15 text-primary"
        onPointerMove={handlePointerMove}
        onPointerLeave={() => setHoveredIndex(null)}
      >
        <div ref={chartRef} className="absolute inset-2">
          <svg
            viewBox="0 0 100 100"
            className="absolute inset-0 h-full w-full"
            aria-label="24 hour price sparkline"
            role="img"
          >
            <defs>
              <linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stopColor="currentColor" stopOpacity="0.24" />
                <stop offset="100%" stopColor="currentColor" stopOpacity="0" />
              </linearGradient>
            </defs>
            <path d={areaPath} fill={`url(#${gradientId})`} opacity="0.75" />
            <polyline
              fill="none"
              points={linePoints}
              stroke="currentColor"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2.25"
            />
          </svg>

          {chartPoints.map((point, index) => {
            const isActive = hoveredIndex === index;
            const label = `${formatTime(point.timestamp)} - approx ${formatPrice(point.numericPrice)}`;

            return (
              <button
                key={`${point.timestamp}-${index}`}
                type="button"
                aria-label={label}
                className={cn(
                  "absolute z-10 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full outline-none transition",
                  isActive
                    ? "scale-125 bg-primary/30 ring-4 ring-primary/20"
                    : "bg-primary/20 hover:scale-125 hover:bg-primary/30 focus-visible:scale-125 focus-visible:bg-primary/30 focus-visible:ring-4 focus-visible:ring-primary/20"
                )}
                style={{
                  left: `${point.x}%`,
                  top: `${point.y}%`,
                }}
                onFocus={() => setHoveredIndex(index)}
                onBlur={() => setHoveredIndex(null)}
                onPointerEnter={() => setHoveredIndex(index)}
              />
            );
          })}

          {activePoint ? (
            <div
              className="pointer-events-none absolute z-20 min-w-[10rem] -translate-x-1/2 -translate-y-full rounded-xl border border-border/60 bg-background/95 px-3 py-2 text-xs shadow-lg backdrop-blur"
              style={{
                left: `${activePoint.x}%`,
                top: `${activePoint.y}%`,
              }}
            >
              <p className="font-medium text-foreground">
                {formatTime(activePoint.timestamp)}
              </p>
              <p className="mt-0.5 font-mono tabular-nums text-muted-foreground">
                approx {formatPrice(activePoint.numericPrice)}
              </p>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export default PriceHistorySparkline;
