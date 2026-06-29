'use client';

import { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';
import { stellarRouteClient } from '@/lib/api/client';
import type { PriceQuote } from '@/types';

interface PriceImpactHeatmapSliderProps {
  fromToken: string;
  toToken: string;
  balance: number;
  currentAmount: string;
  onChangeAmount: (value: string) => void;
  disabled?: boolean;
  decimals?: number;
}

export function PriceImpactHeatmapSlider({
  fromToken,
  toToken,
  balance,
  currentAmount,
  onChangeAmount,
  disabled = false,
  decimals = 7,
}: PriceImpactHeatmapSliderProps) {
  const [quotes, setQuotes] = useState<PriceQuote[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fromSymbol = useMemo(() => {
    if (!fromToken) return '';
    return fromToken === 'native' ? 'XLM' : fromToken.split(':')[0];
  }, [fromToken]);

  const toSymbol = useMemo(() => {
    if (!toToken) return '';
    return toToken === 'native' ? 'XLM' : toToken.split(':')[0];
  }, [toToken]);

  // Fetch batch quotes for 10%, 20%, ..., 100% of balance
  useEffect(() => {
    if (!fromToken || !toToken || !balance || balance <= 0) {
      setQuotes([]);
      setError(null);
      return;
    }

    let active = true;
    const fetchBatchQuotes = async () => {
      setLoading(true);
      setError(null);
      try {
        const steps = Array.from({ length: 10 }, (_, i) => (i + 1) / 10);
        const requests = steps.map((percentage) => {
          const amount = balance * percentage;
          const roundedAmount = Math.floor(amount * 10 ** decimals) / 10 ** decimals;
          return {
            base: fromToken,
            quote: toToken,
            amount: roundedAmount,
            quote_type: 'sell' as const,
          };
        });

        const response = await stellarRouteClient.getQuotesBatch(requests);
        if (active) {
          setQuotes(response.quotes || []);
        }
      } catch (err) {
        if (active) {
          console.error('Failed to fetch batch quotes for heatmap:', err);
          setError(err instanceof Error ? err : new Error(String(err)));
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    fetchBatchQuotes();

    return () => {
      active = false;
    };
  }, [fromToken, toToken, balance, decimals]);

  // Calculate current percentage based on balance and currentAmount
  const currentPercentage = useMemo(() => {
    const amt = parseFloat(currentAmount);
    if (isNaN(amt) || !balance || balance <= 0) return 0;
    return Math.min(100, Math.max(0, (amt / balance) * 100));
  }, [currentAmount, balance]);

  const handleSliderChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      if (disabled || !balance || balance <= 0) return;
      const percentage = parseFloat(e.target.value);
      if (isNaN(percentage)) return;

      const amt = (percentage / 100) * balance;
      const rounded = Math.floor(amt * 10 ** decimals) / 10 ** decimals;
      onChangeAmount(percentage === 100 ? balance.toString() : rounded.toString());
    },
    [balance, onChangeAmount, disabled, decimals]
  );

  const handleSegmentClick = useCallback(
    (stepIndex: number) => {
      if (disabled || !balance || balance <= 0) return;
      const percentage = (stepIndex + 1) * 10;
      const amt = (percentage / 100) * balance;
      const rounded = Math.floor(amt * 10 ** decimals) / 10 ** decimals;
      onChangeAmount(percentage === 100 ? balance.toString() : rounded.toString());
    },
    [balance, onChangeAmount, disabled, decimals]
  );

  // Helper to color segments based on price impact
  const getImpactDetails = useCallback((impact: number) => {
    if (impact < 1) {
      return {
        bg: 'bg-emerald-500 dark:bg-emerald-400',
        text: 'text-emerald-500 dark:text-emerald-400',
        label: 'Safe',
      };
    }
    if (impact < 3) {
      return {
        bg: 'bg-yellow-500 dark:bg-yellow-400',
        text: 'text-yellow-500 dark:text-yellow-400',
        label: 'Moderate',
      };
    }
    if (impact < 5) {
      return {
        bg: 'bg-orange-500 dark:bg-orange-400',
        text: 'text-orange-500 dark:text-orange-400',
        label: 'High',
      };
    }
    return {
      bg: 'bg-destructive',
      text: 'text-destructive',
      label: 'Very High',
    };
  }, []);

  const segments = useMemo(() => {
    return Array.from({ length: 10 }, (_, i) => {
      const percentage = (i + 1) * 10;
      const amount = (percentage / 100) * balance;
      const roundedAmount = (Math.floor(amount * 10 ** decimals) / 10 ** decimals).toFixed(decimals).replace(/\.?0+$/, '');
      const quote = quotes[i];

      const impact = quote ? parseFloat(quote.price_impact || '0') : 0;
      const rate = quote ? parseFloat(quote.price || '0') : 0;
      const colorData = getImpactDetails(impact);

      return {
        index: i,
        percentage,
        amount: roundedAmount,
        impact,
        rate,
        colorData,
        hasQuote: !!quote,
      };
    });
  }, [quotes, balance, decimals, getImpactDetails]);

  if (!balance || balance <= 0) return null;

  return (
    <div className="w-full space-y-3 mt-3 px-1 animate-in fade-in duration-300">
      {/* Slider Header */}
      <div className="flex justify-between items-center text-xs text-muted-foreground font-semibold uppercase tracking-wider">
        <span>Amount Slider</span>
        <span className="text-primary font-bold text-sm tabular-nums">
          {currentPercentage.toFixed(0)}%
        </span>
      </div>

      {/* Range Input Slider */}
      <div className="relative flex items-center group">
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={currentPercentage.toFixed(0)}
          onChange={handleSliderChange}
          disabled={disabled || loading}
          className={cn(
            'w-full h-1.5 bg-muted/50 rounded-lg appearance-none cursor-pointer accent-primary',
            'focus:outline-none focus:ring-2 focus:ring-primary/20 transition-all',
            disabled && 'opacity-50 cursor-not-allowed'
          )}
          aria-label="Amount percentage slider"
          data-testid="price-impact-heatmap-slider-input"
        />
      </div>

      {/* Heatmap Track Blocks */}
      <div className="space-y-1">
        <TooltipProvider>
          <div
            className="grid grid-cols-10 gap-1"
            role="group"
            aria-label="Price impact heatmap segments"
          >
            {segments.map((seg) => {
              const isSelected = currentPercentage >= seg.percentage - 5 && currentPercentage <= seg.percentage + 5;
              const isPassed = currentPercentage >= seg.percentage;

              return (
                <Tooltip key={seg.percentage}>
                  <TooltipTrigger asChild>
                    <button
                      type="button"
                      onClick={() => handleSegmentClick(seg.index)}
                      disabled={disabled}
                      className={cn(
                        'h-2.5 rounded transition-all duration-200 focus:outline-none focus:ring-1 focus:ring-primary',
                        loading || !seg.hasQuote
                          ? 'bg-muted/40 animate-pulse'
                          : error
                          ? 'bg-muted/40'
                          : seg.colorData.bg,
                        isPassed ? 'opacity-100 scale-y-110' : 'opacity-40 hover:opacity-75',
                        isSelected && 'ring-2 ring-primary ring-offset-1 dark:ring-offset-background scale-y-125'
                      )}
                      aria-label={`${seg.percentage}% of balance, price impact ${
                        loading || !seg.hasQuote ? 'unknown' : `${seg.impact.toFixed(2)}%`
                      }`}
                      data-testid={`heatmap-segment-${seg.percentage}`}
                    />
                  </TooltipTrigger>
                  <TooltipContent className="max-w-[200px] p-2 text-xs leading-relaxed">
                    <p className="font-bold mb-0.5">{seg.percentage}% of Balance</p>
                    <p className="text-muted-foreground">
                      Amount: <span className="font-semibold text-foreground">{seg.amount} {fromSymbol}</span>
                    </p>
                    {loading || !seg.hasQuote ? (
                      <p className="text-muted-foreground italic">Fetching quote...</p>
                    ) : error ? (
                      <p className="text-destructive font-medium">Failed to load quote</p>
                    ) : (
                      <>
                        <p className="text-muted-foreground">
                          Price Impact:{' '}
                          <span className={cn('font-bold', seg.colorData.text)}>
                            {seg.impact > 0 ? `${seg.impact.toFixed(2)}%` : '< 0.01%'}
                          </span>{' '}
                          ({seg.colorData.label})
                        </p>
                        {seg.rate > 0 && (
                          <p className="text-[10px] text-muted-foreground mt-0.5 border-t border-border/20 pt-0.5">
                            Rate: 1 {fromSymbol} = {seg.rate.toFixed(4)} {toSymbol}
                          </p>
                        )}
                      </>
                    )}
                  </TooltipContent>
                </Tooltip>
              );
            })}
          </div>
        </TooltipProvider>

        {/* Heatmap Labels */}
        <div className="flex justify-between text-[10px] text-muted-foreground/60 font-bold uppercase tracking-widest px-0.5">
          <span>0%</span>
          <span>50%</span>
          <span>100%</span>
        </div>
      </div>
    </div>
  );
}
