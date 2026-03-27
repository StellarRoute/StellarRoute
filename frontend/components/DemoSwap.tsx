"use client";

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw } from "lucide-react";
import { toast } from "sonner";
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
import { usePairs } from "@/hooks/useApi";
import { useQuoteRefresh } from "@/hooks/useQuoteRefresh";
import { useTransactionHistory } from "@/hooks/useTransactionHistory";
import { useWallet } from "@/components/providers/wallet-provider";
import { useSettings } from "@/components/providers/settings-provider";

import type { PathStep, TradingPair } from "@/types";
import { TransactionStatus } from "@/types/transaction";
import {
  formatMaxAmountForInput,
  maxDecimalsForSellAsset,
} from "@/lib/amount-input";
import { QUOTE_AUTO_REFRESH_INTERVAL_MS } from "@/lib/quote-stale";
import { TradeRouteDisplay } from "@/components/shared/TradeRouteDisplay";
import { SwapValidationSchema } from "@/lib/swap-validation";

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
  const [slippage, setSlippage] = useState<number | null>(0.5);

  const [isModalOpen, setIsModalOpen] = useState(false);
  const [txStatus, setTxStatus] = useState<TransactionStatus | "review">(
    "review",
  );
  const [errorMessage, setErrorMessage] = useState<string>();
  const [txHash, setTxHash] = useState<string>();

  const { addTransaction } = useTransactionHistory(MOCK_WALLET);

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

  const inputValidation = SwapValidationSchema.validate(
    {
      amount: sellRaw,
      maxDecimals: sellMaxDecimals,
      sellAssetId: selectedPair?.base_asset,
      buyAssetId: selectedPair?.counter_asset,
      slippage,
    },
    { mode: "input" },
  );
  const submitValidation = SwapValidationSchema.validate(
    {
      amount: sellRaw,
      maxDecimals: sellMaxDecimals,
      sellAssetId: selectedPair?.base_asset,
      buyAssetId: selectedPair?.counter_asset,
      slippage,
    },
    { mode: "submit" },
  );

  const parseResult = inputValidation.amountResult;

  const numericForQuote = inputValidation.parsed.amount?.numeric;

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
    isStale,
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
    if (!submitValidation.isValid) {
      toast.error(submitValidation.issues[0]?.message ?? "Invalid swap inputs.");
      return;
    }

    setTxStatus("review");
    setErrorMessage(undefined);
    setTxHash(undefined);
    setIsModalOpen(true);
  };

  const handleConfirm = () => {
    setTxStatus("pending");

    setTimeout(() => {
      setTxStatus("submitting");

      setTimeout(() => {
        setTxStatus("processing");

        setTimeout(() => {
          const isSuccess = Math.random() > 0.2;
          const fromAmt =
            parseResult.status === "ok" ? parseResult.normalized : "0";
          const toAmt = quote?.total ?? "10.5";
          const resolvedPriceImpact =
            quote?.priceImpact != null ? `${quote.priceImpact}%` : "—";

          if (isSuccess) {
            const mockHash = "mock_tx_" + Math.random().toString(36).substring(7);
            setTxHash(mockHash);
            setTxStatus("success");

            toast.success("Transaction Successful!", {
              description: `Swapped ${fromAmt} ${selectedPair?.base ?? ""} for ${toAmt} ${selectedPair?.counter ?? ""}`,
            });

            addTransaction({
              id: mockHash,
              timestamp: Date.now(),
              fromAsset: selectedPair?.base ?? "XLM",
              fromAmount: fromAmt,
              toAsset: selectedPair?.counter ?? "USDC",
              toAmount: toAmt,
              exchangeRate: quote?.price ?? "0.105",
              priceImpact: resolvedPriceImpact,
              minReceived: toAmt,
              networkFee: "0.00001",
              routePath: quote?.path?.length ? quote.path : mockRoute,
              status: "success",
              hash: mockHash,
              walletAddress: MOCK_WALLET,
            });
          } else {
            setTxStatus("failed");
            setErrorMessage(
              "Insufficient balance or network congestion. Please try again.",
            );

            toast.error("Transaction Failed", {
              description: "Insufficient balance or network congestion.",
            });

            addTransaction({
              id: "failed_" + Date.now(),
              timestamp: Date.now(),
              fromAsset: selectedPair?.base ?? "XLM",
              fromAmount: fromAmt,
              toAsset: selectedPair?.counter ?? "USDC",
              toAmount: toAmt,
              exchangeRate: quote?.price ?? "0.105",
              priceImpact: resolvedPriceImpact,
              minReceived: toAmt,
              networkFee: "0.00001",
              routePath: quote?.path?.length ? quote.path : mockRoute,
              status: "failed",
              errorMessage: "Insufficient balance.",
              walletAddress: MOCK_WALLET,
            });
          }
        }, 2000);
      }, 1000);
    }, 2000);
  };

  const handleCancel = () => {
    setTxStatus("review");
  };

  const receivePreview =
    quote && parseResult.status === "ok" ? quote.total : "—";

  const priceImpactDisplay =
    quote?.priceImpact != null ? `${quote.priceImpact}%` : "—";

  const slippageWarning = inputValidation.warnings.slippage;
  const slippageError = inputValidation.fieldErrors.slippage;

  return (
    <Card className="mx-auto mt-8 max-w-lg border-primary/20 bg-background/50 p-6 shadow-lg backdrop-blur-sm">
      <div className="space-y-4">
        <div>
          <h2 className="mb-1 text-xl font-bold">Swap Tokens</h2>
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

        <div className="space-y-4 rounded-lg border bg-muted/20 p-4">
          <div>
            <span className="text-sm font-medium">Estimated receive</span>
            <div className="mt-1 text-2xl font-bold text-success">
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
              <p className="mt-1 text-xs text-destructive">
                Quote failed: {quoteError.message}
              </p>
            )}
          </div>

          <div>
            <span className="text-sm font-medium text-muted-foreground">
              Reference price
            </span>
            <div className="mt-1 flex items-center gap-2 text-sm">
              <span>{quote?.price ?? "—"}</span>
              {isStale && (
                <span className="inline-flex items-center rounded-md bg-yellow-400/10 px-2 py-1 text-xs font-medium text-yellow-500 ring-1 ring-inset ring-yellow-400/20">
                  Stale
                </span>
              )}
            </div>
          </div>

          <div className="flex justify-between text-sm">
            <span>
              Price impact{" "}
              <span title="How much this trade moves the quoted market price">
                ⓘ
              </span>
            </span>
            <span>{priceImpactDisplay}</span>
          </div>

          <div className="space-y-2">
            <span className="text-sm font-medium">Slippage tolerance</span>

            <div className="flex gap-2">
              {[0.1, 0.5, 1].map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => setSlippage(preset)}
                  className={`rounded-md border px-3 py-1 text-sm ${slippage === preset ? "bg-primary text-primary-foreground" : ""
                    }`}
                >
                  {preset}%
                </button>
              ))}
            </div>

            <div className="flex items-center gap-2">
              <Input
                type="number"
                min={0}
                max={50}
                step="0.1"
                value={slippage ?? ""}
                onChange={(e) => {
                  const raw = e.target.value;
                  if (raw === "") {
                    setSlippage(null);
                    return;
                  }

                  const val = Number(raw);
                  if (Number.isNaN(val)) {
                    setSlippage(null);
                    return;
                  }

                  setSlippage(val);
                }}
                placeholder="Custom %"
                className="w-24"
              />
              <span className="text-sm">%</span>
            </div>

            {slippageError && (
              <p className="text-xs text-destructive">
                {slippageError}
              </p>
            )}

            {!slippageError && slippageWarning && (
              <p className="text-xs text-yellow-500">{slippageWarning}</p>
            )}
          </div>

        {numericForQuote && (
          <TradeRouteDisplay 
            quote={quote || null} 
            isLoading={quoteLoading} 
            error={quoteError?.message}
          />
        )}

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
          className="h-12 w-full text-lg"
          onClick={handleSwapClick}
          disabled={
            !submitValidation.isValid
          }
        >
          Review Swap
        </Button>
      </div>

      <TransactionConfirmationModal
        isOpen={isModalOpen}
        onOpenChange={setIsModalOpen}
        fromAsset={selectedPair?.base ?? "XLM"}
        fromAmount={
          parseResult.status === "ok" ? parseResult.normalized : sellRaw || "0"
        }
        toAsset={selectedPair?.counter ?? "USDC"}
        toAmount={quote?.total ?? "—"}
        exchangeRate={quote?.price ?? "—"}
        priceImpact="0.1%"
        slippageTolerancePct={settings.slippageTolerance}
        networkFee="0.00001"
        routePath={quote?.path?.length ? quote.path : mockRoute}
        onConfirm={handleConfirm}
        onCancel={handleCancel}
        status={txStatus}
        errorMessage={errorMessage}
        txHash={txHash}
      />
    </Card>
  );
}
