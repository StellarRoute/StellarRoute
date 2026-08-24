"use client";

import { useOrderbook } from "@/hooks/useApi";
import { Skeleton } from "@/components/ui/skeleton";
import { ViewState } from "@/components/shared/ViewState";
import { SpreadIndicator } from "./SpreadIndicator";
import type { OrderbookEntry } from "@/types";

interface OrderbookDepthPanelProps {
  base: string;
  quote: string;
  /** Max rows per side. Defaults to 10. */
  maxRows?: number;
  className?: string;
}

function OrderbookRow({ entry, side }: { entry: OrderbookEntry; side: "bid" | "ask" }) {
  const isBid = side === "bid";
  return (
    <tr className="text-xs tabular-nums">
      <td className={`py-0.5 pr-2 text-right ${isBid ? "text-green-500" : "text-red-500"}`}>
        {entry.price}
      </td>
      <td className="py-0.5 pr-2 text-right text-foreground">{entry.amount}</td>
      <td className="py-0.5 text-right text-muted-foreground">{entry.total}</td>
    </tr>
  );
}

function SkeletonRows({ count }: { count: number }) {
  return (
    <>
      {Array.from({ length: count }).map((_, i) => (
        <tr key={i}>
          <td className="py-0.5 pr-2"><Skeleton className="h-3 w-14 ml-auto" /></td>
          <td className="py-0.5 pr-2"><Skeleton className="h-3 w-14 ml-auto" /></td>
          <td className="py-0.5"><Skeleton className="h-3 w-14 ml-auto" /></td>
        </tr>
      ))}
    </>
  );
}

function SideTable({
  label,
  entries,
  side,
  loading,
  maxRows,
}: {
  label: string;
  entries: OrderbookEntry[];
  side: "bid" | "ask";
  loading: boolean;
  maxRows: number;
}) {
  const rows = entries.slice(0, maxRows);
  return (
    <div className="min-w-0 flex-1">
      <p className={`mb-1 text-xs font-semibold ${side === "bid" ? "text-green-500" : "text-red-500"}`}>
        {label}
      </p>
      <table className="w-full">
        <thead>
          <tr className="text-xs text-muted-foreground">
            <th className="pb-1 pr-2 text-right font-normal">Price</th>
            <th className="pb-1 pr-2 text-right font-normal">Amount</th>
            <th className="pb-1 text-right font-normal">Total</th>
          </tr>
        </thead>
        <tbody>
          {loading ? (
            <SkeletonRows count={maxRows} />
          ) : rows.length === 0 ? (
            <tr>
              <td colSpan={3} className="py-2 text-center text-xs text-muted-foreground">
                No {label.toLowerCase()}
              </td>
            </tr>
          ) : (
            rows.map((entry, i) => <OrderbookRow key={i} entry={entry} side={side} />)
          )}
        </tbody>
      </table>
    </div>
  );
}

export function OrderbookDepthPanel({
  base,
  quote,
  maxRows = 10,
  className,
}: OrderbookDepthPanelProps) {
  const { data, loading, error, midpoint, spreadBps } = useOrderbook(base, quote);

  if (error) {
    return (
      <ViewState
        variant="error"
        title="Orderbook unavailable"
        description={error.message}
        className={className}
      />
    );
  }

  // Sort: bids descending (highest first), asks ascending (lowest first)
  const bids = data ? [...data.bids].sort((a, b) => Number(b.price) - Number(a.price)) : [];
  const asks = data ? [...data.asks].sort((a, b) => Number(a.price) - Number(b.price)) : [];

  return (
    <section
      aria-label={`Orderbook for ${base}/${quote}`}
      className={`rounded-xl border bg-card p-4 ${className ?? ""}`}
    >
      <h2 className="mb-3 text-sm font-semibold">
        Orderbook{" "}
        <span className="text-muted-foreground font-normal">
          {base}/{quote}
        </span>
      </h2>
      <SpreadIndicator
        midpoint={midpoint}
        spreadBps={spreadBps}
        isLoading={loading}
        className="mb-4 pb-2 border-b border-border/20"
      />
      {/* Side-by-side on md+, stacked + horizontally scrollable on mobile */}
      <div className="flex flex-col gap-4 overflow-x-auto sm:flex-row">
        <SideTable label="Bids" entries={bids} side="bid" loading={loading} maxRows={maxRows} />
        <div className="hidden sm:block w-px bg-border" />
        <SideTable label="Asks" entries={asks} side="ask" loading={loading} maxRows={maxRows} />
      </div>
    </section>
  );
}
