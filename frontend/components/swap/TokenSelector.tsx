'use client';

import { useState, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { ChevronDown } from 'lucide-react';
import { TokenSearchModal, AssetOption } from '@/components/shared/TokenSearchModal';
import { usePairs } from '@/hooks/useApi';
import { cn } from '@/lib/utils';

interface TokenSelectorProps {
  selectedAsset: string;
  onSelect: (asset: string) => void;
  className?: string;
  disabled?: boolean;
}

export function TokenSelector({
  selectedAsset,
  onSelect,
  className,
  disabled = false,
}: TokenSelectorProps) {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const { data: pairs, loading } = usePairs();

  // Extract unique assets from all pairs
  const assets: AssetOption[] = useMemo(() => {
    if (!pairs) return [];

    const assetMap = new Map<string, AssetOption>();
    
    // Add native XLM if not present
    assetMap.set('native', {
      code: 'XLM',
      asset: 'native',
      displayName: 'Stellar Lumens',
    });

    pairs.forEach((pair) => {
      // Base asset
      if (!assetMap.has(pair.base_asset)) {
        assetMap.set(pair.base_asset, {
          code: pair.base,
          asset: pair.base_asset,
          issuer: pair.base_asset.includes(':') ? pair.base_asset.split(':')[1] : undefined,
        });
      }
      // Counter asset
      if (!assetMap.has(pair.counter_asset)) {
        assetMap.set(pair.counter_asset, {
          code: pair.counter,
          asset: pair.counter_asset,
          issuer: pair.counter_asset.includes(':') ? pair.counter_asset.split(':')[1] : undefined,
        });
      }
    });

    return Array.from(assetMap.values());
  }, [pairs]);

  const selectedAssetOption = useMemo(() => {
    return assets.find((a) => a.asset === selectedAsset);
  }, [assets, selectedAsset]);

  const displayCode = selectedAssetOption?.code || (selectedAsset === 'native' ? 'XLM' : 'Select');
  
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
        className={cn(
          "h-11 rounded-xl px-3 gap-2 bg-background/60 hover:bg-background/80 border-border/40 shadow-sm transition-all flex-shrink-0 min-w-[120px]",
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
