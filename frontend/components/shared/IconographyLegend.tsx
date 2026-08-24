'use client';

import { cn } from '@/lib/utils';
import { useSwapI18n } from '@/lib/swap-i18n';
import { TransactionStatusBadge } from '@/components/shared/TransactionStatusBadge';
import { VenueTypeBadge } from '@/components/shared/VenueTypeBadge';

const VENUE_TYPES: Array<'SDEX' | 'AMM' | 'Hybrid'> = ['SDEX', 'AMM', 'Hybrid'];
const TRANSACTION_STATUSES: Array<
  | 'pending'
  | 'submitted'
  | 'confirmed'
  | 'failed'
  | 'dropped'
> = ['pending', 'submitted', 'confirmed', 'failed', 'dropped'];

interface IconographyLegendProps {
  className?: string;
  embedded?: boolean;
}

export function IconographyLegend({ className, embedded = false }: IconographyLegendProps) {
  const { t } = useSwapI18n();

  return (
    <details
      className={cn(
        embedded
          ? 'rounded-2xl border border-border/70 bg-muted/40'
          : 'rounded-3xl border border-border bg-muted/80 shadow-sm',
        className,
      )}
      data-testid="iconography-legend"
    >
      <summary
        className={cn(
          'cursor-pointer list-none px-4 py-3 text-sm font-semibold',
          'rounded-2xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
          '[&::-webkit-details-marker]:hidden',
        )}
      >
        <span className="flex items-center justify-between gap-3">
          <span>{t('swap.iconography.disclosure')}</span>
          <span aria-hidden="true" className="text-xs text-muted-foreground">
            ▾
          </span>
        </span>
      </summary>

      <section
        className={cn('flex flex-col gap-3 px-4 pb-4', embedded ? 'pt-1' : 'pt-2')}
        aria-label={t('swap.iconography.title')}
      >
        <div>
          <p className="text-sm uppercase tracking-[0.24em] text-muted-foreground font-semibold mb-2">
            {t('swap.iconography.eyebrow')}
          </p>
          <h2 className="text-xl font-semibold">{t('swap.iconography.title')}</h2>
          <p className="text-sm text-muted-foreground mt-2">
            {t('swap.iconography.description')}
          </p>
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
          <div className="space-y-3">
            <h3 className="text-sm font-semibold">{t('swap.iconography.venueTypes')}</h3>
            <div className="flex flex-wrap gap-2">
              {VENUE_TYPES.map((type) => (
                <VenueTypeBadge key={type} type={type} size={20} />
              ))}
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground">
              <p>{t('swap.iconography.venueTypes.sdex')}</p>
              <p>{t('swap.iconography.venueTypes.hybrid')}</p>
            </div>
          </div>

          <div className="space-y-3">
            <h3 className="text-sm font-semibold">{t('swap.iconography.transactionStates')}</h3>
            <div className="flex flex-wrap gap-2">
              {TRANSACTION_STATUSES.map((status) => (
                <TransactionStatusBadge key={status} status={status} size={20} />
              ))}
            </div>
            <div className="grid gap-2 text-xs text-muted-foreground">
              <p>{t('swap.iconography.sizingNote')}</p>
              <p>{t('swap.iconography.assetFallbackNote')}</p>
            </div>
          </div>
        </div>
      </section>
    </details>
  );
}
