"use client";

import { useSettings } from '@/components/providers/settings-provider';
import { formatRate, formatFee, formatPercentage } from '@/lib/formatting';

interface QuoteSummaryProps {
  rate?: string;
  fee?: string;
  priceImpact?: string;
  fromAmount?: number;
  fromSymbol?: string;
  toAmount?: number;
  toSymbol?: string;
  feeAmount?: number;
  feeSymbol?: string;
  priceImpactValue?: number;
}

export function QuoteSummary({ 
  rate, 
  fee, 
  priceImpact,
  fromAmount = 1,
  fromSymbol = 'XLM',
  toAmount,
  toSymbol = 'USDC',
  feeAmount,
  feeSymbol = 'XLM',
  priceImpactValue
}: QuoteSummaryProps) {
  const { settings } = useSettings();
  const locale = settings.locale;

  // Use provided formatted strings or generate them from numeric values
  const displayRate = rate || (toAmount ? formatRate(fromAmount, fromSymbol, toAmount, toSymbol, locale) : '');
  const displayFee = fee || (feeAmount !== undefined ? formatFee(feeAmount, feeSymbol, locale) : '');
  const displayPriceImpact = priceImpact || (priceImpactValue !== undefined ? formatPercentage(priceImpactValue, locale) : '');

  return (
    <div className="rounded-xl border border-border/50 p-4 space-y-3 bg-muted/30">
      {displayRate && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Rate</span>
          <span className="font-medium truncate max-w-[60%]">{displayRate}</span>
        </div>
      )}
      {displayFee && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Network Fee</span>
          <span className="font-medium truncate max-w-[60%]">{displayFee}</span>
        </div>
      )}
      {displayPriceImpact && (
        <div className="flex justify-between items-center text-sm">
          <span className="text-muted-foreground">Price Impact</span>
          <span className="font-medium text-emerald-500 min-w-0 truncate max-w-[60%]">{displayPriceImpact}</span>
        </div>
      )}
    </div>
  );
}
