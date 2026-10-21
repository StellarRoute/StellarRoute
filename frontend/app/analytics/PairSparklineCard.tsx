"use client";

import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { PriceHistorySparkline } from "@/components/shared/PriceHistorySparkline";
import { ViewState } from "@/components/shared/ViewState";
import { usePairs, usePriceHistory } from "@/hooks/useApi";
import type { TradingPair } from "@/types";

function pairKey(pair: TradingPair): string {
  return `${pair.base_asset}__${pair.counter_asset}`;
}

/**
 * 24h price sparkline for a single trading pair, for /analytics only.
 *
 * Deliberately scoped to this page: the swap surface must not mount a chart,
 * so this reuses the shared `PriceHistorySparkline` here rather than inside
 * `SwapCard`. Data comes from the existing `GET /api/v1/price-history`
 * endpoint via `usePriceHistory` — no new API surface.
 */
export function PairSparklineCard() {
  const { data: pairs, loading: pairsLoading, error: pairsError } = usePairs();
  const [selectedKey, setSelectedKey] = useState<string>("");

  // Default to the first pair once they arrive, and recover if the selected
  // pair disappears from a later refresh.
  useEffect(() => {
    if (!pairs?.length) return;
    setSelectedKey((current) =>
      current && pairs.some((pair) => pairKey(pair) === current)
        ? current
        : pairKey(pairs[0]),
    );
  }, [pairs]);

  const selectedPair = useMemo(
    () => pairs?.find((pair) => pairKey(pair) === selectedKey),
    [pairs, selectedKey],
  );

  const { data: history, loading: historyLoading } = usePriceHistory(
    selectedPair?.base_asset ?? "",
    selectedPair?.counter_asset ?? "",
    60_000,
    !selectedPair,
  );

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-base">24h price trend</CardTitle>
        <CardDescription>
          Mid-market price over the last 24 hours for a single pair.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {pairsLoading ? (
          <ViewState
            variant="loading"
            title="Loading markets"
            description="Fetching available trading pairs..."
          />
        ) : pairsError ? (
          <ViewState
            variant="error"
            title="Could not load markets"
            description={pairsError.message}
          />
        ) : !pairs?.length ? (
          <ViewState
            variant="empty"
            title="No markets available"
            description="The indexer is syncing trading pairs. Check back in a few moments."
          />
        ) : (
          <>
            <div
              className="flex flex-wrap gap-2"
              role="group"
              aria-label="Select a trading pair"
            >
              {pairs.map((pair) => {
                const key = pairKey(pair);
                const isActive = key === selectedKey;
                return (
                  <Button
                    key={key}
                    type="button"
                    size="sm"
                    variant={isActive ? "default" : "outline"}
                    aria-pressed={isActive}
                    onClick={() => setSelectedKey(key)}
                  >
                    {pair.base}/{pair.counter}
                  </Button>
                );
              })}
            </div>

            <PriceHistorySparkline
              points={history?.points}
              loading={historyLoading}
              title={
                selectedPair
                  ? `${selectedPair.base}/${selectedPair.counter} · 24h`
                  : "24h price trend"
              }
            />
          </>
        )}
      </CardContent>
    </Card>
  );
}

export default PairSparklineCard;
