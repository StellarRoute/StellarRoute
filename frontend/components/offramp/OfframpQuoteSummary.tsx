'use client';

import type { OfframpQuotePreview } from '@/lib/offramp/types';
import { OFFRAMP_FIAT } from '@/lib/offramp/assets';
import { cn } from '@/lib/utils';

interface OfframpQuoteSummaryProps {
  quote: OfframpQuotePreview | null;
  className?: string;
}

export function OfframpQuoteSummary({
  quote,
  className,
}: OfframpQuoteSummaryProps) {
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
          Enter an amount to preview Naira
        </p>
        <p className="mt-1 text-sm text-muted-foreground">
          Quotes are indicative until the payout partner is connected.
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
          You receive · indicative
        </p>
        <p className="mt-1 font-display text-3xl font-bold tracking-tight text-foreground sm:text-4xl">
          <span className="text-primary">{OFFRAMP_FIAT.symbol}</span>
          {quote.receiveNgn}
        </p>
        <p className="mt-1 text-sm text-muted-foreground">
          ≈ {quote.netUsdc} USDC after fee · 1 USDC ≈ ₦
          {quote.rateNgn.toLocaleString('en-US')}
        </p>
      </div>

      <dl className="grid gap-3 px-5 py-4 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-muted-foreground">You send</dt>
          <dd className="font-medium">
            {quote.sourceAmount} {quote.sourceSymbol}
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Path</dt>
          <dd className="font-medium">
            {quote.mode === 'direct' ? 'Direct Stellar USDC' : 'Bridge → offramp'}
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">Preview fee (0.5%)</dt>
          <dd className="font-medium">{quote.feeUsdc} USDC</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">ETA</dt>
          <dd className="font-medium">{quote.etaLabel}</dd>
        </div>
      </dl>
    </div>
  );
}
