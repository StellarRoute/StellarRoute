"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ViewState } from "@/components/shared/ViewState";
import { useOrderbook, usePairs } from "@/hooks/useApi";
import { useOptionalTradingPair } from "@/contexts/TradingPairContext";
import { useVirtualWindow } from "@/hooks/useVirtualWindow";
import type { OrderbookEntry, TradingPair } from "@/types";
import { cn } from "@/lib/utils";

const ROW_HEIGHT = 36;
const OVERSCAN = 5;
const MAX_VISIBLE_ROWS = 100;

function pairKey(pair: TradingPair): string {
  return `${pair.base_asset}__${pair.counter_asset}`;
}

function VirtualizedOrderSide({
  entries,
  side,
  highlighted,
}: {
  entries: OrderbookEntry[];
  side: "bid" | "ask";
  highlighted: boolean;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isBid = side === "bid";

  const virtualWindow = useVirtualWindow({
    containerRef: scrollRef,
    itemCount: entries.length,
    itemHeight: ROW_HEIGHT,
    overscan: OVERSCAN,
    defaultViewportHeight: ROW_HEIGHT * 15,
  });

  if (entries.length === 0) {
    return (
      <p className="text-xs text-muted-foreground py-4 text-center">
        No {side}s available
      </p>
    );
  }

  const visibleEntries = virtualWindow.isVirtualized
    ? entries.slice(virtualWindow.startIndex, virtualWindow.endIndex)
    : entries;

  return (
    <div className="space-y-1 text-sm">
      <div className="sticky top-0 z-10 bg-card grid grid-cols-3 text-xs text-muted-foreground font-medium pb-2 border-b">
        <span>Price</span>
        <span>Amount</span>
        <span>Total</span>
      </div>
      <div
        ref={scrollRef}
        className="overflow-auto"
        style={{ height: `${Math.min(entries.length, MAX_VISIBLE_ROWS) * ROW_HEIGHT}px` }}
        data-testid={`${side}-virtual-list`}
      >
        <div
          style={{
            height: `${virtualWindow.totalHeight}px`,
            position: "relative",
          }}
        >
          {virtualWindow.topSpacerHeight > 0 && (
            <div style={{ height: `${virtualWindow.topSpacerHeight}px` }} />
          )}
          {visibleEntries.map((entry, index) => {
            const absoluteIndex = virtualWindow.isVirtualized
              ? virtualWindow.startIndex + index
              : index;
            return (
              <div
                key={`${entry.price}-${absoluteIndex}`}
                data-testid={
                  highlighted
                    ? `highlighted-${side}-row`
                    : `${side}-row`
                }
                className={cn(
                  "grid grid-cols-3 py-1.5 px-2 rounded",
                  isBid
                    ? "hover:bg-emerald-500/10 cursor-pointer"
                    : "hover:bg-red-500/10 cursor-pointer",
                  highlighted && (isBid ? "bg-emerald-500/5" : "bg-red-500/5")
                )}
                style={{ height: `${ROW_HEIGHT}px` }}
              >
                <span
                  className={cn(
                    "font-medium",
                    isBid ? "text-emerald-600" : "text-red-500"
                  )}
                >
                  {entry.price}
                </span>
                <span className="text-muted-foreground truncate">
                  {entry.amount}
                </span>
                <span className="text-muted-foreground truncate">
                  {entry.total}
                </span>
              </div>
            );
          })}
          {virtualWindow.bottomSpacerHeight > 0 && (
            <div style={{ height: `${virtualWindow.bottomSpacerHeight}px` }} />
          )}
        </div>
      </div>
    </div>
  );
}

export default function OrderbookPage() {
  const { data: pairs, loading: pairsLoading, error: pairsError } = usePairs();
  const [selectedPairKey, setSelectedPairKey] = useState<string>("");
  const tradingPairContext = useOptionalTradingPair();

  useEffect(() => {
    if (!pairs?.length) return;
    setSelectedPairKey((current) => {
      if (current && pairs.some((pair) => pairKey(pair) === current)) {
        return current;
      }
      return pairKey(pairs[0]);
    });
  }, [pairs]);

  const selectedPair = useMemo(
    () => pairs?.find((pair) => pairKey(pair) === selectedPairKey),
    [pairs, selectedPairKey],
  );

  const isHighlightedPair = useMemo(() => {
    if (!tradingPairContext?.fromAsset || !tradingPairContext?.toAsset || !selectedPair) {
      return false;
    }
    const matchesForward =
      selectedPair.base_asset === tradingPairContext.fromAsset &&
      selectedPair.counter_asset === tradingPairContext.toAsset;
    const matchesReverse =
      selectedPair.base_asset === tradingPairContext.toAsset &&
      selectedPair.counter_asset === tradingPairContext.fromAsset;
    return matchesForward || matchesReverse;
  }, [tradingPairContext, selectedPair]);

  const {
    data: orderbook,
    loading: orderbookLoading,
    error: orderbookError,
    refresh,
  } = useOrderbook(
    selectedPair?.base_asset ?? "",
    selectedPair?.counter_asset ?? "",
    10_000,
  );

  return (
    <div className="w-full px-4 py-8 sm:px-6 lg:px-8 space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-3xl font-bold">Orderbook</h1>
          <p className="text-muted-foreground">
            Live bids and asks from the selected trading pair.
          </p>
        </div>
        <Button type="button" variant="outline" onClick={refresh}>
          Refresh
        </Button>
      </div>

      {pairsLoading ? (
        <ViewState
          variant="loading"
          title="Loading markets"
          description="Fetching available trading pairs."
        />
      ) : pairsError ? (
        <ViewState
          variant="error"
          title="Could not load markets"
          description="The API is unavailable right now. Please try again."
          action={
            <Button type="button" variant="outline" onClick={refresh}>
              Retry
            </Button>
          }
        />
      ) : !pairs?.length ? (
        <ViewState
          variant="empty"
          title="No markets yet"
          description="No trading pairs are available from the indexer."
        />
      ) : (
        <>
          <div className="flex flex-wrap gap-2">
            {pairs.map((pair) => {
              const key = pairKey(pair);
              const isActive = key === selectedPairKey;
              return (
                <Button
                  key={key}
                  type="button"
                  variant={isActive ? "default" : "outline"}
                  onClick={() => setSelectedPairKey(key)}
                >
                  {pair.base}/{pair.counter}
                </Button>
              );
            })}
          </div>

          {orderbookLoading ? (
            <ViewState
              variant="loading"
              title="Loading orderbook"
              description="Fetching bids and asks for the selected pair."
            />
          ) : orderbookError ? (
            <ViewState
              variant="error"
              title="Could not load orderbook"
              description="Try refreshing or selecting a different pair."
              action={
                <Button type="button" variant="outline" onClick={refresh}>
                  Retry
                </Button>
              }
            />
          ) : !orderbook || (!orderbook.bids.length && !orderbook.asks.length) ? (
            <ViewState
              variant="empty"
              title="No orderbook entries"
              description="There are currently no bids or asks for this pair."
            />
          ) : (
            <>
              {isHighlightedPair && (
                <div
                  className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary/10 border border-primary/20"
                  data-testid="highlighted-pair-indicator"
                >
                  <div className="h-2 w-2 rounded-full bg-primary animate-pulse" />
                  <span className="text-sm font-medium text-primary">
                    This pair is currently selected in the swap panel
                  </span>
                </div>
              )}

              <div className="grid gap-4 md:grid-cols-2">
                <Card
                  className={cn(
                    "p-4 space-y-3 transition-all duration-300",
                    isHighlightedPair &&
                      "ring-2 ring-primary/30 shadow-lg shadow-primary/10"
                  )}
                >
                  <h2 className="font-semibold">Bids ({orderbook.bids.length})</h2>
                  <VirtualizedOrderSide
                    entries={orderbook.bids}
                    side="bid"
                    highlighted={isHighlightedPair}
                  />
                </Card>

                <Card
                  className={cn(
                    "p-4 space-y-3 transition-all duration-300",
                    isHighlightedPair &&
                      "ring-2 ring-primary/30 shadow-lg shadow-primary/10"
                  )}
                >
                  <h2 className="font-semibold">
                    Asks ({orderbook.asks.length})
                  </h2>
                  <VirtualizedOrderSide
                    entries={orderbook.asks}
                    side="ask"
                    highlighted={isHighlightedPair}
                  />
                </Card>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
