"use client";

import React, { useEffect, useMemo, useState } from "react";
import { TokenPairSelector } from "./TokenPairSelector";
import { usePairs } from "@/hooks/useApi";
import { useTokenPairUrl } from "@/hooks/useTokenPairUrl";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { maxDecimalsForSellAsset } from "@/lib/amount-input";
import { SwapValidationSchema } from "@/lib/swap-validation";

/**
 * Example component showing TokenPairSelector integrated with a swap form.
 * This demonstrates the complete flow from pair selection to swap execution.
 */
export function SwapWithPairSelector() {
  const { data: pairsData, loading: pairsLoading, error: pairsError } = usePairs();
  const { base, quote, setPair, isInitializing } = useTokenPairUrl();
  const [amount, setAmount] = useState("");

  // `usePairs()` returns `TradingPair[]` directly (not `{ pairs: ... }`)
  const pairs = useMemo(() => pairsData ?? [], [pairsData]);

  // Auto-select first pair if none selected
  useEffect(() => {
    if (!isInitializing && !base && !quote && pairs.length > 0) {
      setPair(pairs[0].base_asset, pairs[0].counter_asset);
    }
  }, [isInitializing, base, quote, pairs, setPair]);

  if (pairsLoading || isInitializing) {
    return (
      <div className="space-y-4 max-w-lg mx-auto">
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
    );
  }

  const selectedPair = pairs.find(
    (p) => p.base_asset === base && p.counter_asset === quote
  );
  const maxDecimals = selectedPair
    ? maxDecimalsForSellAsset(selectedPair.base_asset, selectedPair.base_decimals)
    : undefined;
  const inputValidation = selectedPair
    ? SwapValidationSchema.validate(
      {
        amount,
        maxDecimals,
        sellAssetId: selectedPair.base_asset,
        buyAssetId: selectedPair.counter_asset,
        slippage: 0.5,
      },
      { mode: "input" },
    )
    : null;
  const submitValidation = selectedPair
    ? SwapValidationSchema.validate(
      {
        amount,
        maxDecimals,
        sellAssetId: selectedPair.base_asset,
        buyAssetId: selectedPair.counter_asset,
        slippage: 0.5,
      },
      { mode: "submit" },
    )
    : null;
  const amountError =
    amount.trim() !== "" &&
    inputValidation &&
    inputValidation.amountResult.status !== "ok" &&
    inputValidation.amountResult.status !== "empty"
      ? inputValidation.amountResult.message
      : null;

  return (
    <div className="space-y-4 max-w-lg mx-auto">
      <TokenPairSelector
        pairs={pairs}
        selectedBase={base}
        selectedQuote={quote}
        onPairChange={setPair}
        loading={pairsLoading}
        error={
          pairsError
            ? "Failed to load trading pairs. Please check your API connection."
            : undefined
        }
      />

      {selectedPair && (
        <Card className="p-6">
          <h3 className="text-lg font-semibold mb-4">Swap Amount</h3>
          <div className="space-y-4">
            <div>
              <label htmlFor="pay-amount-input" className="text-sm font-medium mb-2 block">
                You sell ({selectedPair.base})
              </label>
              <Input
                id="pay-amount-input"
                type="text"
                inputMode="decimal"
                placeholder="0.0"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="text-lg"
                aria-invalid={!!amountError}
              />
              {amountError && (
                <p className="mt-2 text-xs text-destructive">{amountError}</p>
              )}
            </div>

            <div className="rounded-lg bg-muted/50 p-4">
              <p className="text-sm text-muted-foreground mb-1">
                You receive (estimated)
              </p>
              <p className="text-2xl font-bold">
                {amount && !isNaN(Number(amount))
                  ? (Number(amount) * 0.95).toFixed(4)
                  : "0.0"}{" "}
                {selectedPair.counter}
              </p>
            </div>

            <Button
              className="w-full"
              size="lg"
              disabled={!submitValidation?.isValid}
            >
              Review Swap
            </Button>
          </div>
        </Card>
      )}

      {pairs.length === 0 && !pairsLoading && (
        <Card className="p-6 text-center">
          <p className="text-muted-foreground">
            No trading pairs available. Please ensure the API is running.
          </p>
        </Card>
      )}
    </div>
  );
}
