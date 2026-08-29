"use client";

import { useMemo, useState, useEffect } from "react";
import { TokenSelector, type TokenOption } from "@/components/shared/TokenSelector";
import { usePairs } from "@/hooks/useApi";

export interface PairSelectorProps {
  onPairChange?: (base: string, quote: string) => void;
}

export function PairSelector({ onPairChange }: PairSelectorProps) {
  const { data: pairs, loading } = usePairs();
  const [baseAsset, setBaseAsset] = useState<string>("native");
  const [quoteAsset, setQuoteAsset] = useState<string>("native");

  const options: TokenOption[] = useMemo(() => {
    if (!pairs) return [];
    const assetMap = new Map<string, { code: string; asset: string }>();
    
    pairs.forEach((pair) => {
      const addAsset = (asset: string, code: string) => {
        if (!assetMap.has(asset)) {
          assetMap.set(asset, { code, asset });
        }
      };
      addAsset(pair.base_asset, pair.base);
      addAsset(pair.counter_asset, pair.counter);
    });

    return Array.from(assetMap.values())
      .map(({ code, asset }) => ({
        value: asset,
        label: asset === "native" ? "Stellar" : asset.split(":")[1] || "Unknown",
        symbol: code,
      }))
      .sort((a, b) => a.symbol.localeCompare(b.symbol));
  }, [pairs]);

  // Set sensible defaults when options load
  useEffect(() => {
    if (options.length > 0) {
      if (!options.find((o) => o.value === baseAsset)) {
        setBaseAsset(options[0].value);
      }
      if (options.length > 1 && !options.find((o) => o.value === quoteAsset)) {
        const nonNative = options.find((o) => o.value !== "native");
        setQuoteAsset(nonNative ? nonNative.value : options[1].value);
      }
    }
  }, [options, baseAsset, quoteAsset]);

  const handleBaseChange = (value: string) => {
    setBaseAsset(value);
    onPairChange?.(value, quoteAsset);
  };

  const handleQuoteChange = (value: string) => {
    setQuoteAsset(value);
    onPairChange?.(baseAsset, value);
  };

  return (
    <div className="flex flex-col sm:flex-row gap-4 w-full max-w-md mb-6">
      <div className="flex-1 space-y-2">
        <label className="text-sm font-medium text-muted-foreground">Base Asset</label>
        <TokenSelector
          value={baseAsset}
          options={options}
          loading={loading}
          placeholder="Select base"
          onChange={handleBaseChange}
        />
      </div>
      <div className="flex-1 space-y-2">
        <label className="text-sm font-medium text-muted-foreground">Quote Asset</label>
        <TokenSelector
          value={quoteAsset}
          options={options}
          loading={loading}
          placeholder="Select quote"
          onChange={handleQuoteChange}
        />
      </div>
    </div>
  );
}