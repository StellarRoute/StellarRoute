'use client';

import { useState, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { ChevronDown } from 'lucide-react';
import { TokenSearchModal, AssetOption } from '@/components/shared/TokenSearchModal';
import { usePairs } from '@/hooks/useApi';
import { counterpartsFor } from '@/lib/trading-pairs';
import { cn } from '@/lib/utils';

interface TokenSelectorProps {
  selectedAsset: string;
  onSelect: (asset: string) => void;
  className?: string;
  disabled?: boolean;
  /** Optional override for loading state (useful for stories/tests) */
  isLoading?: boolean;
  /**
   * When set, only show assets that share an indexed market with this asset
   * (used for the receive-side selector so users can't pick dead pairs).
   */
  compatibleWith?: string;
}

export function TokenSelector({
  selectedAsset,
  onSelect,
  className,
  disabled = false,
  isLoading: propLoading,
  compatibleWith,
}: TokenSelectorProps) {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const { data: pairs, loading: hookLoading } = usePairs();
  const loading = propLoading ?? hookLoading;

  // Extract unique assets from indexed pairs only (no synthetic XLM when
  // staging has no native markets — avoids quoting dead XLM→USDC defaults).
  const assets: AssetOption[] = useMemo(() => {
    if (!pairs) return [];

    const assetMap = new Map<string, AssetOption>();

    const allow = compatibleWith
      ? new Set(counterpartsFor(compatibleWith, pairs))
      : null;

    pairs.forEach((pair) => {
      const maybeAdd = (canonical: string, code: string) => {
        if (allow && !allow.has(canonical)) return;
        if (assetMap.has(canonical)) return;
        assetMap.set(canonical, {
          code: code === 'native' ? 'XLM' : code,
          asset: canonical,
          issuer: canonical.includes(':')
            ? canonical.split(':')[1]
            : undefined,
          displayName: code === 'native' ? 'Stellar Lumens' : undefined,
        });
      };

      maybeAdd(pair.base_asset, pair.base);
      maybeAdd(pair.counter_asset, pair.counter);
    });

    return Array.from(assetMap.values()).sort((a, b) =>
      a.code.localeCompare(b.code)
    );
  }, [pairs, compatibleWith]);

  const selectedAssetOption = useMemo(() => {
    return assets.find((a) => a.asset === selectedAsset);
  }, [assets, selectedAsset]);

  const displayCode =
    selectedAssetOption?.code ||
    (selectedAsset === 'native'
      ? 'XLM'
      : selectedAsset.includes(':')
        ? selectedAsset.split(':')[0]
        : 'Select');
  
  // Simple icon generator based on code
  const renderIcon = (code: string) => {
    const firstChar = code.charAt(0).toUpperCase();
    const colors = [
      'bg-blue-500', 'bg-orange-500', 'bg-purple-500', 
      'bg-green-500', 'bg-pink-500', 'bg-yellow-500'
    ];
    const colorIndex = code.length % colors.length;
    
    if (code === 'XLM') {
      return (
        <div className="w-6 h-6 rounded-full bg-primary/20 flex items-center justify-center text-[10px] font-bold text-primary border border-primary/20">
          <div className="w-3 h-3 rounded-full bg-primary/80" />
        </div>
      );
    }
    
    return (
      <div className={cn("w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold text-white", colors[colorIndex])}>
        {firstChar}
      </div>
    );
  };

  return (
    <>
      <Button
        variant="secondary"
        onClick={() => setIsModalOpen(true)}
        disabled={disabled || loading}
        aria-label={
          selectedAssetOption
            ? `Select token, currently ${selectedAssetOption.code}`
            : selectedAsset === 'native'
              ? 'Select token, currently XLM'
              : 'Select token'
        }
        className={cn(
          "h-11 min-h-11 rounded-xl px-3 gap-2 bg-background/60 hover:bg-background/80 border-border/40 shadow-sm transition-all flex-shrink-0 min-w-[120px]",
          className
        )}
      >
        {renderIcon(displayCode)}
        <span className="font-bold text-base">{displayCode}</span>
        <ChevronDown className="h-4 w-4 opacity-50" />
      </Button>

      <TokenSearchModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        assets={assets}
        onSelect={onSelect}
        title="Select to Token"
        selectedAsset={selectedAsset}
      />
    </>
  );
}
