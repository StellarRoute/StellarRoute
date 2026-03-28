"use client";

import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ViewState } from "@/components/shared/ViewState";
import { useOrderbook, usePairs } from "@/hooks/useApi";
import type { TradingPair } from "@/types";

function pairKey(pair: TradingPair): string {
  return `${pair.base_asset}__${pair.counter_asset}`;
}

export default function OrderbookPage() {
  const { data: pairs, loading: pairsLoading, error: pairsError } = usePairs();
  const [selectedPairKey, setSelectedPairKey] = useState<string>("");

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
            <div className="grid gap-4 md:grid-cols-2">
              <Card className="p-4 space-y-3">
                <h2 className="font-semibold">Bids</h2>
                <div className="space-y-2 text-sm">
                  {orderbook.bids.slice(0, 10).map((bid, index) => (
                    <div key={`${bid.price}-${index}`} className="grid grid-cols-3">
                      <span className="text-emerald-600">{bid.price}</span>
                      <span>{bid.amount}</span>
                      <span>{bid.total}</span>
                    </div>
                  ))}
                </div>
              </Card>

              <Card className="p-4 space-y-3">
                <h2 className="font-semibold">Asks</h2>
                <div className="space-y-2 text-sm">
                  {orderbook.asks.slice(0, 10).map((ask, index) => (
                    <div key={`${ask.price}-${index}`} className="grid grid-cols-3">
                      <span className="text-red-500">{ask.price}</span>
                      <span>{ask.amount}</span>
                      <span>{ask.total}</span>
                    </div>
                  ))}
                </div>
              </Card>
            </div>
          )}
        </>
      )}
    </div>
  );
}
