'use client';

import type { OfframpQuotePreview } from '@/lib/offramp/types';
import { OFFRAMP_FIAT } from '@/lib/offramp/assets';
import { useOfframpI18n } from '@/lib/offramp-i18n';
import { cn } from '@/lib/utils';

interface OfframpQuoteSummaryProps {
  quote: OfframpQuotePreview | null;
  className?: string;
}

export function OfframpQuoteSummary({
  quote,
  className,
}: OfframpQuoteSummaryProps) {
  const { t } = useOfframpI18n();

  if (!quote) {
    return (
      <div
        className={cn(
          'rounded-2xl border border-dashed border-border/70 bg-muted/20 px-5 py-8 text-center',
          className,
        )}
        data-testid="offramp-quote-empty"
      >
        <p className="font-display text-base font-semibold text-foreground">
          {t('offramp.summary.emptyTitle')}
        </p>
        <p className="mt-1 text-sm text-muted-foreground">
          {t('offramp.summary.emptyDescription')}
        </p>
      </div>
    );
  }

  return (
    <div
      className={cn(
        'overflow-hidden rounded-2xl border border-border/70 bg-card/60',
        className,
      )}
      data-testid="offramp-quote-summary"
    >
      <div className="border-b border-border/60 bg-primary/5 px-5 py-4">
        <p className="font-mono text-[10px] font-semibold uppercase tracking-[0.22em] text-primary">
          {t('offramp.summary.receiveLabel')}
        </p>
        <p className="mt-1 font-display text-3xl font-bold tracking-tight text-foreground sm:text-4xl">
          <span className="text-primary">{OFFRAMP_FIAT.symbol}</span>
          {quote.receiveNgn}
        </p>
        <p className="mt-1 text-sm text-muted-foreground">
          {t('offramp.summary.rateSubtext', {
            netUsdc: quote.netUsdc,
            rate: quote.rateNgn.toLocaleString('en-US'),
          })}
        </p>
      </div>

      <dl className="grid gap-3 px-5 py-4 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-muted-foreground">{t('offramp.summary.youSend')}</dt>
          <dd className="font-medium">
            {quote.sourceAmount} {quote.sourceSymbol}
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t('offramp.summary.path')}</dt>
          <dd className="font-medium">
            {quote.mode === 'direct'
              ? t('offramp.summary.directPath')
              : t('offramp.summary.bridgePath')}
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">
            {t('offramp.summary.previewFee', { feePercent: '0.5' })}
          </dt>
          <dd className="font-medium">{quote.feeUsdc} USDC</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t('offramp.summary.eta')}</dt>
          <dd className="font-medium">{quote.etaLabel}</dd>
        </div>
      </dl>
    </div>
  );
}

