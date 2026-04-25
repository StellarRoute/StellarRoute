"use client";

import React, { useMemo, useState } from "react";
import { ArrowLeftRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { toast } from "sonner";
import type { TradingPair } from "@/types";
import { AssetIcon } from "@/components/shared/AssetIcon";
import { TokenSearchModal } from "@/components/shared/TokenSearchModal";
import { useRecentTokens } from "@/hooks/useRecentTokens";

export interface TokenPairSelectorProps {
  /** Available trading pairs from the API */
  pairs: TradingPair[];
  /** Currently selected base asset (sell) */
  selectedBase?: string;
  /** Currently selected quote asset (buy) */
  selectedQuote?: string;
  /** Callback when pair selection changes */
  onPairChange: (base: string, quote: string) => void;
  /** Loading state */
  loading?: boolean;
  /** Error message when no valid pair exists */
  error?: string;
  /** Optional className for styling */
  className?: string;
}

interface AssetOption {
  code: string;
  asset: string;
  issuer?: string;
  displayName: string;
}

function truncateIssuer(issuer: string, maxLength = 12): string {
  if (issuer.length <= maxLength) return issuer;
  return `${issuer.slice(0, 6)}...${issuer.slice(-4)}`;
}

function parseAsset(assetString: string): { code: string; issuer?: string } {
  if (assetString === "native") {
    return { code: "XLM" };
  }
  const parts = assetString.split(":");
  return {
    code: parts[0],
    issuer: parts[1],
  };
}

function AssetButton({
  label,
  code,
  issuer,
  onClick,
  disabled,
}: {
  label: string;
  code: string;
  issuer?: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "flex flex-col items-start gap-2 rounded-lg border bg-background p-3 transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      )}
    >
      <span className="text-xs text-muted-foreground">{label}</span>
      <div className="flex items-center gap-2">
        <AssetIcon
          symbol={code || "Select"}
          className="size-8 border-border/50 bg-primary/5"
        />
        <span className="text-lg font-semibold">{code || "Select"}</span>
      </div>
      {issuer && (
        <span className="text-xs text-muted-foreground font-mono">
          {truncateIssuer(issuer)}
        </span>
      )}
    </button>
  );
}

export function TokenPairSelector({
  pairs,
  selectedBase,
  selectedQuote,
  onPairChange,
  loading = false,
  error,
  className,
}: TokenPairSelectorProps) {
  const [baseDialogOpen, setBaseDialogOpen] = useState(false);
  const [quoteDialogOpen, setQuoteDialogOpen] = useState(false);
  const { addRecentToken } = useRecentTokens();

  const { baseAssets, quoteAssets, validPairs } = useMemo(() => {
    const baseSet = new Map<string, AssetOption>();
    const quoteSet = new Map<string, AssetOption>();
    const pairMap = new Map<string, Set<string>>();

    pairs.forEach((pair) => {
      const baseInfo = parseAsset(pair.base_asset);
      const quoteInfo = parseAsset(pair.counter_asset);

      baseSet.set(pair.base_asset, {
        code: pair.base,
        asset: pair.base_asset,
        issuer: baseInfo.issuer,
        displayName: pair.base,
      });

      quoteSet.set(pair.counter_asset, {
        code: pair.counter,
        asset: pair.counter_asset,
        issuer: quoteInfo.issuer,
        displayName: pair.counter,
      });

      if (!pairMap.has(pair.base_asset)) {
        pairMap.set(pair.base_asset, new Set());
      }
      pairMap.get(pair.base_asset)!.add(pair.counter_asset);
    });

    return {
      baseAssets: Array.from(baseSet.values()).sort((a, b) =>
        a.code.localeCompare(b.code)
      ),
      quoteAssets: Array.from(quoteSet.values()).sort((a, b) =>
        a.code.localeCompare(b.code)
      ),
      validPairs: pairMap,
    };
  }, [pairs]);

  const availableQuoteAssets = useMemo(() => {
    if (!selectedBase) return quoteAssets;
    const validQuotes = validPairs.get(selectedBase);
    if (!validQuotes) return [];
    return quoteAssets.filter((asset) => validQuotes.has(asset.asset));
  }, [selectedBase, quoteAssets, validPairs]);

  const selectedBaseInfo = useMemo(() => {
    if (!selectedBase) return null;
    return baseAssets.find((a) => a.asset === selectedBase);
  }, [selectedBase, baseAssets]);

  const selectedQuoteInfo = useMemo(() => {
    if (!selectedQuote) return null;
    return quoteAssets.find((a) => a.asset === selectedQuote);
  }, [selectedQuote, quoteAssets]);

  const isPairValid = useMemo(() => {
    if (!selectedBase || !selectedQuote) return true;
    return validPairs.get(selectedBase)?.has(selectedQuote) ?? false;
  }, [selectedBase, selectedQuote, validPairs]);

  const canSwapSides = useMemo(() => {
    if (!selectedBase || !selectedQuote) return false;
    return validPairs.get(selectedQuote)?.has(selectedBase) ?? false;
  }, [selectedBase, selectedQuote, validPairs]);

  const handleSwapSides = () => {
    if (!selectedBase || !selectedQuote) return;
    if (canSwapSides) {
      onPairChange(selectedQuote, selectedBase);
    } else {
      toast.error("This pair cannot be swapped", {
        description: "The reverse trading pair is not available",
      });
    }
  };

  const handleBaseSelect = (asset: string) => {
    if (selectedQuote && !validPairs.get(asset)?.has(selectedQuote)) {
      onPairChange(asset, "");
    } else {
      onPairChange(asset, selectedQuote || "");
    }
    addRecentToken(asset);
  };

  const handleQuoteSelect = (asset: string) => {
    onPairChange(selectedBase || "", asset);
    addRecentToken(asset);
  };

  return (
    <Card className={cn("p-4", className)}>
      <div className="space-y-4">
        <div className="flex items-center gap-3">
          <div className="flex-1">
            {loading ? (
              <Skeleton className="h-[74px] w-full rounded-lg" />
            ) : (
              <AssetButton
                label="You sell"
                code={selectedBaseInfo?.code || ""}
                issuer={selectedBaseInfo?.issuer}
                onClick={() => setBaseDialogOpen(true)}
                disabled={loading || pairs.length === 0}
              />
            )}
          </div>

          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={handleSwapSides}
            disabled={
              loading || !selectedBase || !selectedQuote || !canSwapSides
            }
            title="Swap base and quote assets"
            className="shrink-0"
          >
            <ArrowLeftRight className="h-4 w-4" />
            <span className="sr-only">Swap base and quote assets</span>
          </Button>

          <div className="flex-1">
            {loading ? (
              <Skeleton className="h-[74px] w-full rounded-lg" />
            ) : (
              <AssetButton
                label="You buy"
                code={selectedQuoteInfo?.code || ""}
                issuer={selectedQuoteInfo?.issuer}
                onClick={() => setQuoteDialogOpen(true)}
                disabled={loading || pairs.length === 0 || !selectedBase}
              />
            )}
          </div>
        </div>

        {!isPairValid && selectedBase && selectedQuote && (
          <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3">
            <p className="text-sm text-destructive font-medium">
              Invalid pair selection
            </p>
            <p className="text-xs text-destructive/80 mt-1">
              This trading pair is not available. Please{" "}
              <button
                type="button"
                onClick={() => onPairChange(selectedBase, "")}
                className="underline hover:no-underline font-medium"
              >
                select a different quote asset
              </button>{" "}
              or{" "}
              <button
                type="button"
                onClick={() => onPairChange("", "")}
                className="underline hover:no-underline font-medium"
              >
                reset your selection
              </button>
              .
            </p>
          </div>
        )}

        {error && (
          <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3">
            <p className="text-sm text-destructive">{error}</p>
          </div>
        )}
      </div>

      <TokenSearchModal
        isOpen={baseDialogOpen}
        onClose={() => setBaseDialogOpen(false)}
        assets={baseAssets}
        onSelect={handleBaseSelect}
        title="Select asset to sell"
        selectedAsset={selectedBase}
      />

      <TokenSearchModal
        isOpen={quoteDialogOpen}
        onClose={() => setQuoteDialogOpen(false)}
        assets={availableQuoteAssets}
        onSelect={handleQuoteSelect}
        title="Select asset to buy"
        selectedAsset={selectedQuote}
      />
    </Card>
  );
}
