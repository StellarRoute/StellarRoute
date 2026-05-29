"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ViewState } from "@/components/shared/ViewState";
import { useOrderbook, usePairs } from "@/hooks/useApi";
import { useOptionalTradingPair } from "@/contexts/TradingPairContext";
import type { TradingPair } from "@/types";
import { cn } from "@/lib/utils";

function pairKey(pair: TradingPair): string {
  return `${pair.base_asset}__${pair.counter_asset}`;
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

  // Check if current orderbook pair matches the trading pair from swap context
  const isHighlightedPair = useMemo(() => {
    if (!tradingPairContext?.fromAsset || !tradingPairContext?.toAsset || !selectedPair) {
      return false;
    }
    
    // Check both directions (from->to and to->from)
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
                <Card className={cn(
                  "p-4 space-y-3 transition-all duration-300",
                  isHighlightedPair && "ring-2 ring-primary/30 shadow-lg shadow-primary/10"
                )}>
                  <h2 className="font-semibold">Bids</h2>
                  <div className="space-y-1 text-sm">
                    <div className="grid grid-cols-3 text-xs text-muted-foreground font-medium pb-2 border-b">
                      <span>Price</span>
                      <span>Amount</span>
                      <span>Total</span>
                    </div>
                    {orderbook.bids.slice(0, 10).map((bid, index) => (
                      <div 
                        key={`${bid.price}-${index}`} 
                        className={cn(
                          "grid grid-cols-3 py-1.5 px-2 rounded transition-all duration-200",
                          "hover:bg-emerald-500/10 hover:scale-[1.02] cursor-pointer",
                          isHighlightedPair && "bg-emerald-500/5"
                        )}
                        data-testid={isHighlightedPair ? "highlighted-bid-row" : "bid-row"}
                      >
                        <span className="text-emerald-600 font-medium">{bid.price}</span>
                        <span className="text-muted-foreground">{bid.amount}</span>
                        <span className="text-muted-foreground">{bid.total}</span>
                      </div>
                    ))}
                  </div>
                </Card>

                <Card className={cn(
                  "p-4 space-y-3 transition-all duration-300",
                  isHighlightedPair && "ring-2 ring-primary/30 shadow-lg shadow-primary/10"
                )}>
                  <h2 className="font-semibold">Asks</h2>
                  <div className="space-y-1 text-sm">
                    <div className="grid grid-cols-3 text-xs text-muted-foreground font-medium pb-2 border-b">
                      <span>Price</span>
                      <span>Amount</span>
                      <span>Total</span>
                    </div>
                    {orderbook.asks.slice(0, 10).map((ask, index) => (
                      <div 
                        key={`${ask.price}-${index}`} 
                        className={cn(
                          "grid grid-cols-3 py-1.5 px-2 rounded transition-all duration-200",
                          "hover:bg-red-500/10 hover:scale-[1.02] cursor-pointer",
                          isHighlightedPair && "bg-red-500/5"
                        )}
                        data-testid={isHighlightedPair ? "highlighted-ask-row" : "ask-row"}
                      >
                        <span className="text-red-500 font-medium">{ask.price}</span>
                        <span className="text-muted-foreground">{ask.amount}</span>
                        <span className="text-muted-foreground">{ask.total}</span>
                      </div>
                    ))}
                  </div>
                </Card>
              </div>
            </>
          )}
        </>
      )}
    </div>
  );
}
