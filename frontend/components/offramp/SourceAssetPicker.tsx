'use client';

import { useOfframpI18n } from '@/lib/offramp-i18n';
import { cn } from '@/lib/utils';
import type { OfframpSourceAsset } from '@/lib/offramp/types';

interface SourceAssetPickerProps {
  assets: OfframpSourceAsset[];
  selectedId: string;
  onSelect: (id: string) => void;
  /** When direct mode, only Stellar USDC is selectable. */
  directOnly?: boolean;
  className?: string;
}

export function SourceAssetPicker({
  assets,
  selectedId,
  onSelect,
  directOnly = false,
  className,
}: SourceAssetPickerProps) {
  const { t } = useOfframpI18n();

  function statusLabel(asset: OfframpSourceAsset): string {
    switch (asset.status) {
      case 'ready':
        return t('offramp.source.statusReady');
      case 'bridge_required':
        return t('offramp.source.statusBridge');
      case 'swap_then_offramp':
        return t('offramp.source.statusSwap');
      case 'coming_soon':
        return t('offramp.source.statusSoon');
    }
  }

  return (
    <div className={cn('space-y-3', className)} data-testid="offramp-source-picker">
      <div className="flex items-end justify-between gap-3">
        <div>
          <h2 className="font-display text-lg font-semibold tracking-tight">
            {t('offramp.source.title')}
          </h2>
          <p className="text-sm text-muted-foreground">
            {directOnly
              ? t('offramp.source.directDescription')
              : t('offramp.source.bridgeDescription')}
          </p>
        </div>
      </div>

      <div
        className="grid gap-2 sm:grid-cols-2"
        role="listbox"
        aria-label="Source asset"
      >
        {assets.map((asset) => {
          const selected = asset.id === selectedId;
          const locked =
            (directOnly && !asset.isStellarUsdc) ||
            asset.status === 'coming_soon';

          return (
            <button
              key={asset.id}
              type="button"
              role="option"
              aria-selected={selected}
              disabled={locked}
              onClick={() => onSelect(asset.id)}
              className={cn(
                'flex items-start gap-3 rounded-xl border px-4 py-3 text-left transition-all duration-200',
                'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                selected
                  ? 'border-primary bg-primary/10'
                  : 'border-border/60 bg-background/50 hover:border-border',
                locked && 'cursor-not-allowed opacity-45',
              )}
              data-testid={`offramp-asset-${asset.id}`}
            >
              <span
                aria-hidden
                className={cn(
                  'mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-full font-mono text-xs font-bold',
                  selected
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground',
                )}
              >
                {asset.symbol.slice(0, 2)}
              </span>
              <span className="min-w-0 flex-1">
                <span className="flex items-center gap-2">
                  <span className="font-semibold text-foreground">
                    {asset.symbol}
                  </span>
                  <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
                    {asset.chainLabel}
                  </span>
                  <span
                    className={cn(
                      'ml-auto rounded-md px-1.5 py-0.5 font-mono text-[9px] font-semibold uppercase tracking-wider',
                      asset.status === 'ready' &&
                        'bg-success/15 text-success',
                      asset.status === 'bridge_required' &&
                        'bg-chart-3/15 text-chart-3',
                      asset.status === 'swap_then_offramp' &&
                        'bg-signal/15 text-signal',
                      asset.status === 'coming_soon' &&
                        'bg-muted text-muted-foreground',
                    )}
                  >
                    {statusLabel(asset)}
                  </span>
                </span>
                <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                  {asset.hint}
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

