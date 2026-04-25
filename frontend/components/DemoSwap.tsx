"use client";

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { TransactionConfirmationModal } from "@/components/shared/TransactionConfirmationModal";
import { TradeRouteDisplay } from "@/components/shared/TradeRouteDisplay";
import { usePairs } from "@/hooks/useApi";
import { useQuoteRefresh } from "@/hooks/useQuoteRefresh";
import { useTransactionLifecycle } from "@/hooks/useTransactionLifecycle";
import { useWallet } from "@/components/providers/wallet-provider";
import { useSettings } from "@/components/providers/settings-provider";
import { toast } from "sonner";
import type { PathStep, TradingPair } from "@/types";
import {
  formatMaxAmountForInput,
  maxDecimalsForSellAsset,
  parseSellAmount,
} from "@/lib/amount-input";

import { QUOTE_AUTO_REFRESH_INTERVAL_MS } from "@/lib/quote-stale";

const MOCK_WALLET = "GBSU...XYZ9";

function pairKey(p: TradingPair): string {
  return `${p.base_asset}__${p.counter_asset}`;
}

const mockRoute: PathStep[] = [
  {
    from_asset: { asset_type: "native" },
    to_asset: {
      asset_type: "credit_alphanum4",
      asset_code: "USDC",
      asset_issuer: "GA5Z...",
    },
    price: "0.105",
    source: "sdex",
  },
];

export function DemoSwap() {
  const { data: pairs, loading: pairsLoading, error: pairsError } = usePairs();
  const { isConnected, stubSpendableBalance } = useWallet();
  const { settings } = useSettings();

  const [selectedKey, setSelectedKey] = useState<string>("");
  const [sellRaw, setSellRaw] = useState<string>("");
  const [isModalOpen, setIsModalOpen] = useState(false);

  const {
    status: txStatus,
    txHash,
    errorMessage,
    initiateSwap,
    cancel,
    resubmit,
    tryAgain,
    dismiss,
  } = useTransactionLifecycle();

  useEffect(() => {
    if (!pairs?.length) return;
    setSelectedKey((current: string) => {
      if (current && pairs.some((p) => pairKey(p) === current)) {
        return current;
      }
      return pairKey(pairs[0]);
    });
  }, [pairs]);

  const selectedPair = useMemo(
    () => pairs?.find((p) => pairKey(p) === selectedKey) ?? null,
    [pairs, selectedKey],
  );

  const sellMaxDecimals = selectedPair
    ? maxDecimalsForSellAsset(
        selectedPair.base_asset,
        selectedPair.base_decimals,
      )
    : maxDecimalsForSellAsset("native");

  const parseResult = parseSellAmount(sellRaw, sellMaxDecimals);

  const numericForQuote =
    parseResult.status === "ok" ? parseResult.numeric : undefined;

  const quoteBase = selectedPair?.base_asset ?? "";
  const quoteCounter = selectedPair?.counter_asset ?? "";

  const {
    data: quote,
    loading: quoteLoading,
    error: quoteError,
    refresh,
    manualRefreshCoolingDown,
    autoRefreshEnabled,
    setAutoRefreshEnabled,
  } = useQuoteRefresh(quoteBase, quoteCounter, numericForQuote, "sell");

  const refreshDisabled = quoteLoading || manualRefreshCoolingDown || !numericForQuote;

  const amountInputInvalid =
    sellRaw.trim() !== "" &&
    parseResult.status !== "ok" &&
    parseResult.status !== "empty";

  const maxButtonTitle = !isConnected
    ? "Connect wallet to use your maximum balance"
    : undefined;

  const applyMax = useCallback(() => {
    if (!isConnected || stubSpendableBalance == null) return;
    setSellRaw(formatMaxAmountForInput(stubSpendableBalance, sellMaxDecimals));
  }, [isConnected, stubSpendableBalance, sellMaxDecimals]);

  const handleSwapClick = () => {
    if (parseResult.status !== "ok" || !selectedPair) {
      toast.error("Enter a valid sell amount and select a pair.");
      return;
    }
    setIsModalOpen(true);
  };

  const handleConfirm = () => {
    if (parseResult.status !== "ok" || !selectedPair) return;
    const fromAmt = parseResult.normalized;
    const toAmt = quote?.total ?? "10.5";

    initiateSwap({
      fromAsset: selectedPair.base ?? "XLM",
      fromAmount: fromAmt,
      toAsset: selectedPair.counter ?? "USDC",
      toAmount: toAmt,
      exchangeRate: quote?.price ?? "0.105",
      priceImpact: "0.1%",
      minReceived: toAmt,
      networkFee: "0.00001",
      routePath: quote?.path?.length ? quote.path : mockRoute,
      walletAddress: MOCK_WALLET,
    }).then(() => {
      if (txStatus === "confirmed") {
        toast.success("Transaction Successful!", {
          description: `Swapped ${fromAmt} ${selectedPair.base ?? ""} for ${toAmt} ${selectedPair.counter ?? ""}`,
        });
      }
    });
  };

  const handleDismiss = () => {
    dismiss();
    setIsModalOpen(false);
  };

  const handleDone = () => {
    dismiss();
    setIsModalOpen(false);
  };

  const handleTryAgain = () => {
    tryAgain();
  };

  const handleResubmit = () => {
    resubmit();
  };

  const receivePreview =
    quote && parseResult.status === "ok" ? quote.total : "—";

  return (
    <Card className="p-6 max-w-lg mx-auto shadow-lg mt-8 border-primary/20 bg-background/50 backdrop-blur-sm">
      <div className="space-y-4">
        <div>
          <h2 className="text-xl font-bold mb-1">Swap Tokens</h2>
          <p className="text-sm text-muted-foreground">
            Demo swap with sell amount validation and debounced quotes
          </p>
        </div>

        <div className="space-y-2">
          <span className="text-sm font-medium">Pair</span>
          {pairsLoading ? (
            <p className="text-sm text-muted-foreground">Loading pairs…</p>
          ) : pairsError ? (
            <p className="text-sm text-destructive">
              Could not load pairs. Start the API to select a market.
            </p>
          ) : pairs && pairs.length > 0 ? (
            <Select value={selectedKey} onValueChange={setSelectedKey}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Select pair" />
              </SelectTrigger>
              <SelectContent>
                {pairs.map((p) => (
                  <SelectItem key={pairKey(p)} value={pairKey(p)}>
                    {p.base} / {p.counter}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <p className="text-sm text-muted-foreground">
              No pairs from the indexer yet. You can still try amount validation
              (sell asset defaults to native precision).
            </p>
          )}
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="text-sm font-medium">
              Sell amount
              {selectedPair ? ` (${selectedPair.base})` : " (XLM)"}
            </span>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8"
              disabled={!isConnected}
              title={maxButtonTitle}
              onClick={applyMax}
            >
              Max
            </Button>
          </div>
          <Input
            inputMode="decimal"
            autoComplete="off"
            placeholder="0.0"
            value={sellRaw}
            aria-invalid={amountInputInvalid}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setSellRaw(e.target.value)}
            className="text-lg font-medium"
          />
          <div className="min-h-[1.25rem] text-xs">
            {parseResult.status === "precision_exceeded" && (
              <span className="text-destructive">{parseResult.message}</span>
            )}
            {parseResult.status === "invalid" && (
              <span className="text-destructive">{parseResult.message}</span>
            )}
            {parseResult.status === "ok" && (
              <span className="text-muted-foreground">
                Up to {sellMaxDecimals} decimals for this asset. Quotes update
                after you stop typing.
              </span>
            )}
            {parseResult.status === "empty" && sellRaw.trim() === "" && (
              <span className="text-muted-foreground">
                Paste amounts with US (1,234.56) or EU (1.234,56) grouping.
                Scientific notation is not supported.
              </span>
            )}
          </div>
        </div>

        <div className="space-y-4 bg-muted/20 p-4 rounded-lg border">
          <div>
            <span className="text-sm font-medium">Estimated receive</span>
            <div className="text-2xl font-bold mt-1 text-success">
              {quoteLoading && numericForQuote !== undefined ? (
                <span className="flex items-center gap-2">
                  <Loader2 className="h-5 w-5 animate-spin" />
                  ~ …
                </span>
              ) : (
                <>
                  {receivePreview}
                  {selectedPair ? ` ${selectedPair.counter}` : ""}
                </>
              )}
            </div>
            {quoteError && numericForQuote !== undefined && (
              <p className="text-xs text-destructive mt-1">
                Quote failed: {quoteError.message}
              </p>
            )}
          </div>
          <div>
            <span className="text-sm font-medium text-muted-foreground">
              Reference price
            </span>
            <div className="text-sm mt-1">{quote?.price ?? "—"}</div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={refreshDisabled}
              onClick={() => refresh()}
              className="gap-2"
            >
              {quoteLoading ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              ) : (
                <RefreshCw className="h-4 w-4" aria-hidden />
              )}
              Refresh quote
            </Button>
            <label className="flex cursor-pointer items-center gap-2 text-sm text-muted-foreground">
              <input
                type="checkbox"
                className="h-4 w-4 rounded border-input"
                checked={autoRefreshEnabled}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAutoRefreshEnabled(e.target.checked)}
              />
              Auto-refresh (~{Math.round(QUOTE_AUTO_REFRESH_INTERVAL_MS / 1000)}s,
              pauses when tab hidden)
            </label>
          </div>
        </div>

        <Button
          className="w-full text-lg h-12"
          onClick={handleSwapClick}
          disabled={!selectedPair || parseResult.status !== "ok"}
        >
          Review Swap
        </Button>
      </div>

      <TransactionConfirmationModal
        isOpen={isModalOpen}
        onOpenChange={(open) => {
          if (!open && txStatus !== "pending" && txStatus !== "submitted") {
            setIsModalOpen(false);
          }
        }}
        fromAsset={selectedPair?.base ?? "XLM"}
        fromAmount={parseResult.status === "ok" ? parseResult.normalized : ""}
        toAsset={selectedPair?.counter ?? "USDC"}
        toAmount={quote?.total ?? "—"}
        exchangeRate={quote?.price ?? "—"}
        priceImpact="0.1%"
        networkFee="0.00001"
        slippageTolerancePct={settings?.slippageTolerance}
        routePath={quote?.path?.length ? quote.path : mockRoute}
        status={txStatus}
        txHash={txHash}
        errorMessage={errorMessage}
        onConfirm={handleConfirm}
        onCancel={cancel}
        onTryAgain={handleTryAgain}
        onResubmit={handleResubmit}
        onDismiss={handleDismiss}
        onDone={handleDone}
      />
    </Card>
  );
}
